use anyhow::{bail, Context, Error, Ok};
use getset::Getters;
use itertools::Itertools;
use reqwest::{Client as HttpClient, ClientBuilder, IntoUrl, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::warn;

use crate::utils::Timing;

#[derive(Deserialize, Getters)]
pub struct Config {
    #[getset(get = "pub")]
    api_key: String,

    #[getset(get = "pub")]
    lang: String,
}

#[derive(Debug, Getters, Serialize)]
pub struct CityWeather {
    #[getset(get = "pub")]
    weather: String,

    #[getset(get = "pub")]
    country: String,

    #[getset(get = "pub")]
    state: Option<String>,

    #[getset(get = "pub")]
    city_name: String,

    #[getset(get = "pub")]
    temperature: f32,
}

pub struct Offline;

pub struct Connected;

pub struct Client<T = Offline> {
    config: Config,
    client: HttpClient,
    _state: std::marker::PhantomData<T>,
}

#[derive(Debug, Deserialize)]
struct WeatherResponse {
    weather: Vec<Weather>,
    main: MainWeather,
}

#[derive(Debug, Deserialize)]
struct Weather {
    description: String,
}

#[derive(Debug, Deserialize)]
struct MainWeather {
    temp: f32,
}

#[derive(Debug, Deserialize)]
struct CityLocation {
    lat: f64,
    lon: f64,
    country: String,
    state: Option<String>,
    name: String,
}

impl Client<Offline> {
    pub fn new(config: Config) -> Self {
        Client::<Offline> {
            config,
            client: ClientBuilder::new().build().unwrap(),
            _state: std::marker::PhantomData,
        }
    }

    pub async fn connect(self) -> Result<Client<Connected>, Error> {
        let client = Client::<Connected> {
            config: self.config,
            client: self.client,
            _state: std::marker::PhantomData,
        };

        client
            .get_city_locations("London")
            .await
            .context("Failed to connect to the weather service.")?;

        Ok(client)
    }
}

impl Client<Connected> {
    pub async fn get_weather(&self, city: &str) -> Result<Vec<CityWeather>, Error> {
        let locations = self.get_city_locations(city).await?;
        let mut weathers = Vec::new();

        for weather in locations
            .into_iter()
            .sorted_by(|a, b| Ord::cmp(&b.country, &a.country))
            .sorted_by(|a, b| Ord::cmp(&b.state, &a.state))
            .dedup_by(|x, y| x.country == y.country && x.state == y.state)
        {
            if let Some(val) = self
                .get_city_weather(weather.lat, weather.lon, &weather.name)
                .await
            {
                weathers.push(CityWeather {
                    weather: val.0,
                    temperature: val.1,
                    country: weather.country,
                    city_name: weather.name,
                    state: weather.state,
                });
            }
        }

        Ok(weathers)
    }

    async fn get_city_weather(&self, lat: f64, lon: f64, city: &str) -> Option<(String, f32)> {
        let response: Result<WeatherResponse, Error> = self
            .get_response(
                "https://api.openweathermap.org/data/2.5/weather",
                &[
                    ("lat", lat.to_string().as_str()),
                    ("lon", lon.to_string().as_str()),
                    ("units", "metric"),
                    ("lang", self.config.lang.as_str()),
                ],
                "city_weather",
            )
            .await;

        if let Err(e) = response {
            warn!("failed to get weather for {} city: {}", city, e);
            return None;
        }

        let response = response.unwrap();

        response
            .weather
            .first()
            .map(|v| (v.description.to_string(), response.main.temp))
    }

    async fn get_city_locations(&self, city: &str) -> Result<Vec<CityLocation>, Error> {
        let locations: Vec<CityLocation> = self
            .get_response(
                "http://api.openweathermap.org/geo/1.0/direct",
                &[("q", city), ("limit", "100")],
                "city_location",
            )
            .await?;
        Ok(locations)
    }

    async fn get_response<T: DeserializeOwned, U: Serialize + Sized>(
        &self,
        url: impl IntoUrl,
        query: &U,
        identifier: &'static str,
    ) -> Result<T, Error> {
        let _timing = Timing::new(identifier);
        let request = self
            .client
            .get(url)
            .query(query)
            .query(&[("appid", &self.config.api_key)]);

        let result = request.send().await?;

        if result.status() == StatusCode::UNAUTHORIZED {
            bail!("Invalid API key for weather service. Please check the configuration.")
        };

        Ok(result
            .json::<T>()
            .await
            .context("Failed to parse JSON response.")?)
    }
}
