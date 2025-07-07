// backend/tests/helpers.rs
use backend::{config::AppConfig, web_server::AppState};
use common::{Credentials, LoginResponse};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;
use reqwest::StatusCode;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;

/// Spawn a test server and return the address and a reqwest client.
pub async fn spawn_app() -> (SocketAddr, reqwest::Client, SqlitePool) {
    // 2. The listener is now created asynchronously.
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    // Create connection options that enforce foreign keys
    let connect_options = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(true); // This is the crucial line

    // Create the pool from the connection options
    let db_pool = SqlitePoolOptions::new()
         .max_connections(1)
        .connect_with(connect_options)
        .await
        .expect("Failed to create in-memory database pool.");
    
    
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations on test database.");

    let config = AppConfig {
        web: backend::config::WebConfig {
            addr: "127.0.0.1".to_string(),
            port: addr.port(),
            cors_origin: "http://localhost:5173".to_string(),
        },
        database: backend::config::DatabaseConfig {
            url: "sqlite::memory:".to_string(),
        },
        jwt_secret: "test-secret".to_string(),
    };

    let app_state = AppState {
        db_pool: db_pool.clone(),
        app_config: config,
    };

    let app = backend::web_server::create_router(app_state);

    tokio::spawn(async move {
        // 3. Pass the listener and a service with connection info to axum::serve.
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    (addr, client, db_pool)
}


/// Helper to register and login a test user, returning their auth token.
pub async fn get_auth_token(addr: &SocketAddr, client: &reqwest::Client) -> String {
    let register_url = format!("http://{}/api/v1/register", addr);
    let login_url = format!("http://{}/api/v1/login", addr);
    //println!("Register URL: {}", register_url);
    let credentials = Credentials {
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
    };

    // Register
    let res = client
        .post(&register_url)
        .json(&credentials)
        .send()
        .await
        .expect("Failed to register user");
    //println!("Response: {:?}", res);
    assert_eq!(res.status(), StatusCode::CREATED, "Registration failed");

    // Login
    let response = client
        .post(&login_url)
        .json(&credentials)
        .send()
        .await
        .expect("Failed to login user");

    let status = response.status();
    // Read the body as text INSTEAD of trying to parse as JSON immediately
    let body_text = response.text().await.expect("Failed to read response body");

    // --- Add this block for debugging ---
    println!("\n--- LOGIN RESPONSE DEBUG ---");
    println!("Status Code: {}", status);
    println!("Response Body: '{}'", body_text);
    println!("--- END DEBUG ---\n");
    // ------------------------------------

    assert_eq!(status, StatusCode::OK, "Login request did not return 200 OK");

    // Now, try to parse the text we received
    let login_response: LoginResponse =
        serde_json::from_str(&body_text).expect("Failed to parse login response from text");

    login_response.access_token
}