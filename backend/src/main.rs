// --- File: backend/src/main.rs ---

// Use the library part of the `backend` crate instead of a local module.
use backend::web_server::{run_server, AppState};
use common::{utils::is_valid_email, ContactDto};
use std::sync::{Arc, Mutex};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // --- Setup ---
    // 1. Initialize structured logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

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

    // Create the AppState instance
    let app_state = AppState {
        contacts: Arc::new(Mutex::new(initial_contacts)),
    };

    // --- Run Server ---
    // 3. Start the web server and pass it the state
    tracing::info!("Initializing server...");
    run_server(app_state).await;
}
