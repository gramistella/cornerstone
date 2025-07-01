use axum::{
    debug_handler,
    extract::{Extension, Path, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, get_service, post, put},
    Json, Router,
};

use common::ContactDto; // Assuming Contact is not directly used for serde
use sqlx::SqlitePool;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing;
use validator::Validate;

use crate::auth;
use crate::error::AppError;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub jwt_secret: String,
}

pub fn create_router(app_state: AppState) -> Router {
    let static_file_service =
        get_service(ServeDir::new("backend/static")).handle_error(|error| async move {
            tracing::error!("Failed to serve static file: {}", error);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serve static file: {}", error),
            )
        });

    // New auth routes
    let auth_routes = Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login));

    // Existing contact routes (will be protected later)
    let contact_routes = Router::new()
        .route("/contacts", get(get_contacts).post(create_contact))
        .route(
            "/contacts/{id}",
            get(get_contact).put(update_contact).delete(delete_contact),
        )
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            crate::auth::auth_middleware,
        ));

    let cors = CorsLayer::new()
        .allow_origin(Any) // Or be more restrictive: .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods(Any) // Or specify: [Method::GET, Method::POST, ...]
        .allow_headers(Any); // Or specify: [header::CONTENT_TYPE, header::AUTHORIZATION]

    Router::new()
        .nest(
            "/api/v1",
            // Combine auth and contact routes
            auth_routes.merge(contact_routes),
        )
        .with_state(app_state) // Provide state to all nested routes
        .fallback_service(static_file_service)
        .layer(cors)
}

// --- API Handlers ---

#[debug_handler]
async fn create_contact(
    State(state): State<AppState>,
    Extension(user_id_str): Extension<String>,
    Json(new_contact_dto): Json<ContactDto>,
) -> Result<(StatusCode, Json<ContactDto>), AppError> {
    tracing::info!("Creating contact: {:?}, assigned to user {}", new_contact_dto, user_id_str);

    // Validate the new contact DTO
    new_contact_dto.validate()?;

    let user_id: i64 = user_id_str.parse().map_err(|_| AppError::InternalServerError("Invalid user ID".to_string()))?;

    let result = sqlx::query_as!(
        ContactDto,
        r#"
        INSERT INTO contacts (user_id, name, email, age, subscribed, contact_type)
        VALUES (?, ?, ?, ?, ?, ?)
        RETURNING id, name, email, age, subscribed, contact_type;
        "#,
        user_id, // Add the user_id here
        new_contact_dto.name,
        new_contact_dto.email,
        new_contact_dto.age,
        new_contact_dto.subscribed,
        new_contact_dto.contact_type
    )
    .fetch_one(&state.db_pool)
    .await;

    match result {
        Ok(created_contact) => Ok((StatusCode::CREATED, Json(created_contact))),
        Err(e) => {
            tracing::error!("Failed to create contact: {}", e);
            Err(AppError::InternalServerError(
                "Failed to create contact".to_string(),
            ))
        }
    }
}

#[debug_handler]
async fn get_contact(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(user_id_str): Extension<String>,
) -> Result<Json<ContactDto>, AppError> {
    tracing::info!("Fetching single contact with id: {} for user {}", id, user_id_str);

    let user_id: i64 = user_id_str.parse().map_err(|_| AppError::InternalServerError("Invalid user ID".to_string()))?;

    let result = sqlx::query_as!(
        ContactDto,
        "SELECT id, name, email, age, subscribed, contact_type FROM contacts WHERE id = ? AND user_id = ?",
        id,
        user_id
    )
    .fetch_optional(&state.db_pool)
    .await;

    match result {
        Ok(Some(contact)) => Ok(Json(contact)),
        Ok(None) => Err(AppError::NotFound),
        Err(e) => {
            tracing::error!("Failed to fetch contact: {}", e);
            Err(AppError::InternalServerError(
                "Failed to fetch contact".to_string(),
            ))
        }
    }
}

#[debug_handler]
async fn get_contacts(
    State(state): State<AppState>,
    Extension(user_id_str): Extension<String>,
) -> Result<Json<Vec<ContactDto>>, AppError> {

    // Now you have the user's ID and can use it in your logic.
    // The type `String` must match exactly what you inserted in the middleware.
    tracing::info!(
        "Fetching all contacts from database for user_id: {}",
        user_id_str
    );

    let user_id: i64 = user_id_str.parse().map_err(|_| AppError::InternalServerError("Invalid user ID".to_string()))?;
    
    let result = sqlx::query_as!(
        ContactDto,
        "SELECT id, name, email, age, subscribed, contact_type FROM contacts WHERE user_id = ?",
        user_id
    )
    .fetch_all(&state.db_pool)
    .await;

    match result {
        Ok(contacts) => Ok(Json(contacts)),
        Err(e) => {
            tracing::error!("Failed to fetch contacts: {}", e);
            Err(AppError::InternalServerError(
                "Failed to fetch contacts".to_string(),
            ))
        }
    }
}

#[debug_handler]
async fn update_contact(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(user_id_str): Extension<String>,
    Json(updated_contact): Json<ContactDto>,
) -> Result<Json<ContactDto>, AppError> {
    tracing::info!("Updating contact with id: {} for user {}", id, user_id_str);

    updated_contact.validate()?;
    
    let user_id: i64 = user_id_str.parse().map_err(|_| AppError::InternalServerError("Invalid user ID".to_string()))?;

    let result = sqlx::query(
        r#"
        UPDATE contacts
        SET name = ?, email = ?, age = ?, subscribed = ?, contact_type = ?
        WHERE id = ? AND user_id = ?
        "#,
    )
    .bind(&updated_contact.name)
    .bind(&updated_contact.email)
    .bind(updated_contact.age)
    .bind(updated_contact.subscribed)
    .bind(&updated_contact.contact_type)
    .bind(id)
    .bind(user_id)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(execution_result) => {
            if execution_result.rows_affected() > 0 {
                // Return the updated data
                Ok(Json(updated_contact))
            } else {
                Err(AppError::NotFound)
            }
        }
        Err(e) => {
            tracing::error!("Failed to update contact: {}", e);
            Err(AppError::InternalServerError(
                "Failed to update contact".to_string(),
            ))
        }
    }
}

#[debug_handler]
async fn delete_contact(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(user_id_str): Extension<String>,
) -> Result<StatusCode, AppError> {
    tracing::info!("Deleting contact with id: {} for user {}", id, user_id_str);
    
    let user_id: i64 = user_id_str.parse().map_err(|_| AppError::InternalServerError("Invalid user ID".to_string()))?;

    let result = sqlx::query("DELETE FROM contacts WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(&state.db_pool)
        .await;

    match result {
        Ok(execution_result) => {
            if execution_result.rows_affected() > 0 {
                Ok(StatusCode::NO_CONTENT)
            } else {
                // Use NotFound to prevent leaking information about which contacts exist
                Err(AppError::NotFound)
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete contact: {}", e);
            Err(AppError::InternalServerError(
                "Failed to delete contact".to_string(),
            ))
        }
    }
}
