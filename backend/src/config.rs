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
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expires_minutes: i64,
    pub refresh_token_expires_days: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitConfig {
    pub per_second: u64,
    pub burst_size: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub web: WebConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub ratelimit: RateLimitConfig,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, Box<figment::Error>> {
        dotenv().ok();

        // Check for JWT_SECRET first
        if std::env::var("APP_JWT__SECRET").is_err() {
            // Use a more specific error type or just panic for critical configs
            panic!("FATAL: APP_JWT__SECRET environment variable not set.");
        }

        let config = Figment::new()
            .merge(Toml::file("Config.toml")) // For non-sensitive defaults
            .merge(Env::prefixed("APP_").split("__")) // e.g., APP_DATABASE__URL
            .extract();

        match config {
            Ok(cfg) => {
                tracing::info!("Configuration loaded successfully, full config: {:?}", cfg);
                Ok(cfg)
            }
            Err(e) => Err(Box::new(e)), // Box the error here
        }
    }
}
