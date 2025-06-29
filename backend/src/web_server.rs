// --- File: backend/src/web_server.rs ---

use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, get_service, post, put, delete},
    middleware,
    Json, Router,
};
use common::ContactDto; // Assuming Contact is not directly used for serde
use std::{net::SocketAddr, sync::{Arc, Mutex}};
use tower_http::services::ServeDir;
use tracing;
use sqlx::SqlitePool;

use crate::auth;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub jwt_secret: String,
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
    let static_file_service = get_service(ServeDir::new("backend/static")).handle_error(|error| async move {
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
        .route("/contacts/{id}", get(get_contact).put(update_contact).delete(delete_contact))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            crate::auth::auth_middleware,
        ));
    
    Router::new()
        .nest("/api", 
            // Combine auth and contact routes
            auth_routes.merge(contact_routes)
        )
        .with_state(app_state) // Provide state to all nested routes
        .fallback_service(static_file_service)

}

// --- API Handlers ---

#[debug_handler]
async fn create_contact(
    State(state): State<AppState>,
    Json(new_contact_dto): Json<ContactDto>,
) -> Result<(StatusCode, Json<ContactDto>), StatusCode> {
    tracing::info!("Creating contact: {:?}", new_contact_dto);
    
    let result = sqlx::query_as!(
        ContactDto,
        r#"
        INSERT INTO contacts (name, email, age, subscribed, contact_type)
        VALUES (?, ?, ?, ?, ?)
        RETURNING id, name, email, age, subscribed, contact_type;
        "#,
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
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}


#[debug_handler]
async fn get_contact(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ContactDto>, StatusCode> {
    tracing::info!("Fetching single contact with id: {}", id);
    
    let result = sqlx::query_as!(
        ContactDto,
        "SELECT id, name, email, age, subscribed, contact_type FROM contacts WHERE id = ?",
        id
    )
    .fetch_optional(&state.db_pool)
    .await;

    match result {
        Ok(Some(contact)) => Ok(Json(contact)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to fetch contact: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[debug_handler]
async fn get_contacts(
    State(state): State<AppState>,
) -> Result<Json<Vec<ContactDto>>, StatusCode> {
    tracing::info!("Fetching all contacts from database");
    
    let result = sqlx::query_as!(
        ContactDto,
        "SELECT id, name, email, age, subscribed, contact_type FROM contacts"
    )
    .fetch_all(&state.db_pool)
    .await;

    match result {
        Ok(contacts) => Ok(Json(contacts)),
        Err(e) => {
            tracing::error!("Failed to fetch contacts: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[debug_handler]
async fn update_contact(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(updated_contact): Json<ContactDto>,
) -> Result<Json<ContactDto>, StatusCode> {
    tracing::info!("Updating contact with id: {}", id);
    
    let result = sqlx::query(
        r#"
        UPDATE contacts
        SET name = ?, email = ?, age = ?, subscribed = ?, contact_type = ?
        WHERE id = ?
        "#,
    )
    .bind(&updated_contact.name)
    .bind(&updated_contact.email)
    .bind(updated_contact.age)
    .bind(updated_contact.subscribed)
    .bind(&updated_contact.contact_type)
    .bind(id)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(execution_result) => {
            if execution_result.rows_affected() > 0 {
                // Return the updated data
                Ok(Json(updated_contact))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Failed to update contact: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[debug_handler]
async fn delete_contact(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> StatusCode {
    tracing::info!("Deleting contact with id: {}", id);
    
    let result = sqlx::query("DELETE FROM contacts WHERE id = ?")
        .bind(id)
        .execute(&state.db_pool)
        .await;

    match result {
        Ok(execution_result) => {
            if execution_result.rows_affected() > 0 {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::NOT_FOUND
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete contact: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
