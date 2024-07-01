use std::{fs, io::Write, path::Path, process};

use getset::Getters;
use prettytable::{
    format::{self},
    row, Table,
};
use serde::Deserialize;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::weather_client::{CityWeather, Client, Config};

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum OutputType {
    Table,
    Simple,
    Json,
}

#[derive(Deserialize, Getters)]
struct AppConfig {
    #[getset(get = "pub")]
    client: Config,

    #[getset(get = "pub")]
    output: OutputType,

    #[getset(get = "pub")]
    level: Option<String>,
}

impl AppConfig {
    fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let content: String =
            fs::read_to_string(path).map_err(|_| "failed to read the configuration file")?;
        Ok(toml::from_str(&content)
            .map_err(|e| format!("failed to deserialize config file: {}", e))?)
    }
}

pub async fn run() {
    let config = AppConfig::load("config.toml").unwrap_or_else(|e| {
        eprintln!("Failed to read the config file: {}", e);
        std::process::exit(1);
    });

    init_tracing(match config.level().as_ref() {
        Some(level) => level.parse::<Level>().unwrap_or(Level::INFO),
        None => Level::INFO,
    });

    let print_table = config.output().to_owned();

    let app = Client::new(config.client);
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

        match app.get_weather(&city.trim()).await {
            Ok(weathers) => {
                if weathers.len() == 0 {
                    continue;
                }

                match print_table {
                    OutputType::Table => print_weathers_table(weathers),
                    OutputType::Simple => print_weathers_simple(weathers),
                    OutputType::Json => print_weathers_json(weathers),
                }
            }
            Err(error) => {
                eprintln!("Error: {}", error);
                process::exit(1);
            }
        }
    }
}

fn print_weathers_simple(weathers: Vec<CityWeather>) {
    for weather in weathers {
        println!(
            "{} ({}, {}): {}, {}°",
            weather.city_name(),
            weather.country(),
            weather.state().as_deref().unwrap_or_else(|| ""),
            weather.weather(),
            *weather.temperature() as i16
        );
    }
    println!("");
}

pub(crate) fn print_weathers_table(weathers: Vec<CityWeather>) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row!["City", "Country", "State", "Weather", "Degrees"]);

    for weather in weathers {
        table.add_row(row![
            weather.city_name(),
            weather.country(),
            weather.state().as_deref().unwrap_or_else(|| ""),
            weather.weather(),
            format!("{}°", *weather.temperature() as i16)
        ]);
    }

    table.printstd();
    println!("");
}

fn print_weathers_json(weathers: Vec<CityWeather>) {
    println!(
        "{}",
        serde_json::to_string_pretty(&weathers).unwrap_or(String::new())
    );
    println!("");
}

fn init_tracing(level: Level) {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
