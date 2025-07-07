// Use the library part of the `backend` crate instead of a local module.
use backend::web_server::AppState;
use sqlx::sqlite::SqlitePoolOptions;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use backend::config::AppConfig;

use tokio::signal;

use std::net::IpAddr;

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
        app_config: config.clone(),
    };

    // --- Run Server ---
    // 3. Start the web server and pass it the state
    tracing::info!("Initializing server...");
    let app = backend::web_server::create_router(app_state.clone());

    let ip_addr: IpAddr = config
        .web
        .addr
        .parse()
        .expect("Invalid IP address in config");

    let addr = SocketAddr::new(ip_addr, config.web.port);
    tracing::info!("Serving frontend and API at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();

    // This code runs after the server has stopped accepting new connections
    tracing::info!("Server shut down gracefully. Closing database connections.");
    app_state.db_pool.close().await;
    tracing::info!("Database pool closed.");
}
