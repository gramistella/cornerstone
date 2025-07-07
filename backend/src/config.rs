use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;

use dotenvy::dotenv;

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebConfig {
    pub addr: String,
    pub port: u16,
    pub cors_origin: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub web: WebConfig,
    pub database: DatabaseConfig,
    pub jwt_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, figment::Error> {
        dotenv().ok();

        // Check for JWT_SECRET first
        if std::env::var("APP_JWT_SECRET").is_err() {
            // Use a more specific error type or just panic for critical configs
            panic!("FATAL: APP_JWT_SECRET environment variable not set.");
        }

        let config: _ = Figment::new()
            .merge(Toml::file("Config.toml")) // For non-sensitive defaults
            .merge(Env::prefixed("APP_").split("__")) // e.g., APP_DATABASE__URL
            .extract();

        tracing::info!(
            "Configuration loaded successfully, full config: {:?}",
            config
        );

        return config;
    }
}
