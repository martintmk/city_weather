use clap::Parser;
use tracing::Level;
use weather::{
    app::{self, OutputType},
    weather_client,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Config {
    /// The city name to retrieve the weather information.
    #[arg(short, long)]
    pub city: Option<String>,

    /// The type of output to display the weather information.
    #[arg(short, long)]
    pub output: Option<OutputType>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli_config = Config::parse();
    let app_config = app::AppConfig::load("config.toml")?;

    app::init_tracing(match app_config.level().as_ref() {
        Some(level) => level.parse::<Level>().unwrap_or(Level::INFO),
        None => Level::INFO,
    });

    let output = cli_config
        .output
        .unwrap_or_else(|| app_config.output().to_owned());

    let client = weather_client::Client::new(app_config.client)
        .connect()
        .await?;

    if let Some(city) = &cli_config.city {
        app::print_city_weather(&client, city, &output).await?;
    } else {
        app::print_city_weather_interactive(&client, &output).await;
    }

    Ok(())
}
