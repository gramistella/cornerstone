use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;

use dotenvy::dotenv;

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub jwt_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, figment::Error> {
        dotenv().ok();

        Figment::new()
            .merge(Toml::file("Config.toml")) // For non-sensitive defaults
            .merge(Env::prefixed("APP_").split("__")) // e.g., APP_DATABASE__URL
            .extract()
    }
}
