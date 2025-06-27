// --- File: backend/src/web_server.rs ---

use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, get_service, post, put, delete},
    Json, Router,
};
use common::ContactDto; // Assuming Contact is not directly used for serde
use std::{net::SocketAddr, sync::{Arc, Mutex}};
use tower_http::services::ServeDir;
use tracing;

#[derive(Clone)]
pub struct AppState {
    pub contacts: Arc<Mutex<Vec<ContactDto>>>,
}

pub async fn run_server(app_state: AppState) {
    let app = create_router(app_state);
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::info!("Serving frontend and API at http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub fn create_router(app_state: AppState) -> Router {
    let static_file_service = get_service(ServeDir::new("static")).handle_error(|error| async move {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serve static file: {}", error),
        )
    });

    // Pass the AppState as Arc<AppState> to the router, as you correctly did for create_contact
    // When you call .with_state(app_state) with an AppState that derives Clone,
    // Axum internally wraps it in an Arc. So, the handlers should expect Arc<AppState>.
    Router::new()
        .nest("/api",
              Router::new()
                  .route("/contacts", get(get_contacts).post(create_contact))
                  .route("/contacts/{id}", get(get_contact).put(update_contact).delete(delete_contact))
                  .with_state(Arc::new(app_state)) // Wrap the initial app_state in Arc here
        )
        .fallback_service(static_file_service)
}

// --- API Handlers ---

#[debug_handler]
async fn create_contact(
    State(state): State<Arc<AppState>>,
    Json(mut new_contact_dto): Json<ContactDto>,
) -> (StatusCode, Json<ContactDto>) {
    tracing::info!("Creating contact");
    let mut contacts = state.contacts.lock().unwrap();

    // Generate a new ID
    let new_id = contacts.iter().filter_map(|c| c.id).max().unwrap_or(0) + 1;
    new_contact_dto.id = Some(new_id);

    contacts.push(new_contact_dto.clone());

    (StatusCode::CREATED, Json(new_contact_dto))
}


#[debug_handler]
async fn get_contact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<ContactDto>, StatusCode> {
    tracing::info!("Fetching single contact with id: {}", id);
    let contacts = state.contacts.lock().unwrap();

    if let Some(contact) = contacts.iter().find(|c| c.id == Some(id)) {
        Ok(Json(contact.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[debug_handler]
async fn get_contacts(
    State(state): State<Arc<AppState>>, // <--- CHANGE THIS LINE to expect Arc<AppState>
) -> Json<Vec<ContactDto>> {
    tracing::info!("Fetching contacts from state");
    let contacts = state.contacts.lock().unwrap();
    Json(contacts.clone())
}

// --- NEW HANDLER for updating a contact ---
#[debug_handler]
async fn update_contact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(updated_contact): Json<ContactDto>,
) -> Result<Json<ContactDto>, StatusCode> {
    tracing::info!("Updating contact with id: {}", id);
    let mut contacts = state.contacts.lock().unwrap();

    if let Some(contact) = contacts.iter_mut().find(|c| c.id == Some(id)) {
        *contact = updated_contact.clone();
        Ok(Json(updated_contact))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// --- NEW HANDLER for deleting a contact ---
#[debug_handler]
async fn delete_contact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> StatusCode {
    tracing::info!("Deleting contact with id: {}", id);
    let mut contacts = state.contacts.lock().unwrap();
    let original_len = contacts.len();
    contacts.retain(|c| c.id != Some(id));

    if contacts.len() < original_len {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}