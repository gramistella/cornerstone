// --- File: backend/src/main.rs ---

// Use the library part of the `backend` crate instead of a local module.
use backend::web_server::AppState;
use common::{utils::is_valid_email, ContactDto};
use dotenvy::dotenv;
use sqlx::sqlite::SqlitePoolOptions;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
use config::AppConfig;

use tokio::signal;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("signal received, starting graceful shutdown");
}

#[tokio::main]
async fn main() {
    // --- Setup ---
    // 1. Initialize structured logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::filter::LevelFilter::INFO) // This sets the minimum level to INFO
        .init();

    let config = AppConfig::from_env().expect("Failed to load configuration");

    let db_pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database.url)
        .await
        .unwrap();

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&db_pool).await.ok();
    tracing::info!("Migrations complete.");

    let app_state = AppState {
        db_pool,
        jwt_secret: config.jwt_secret,
    };

    // 2. Initialize application state (e.g., in-memory DB)
    let initial_contacts: Vec<ContactDto> = vec![
        ContactDto {
            id: Some(1),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 35,
            subscribed: true,
            contact_type: "Customer".to_string(),
        },
        ContactDto {
            id: Some(2),
            name: "Jane Smith".to_string(),
            email: "jane@example.com".to_string(),
            age: 28,
            subscribed: false,
            contact_type: "Lead".to_string(),
        },
    ];

    // --- Run Server ---
    // 3. Start the web server and pass it the state
    tracing::info!("Initializing server...");
    let app = backend::web_server::create_router(app_state.clone());
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::info!("Serving frontend and API at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    // This code runs after the server has stopped accepting new connections
    tracing::info!("Server shut down gracefully. Closing database connections.");
    app_state.db_pool.close().await;
    tracing::info!("Database pool closed.");
}
