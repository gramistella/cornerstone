use backend::config::{JwtConfig, RateLimitConfig, WebConfig};
use backend::db::DbPool;
use backend::db::DbPoolOptions;
use backend::{config::AppConfig, web_server::AppState};
use common::{Credentials, LoginResponse};
use reqwest::StatusCode;
use sqlx::Executor;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub const TEST_JWT_SECRET: &str = "test_secret";

/// Spawn a test server and return the address and a reqwest client.
pub async fn spawn_app() -> (SocketAddr, reqwest::Client, DbPool) {
    // The listener is bound to a random available port.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let addr = listener.local_addr().unwrap();

    // Load .env file for database URLs, especially for local Postgres tests.
    dotenvy::dotenv().ok();

    // --- Database and Config Setup based on feature flags ---
    let (db_pool, config) = if cfg!(feature = "db-postgres") {
        // --- PostgreSQL Setup ---
        println!("🧪 Setting up test environment for PostgreSQL...");

        // 1. Get the database URL from environment, panic if not set.
        let db_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for Postgres tests.");

        // 2. Create a master pool to create a unique test database.
        let master_pool = DbPoolOptions::new()
            .connect(&db_url)
            .await
            .expect("Failed to connect to Postgres master database.");

        // 3. Generate a unique database name for this test run.
        let db_name = uuid::Uuid::new_v4().to_string();
        master_pool
            .execute(format!(r#"CREATE DATABASE "{db_name}";"#).as_str())
            .await
            .unwrap_or_else(|_| panic!("Failed to create test database: {db_name}"));

        // 4. Create the connection URL for the new test database.
        let test_db_url = format!("{}/{}", db_url.rsplit_once('/').unwrap().0, db_name);

        // 5. Create the actual connection pool for the test database.
        let db_pool = DbPoolOptions::new()
            .connect(&test_db_url)
            .await
            .expect("Failed to connect to the test database.");

        // 6. Run migrations on the test database.
        sqlx::migrate!("./migrations/postgres")
            .run(&db_pool)
            .await
            .unwrap();

        // 7. Create the AppConfig for Postgres.
        let config = AppConfig {
            web: WebConfig {
                addr: "127.0.0.1".to_string(),
                port: addr.port(),
                cors_origin: "http://localhost:5173".to_string(),
            },
            jwt: JwtConfig {
                secret: TEST_JWT_SECRET.to_string(),
                access_token_expires_minutes: 1,
                refresh_token_expires_days: 1,
            },
            ratelimit: RateLimitConfig {
                per_second: 1000,
                burst_size: 500,
            },
        };
        (db_pool, config)
    } else if cfg!(feature = "db-sqlite") {
        // --- SQLite Setup ---
        println!("🧪 Setting up test environment for SQLite (in-memory)...");

        // 1. Create an in-memory SQLite pool.
        let db_pool = DbPoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // 2. Run migrations.
        sqlx::migrate!("./migrations/sqlite")
            .run(&db_pool)
            .await
            .unwrap();

        // 3. Create the AppConfig for SQLite.
        let config = AppConfig {
            web: WebConfig {
                addr: "127.0.0.1".to_string(),
                port: addr.port(),
                cors_origin: "http://localhost:5173".to_string(),
            },
            jwt: JwtConfig {
                secret: TEST_JWT_SECRET.to_string(),
                access_token_expires_minutes: 15,
                refresh_token_expires_days: 7,
            },
            ratelimit: RateLimitConfig {
                per_second: 1000,
                burst_size: 500,
            },
        };
        (db_pool, config)
    } else {
        panic!("A database feature ('db-postgres' or 'db-sqlite') must be enabled for tests.");
    };

    // --- Common App Setup ---
    let app_state = AppState {
        db_pool: db_pool.clone(),
        app_config: config,
    };

    let app = backend::web_server::create_router(app_state);

    tokio::spawn(async move {
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
    let register_url = format!("http://{addr}/api/v1/register");
    let login_url = format!("http://{addr}/api/v1/login");

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

    assert_eq!(
        status,
        StatusCode::OK,
        "Login request did not return 200 OK"
    );

    // Now, try to parse the text we received
    let login_response: LoginResponse =
        serde_json::from_str(&body_text).expect("Failed to parse login response from text");

    login_response.access_token
}
