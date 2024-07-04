use std::{error::Error, fs, io::Write, path::Path, process};

use clap::ValueEnum;
use getset::Getters;
use prettytable::{
    format::{self},
    row, Table,
};
use serde::Deserialize;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::weather_client::{self, CityWeather, Client, Config, Connected};

#[derive(Debug, Deserialize, Clone, Copy, ValueEnum)]
pub enum OutputType {
    Table,
    Simple,
    Json,
}

#[derive(Deserialize, Getters)]
pub struct AppConfig {
    #[getset(get = "pub")]
    pub client: Config,

    #[getset(get = "pub")]
    output: OutputType,

    #[getset(get = "pub")]
    level: Option<String>,
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }
}

pub async fn print_city_weather_interactive(
    client: &weather_client::Client<Connected>,
    output_type: &OutputType,
) {
    let mut city = String::new();

    loop {
        city.clear();

        print!("Enter the city name: ");
        std::io::stdout().flush().expect("failed to flush stdout");
        std::io::stdin()
            .read_line(&mut city)
            .expect("failed to read line");

        if city.trim().is_empty() {
            continue;
        }

        if let Err(error) = print_city_weather(client, &city, output_type).await {
            eprintln!("{}", error);
            process::exit(1);
        }
    }
}

pub async fn print_city_weather(
    app: &Client<Connected>,
    city: &str,
    output_type: &OutputType,
) -> Result<(), Box<dyn Error>> {
    let weathers = app.get_weather(city.trim()).await?;

    if !weathers.is_empty() {
        match output_type {
            OutputType::Table => print_weathers_table(weathers),
            OutputType::Simple => print_weathers_simple(weathers),
            OutputType::Json => print_weathers_json(weathers),
        };
    }

    Ok(())
}

fn print_weathers_simple(weathers: Vec<CityWeather>) {
    for weather in weathers {
        println!(
            "{} ({}, {}): {}, {}°",
            weather.city_name(),
            weather.country(),
            weather.state().as_deref().unwrap_or(""),
            weather.weather(),
            *weather.temperature() as i16
        );
    }
    println!();
}

fn print_weathers_table(weathers: Vec<CityWeather>) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row!["City", "Country", "State", "Weather", "Degrees"]);

    for weather in weathers {
        table.add_row(row![
            weather.city_name(),
            weather.country(),
            weather.state().as_deref().unwrap_or(""),
            weather.weather(),
            format!("{}°", *weather.temperature() as i16)
        ]);
    }

    table.printstd();
    println!();
}

fn print_weathers_json(weathers: Vec<CityWeather>) {
    println!(
        "{}",
        serde_json::to_string_pretty(&weathers).unwrap_or_default()
    );
    println!();
}

pub fn init_tracing(level: Level) {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
