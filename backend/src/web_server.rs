use std::{sync::Arc, time::Duration};

use axum::{
    debug_handler,
    extract::{Path, State},
    http::{header, HeaderValue, Method, StatusCode},
    middleware,
    routing::{get, get_service, post},
    Json, Router,
};

use crate::db::DbPool;
use serde::Deserialize;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use tracing;
use validator::Validate;

use crate::error::AppError;
use crate::extractors::AuthUser;
use crate::{auth, config::AppConfig};
use common::ContactDto;

use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use common::Credentials;
use common::LoginResponse;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::register,
        auth::login,
        auth::refresh,
        auth::logout,
        get_contacts,
        create_contact,
        get_contact,
    ),
    // 👇 All components are now in a single block
    components(
        schemas(ContactDto, Credentials, LoginResponse),
    ),
    tags(
        (name = "Cornerstone API", description = "Full-stack Rust template API")
    ),
    // This part remains the same, it *applies* the security scheme to the UI
    security(
        ("bearer_auth" = [])
    )
)]
struct ApiDoc;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DbPool,
    pub app_config: AppConfig,
}

fn create_static_router() -> Router {
    // This will cause a compilation error if neither `svelte-ui` nor `slint-ui` feature is enabled.
    #[cfg(not(any(feature = "svelte-ui", feature = "slint-ui")))]
    compile_error!("You must enable either the 'svelte-ui' or 'slint-ui' feature.");

    // This code block will only be included if the `svelte-ui` feature is enabled
    #[cfg(feature = "svelte-ui")]
    let static_service = get_service(
        ServeDir::new("backend/static/svelte-build")
            .not_found_service(ServeFile::new("backend/static/svelte-build/index.html")),
    )
    .handle_error(|error| async move {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serve Svelte app: {error}"),
        )
    });

    // This code block will only be included if the `slint-ui` feature is enabled
    #[cfg(feature = "slint-ui")]
    let static_service = get_service(
        ServeDir::new("backend/static/slint-build")
            .not_found_service(ServeFile::new("backend/static/slint-build/index.html")),
    )
    .handle_error(|error| async move {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serve Slint app: {error}"),
        )
    });

    Router::new().fallback_service(static_service)
}

pub fn create_router(app_state: AppState) -> Router {
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(app_state.app_config.ratelimit.per_second)
            .burst_size(app_state.app_config.ratelimit.burst_size)
            .finish()
            .unwrap(),
    );

    // This avoids the storage size growing indefinitely
    let governor_limiter = governor_conf.limiter().clone();
    let interval = Duration::from_secs(60);
    // a separate background task to clean up
    std::thread::spawn(move || loop {
        std::thread::sleep(interval);
        tracing::debug!("Rate limiting storage size: {}", governor_limiter.len());
        governor_limiter.retain_recent();
    });

    // Public routes that do not require authentication
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
        .route("/refresh", post(auth::refresh))
        // Apply the rate-limiting layer to public routes
        .layer(GovernorLayer {
            config: governor_conf,
        });

    // Protected routes that require authentication
    let protected_routes = Router::new()
        .route("/logout", post(auth::logout))
        .route("/contacts", get(get_contacts).post(create_contact))
        .route(
            "/contacts/{id}",
            get(get_contact).put(update_contact).delete(delete_contact),
        )
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::auth_middleware,
        ));

    // Combine public and protected routes under the /api/v1 prefix
    let api_routes = Router::new().merge(public_routes).merge(protected_routes);

    let cors = CorsLayer::new()
        .allow_origin(
            app_state
                .app_config
                .web
                .cors_origin
                .parse::<HeaderValue>()
                .expect("Invalid CORS_ORIGIN in config.toml"),
        )
        // It's good practice to be specific about allowed methods and headers
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        // This is required to allow the browser to send credentials (e.g., cookies, auth tokens)
        .allow_credentials(true);

    let mut router = Router::new();

    if cfg!(debug_assertions) {
        tracing::info!("Debug mode: Enabling /docs endpoint");
        router =
            router.merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()));
    }

    router
        .nest("/api/v1", api_routes) // Nest all API routes under /api/v1
        .fallback_service(create_static_router())
        .with_state(app_state)
        .layer(
            TraceLayer::new_for_http()
                // Add this layer to add a request ID to all traces
                .make_span_with(tower_http::trace::DefaultMakeSpan::new().include_headers(true))
                .on_response(tower_http::trace::DefaultOnResponse::new().include_headers(true)),
        )
        .layer(SetRequestIdLayer::new(
            "x-request-id".parse().unwrap(),
            MakeRequestUuid,
        )) // This line adds the request ID
        .layer(cors)
}
// --- API Handlers ---

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "Service is healthy")
    )
)]
#[debug_handler]
async fn health_check() -> StatusCode {
    StatusCode::OK
}

#[utoipa::path(
    post,
    path = "/api/v1/contacts",
    request_body = ContactDto,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 201, description = "Contact created successfully", body = ContactDto),
        (status = 401, description = "Authentication required"),
        (status = 422, description = "Validation error"),
    )
)]
#[debug_handler]
async fn create_contact(
    State(state): State<AppState>,
    user: AuthUser,
    Json(new_contact_dto): Json<ContactDto>,
) -> Result<(StatusCode, Json<ContactDto>), AppError> {
    tracing::info!(
        "Creating contact: {:?}, assigned to user {}",
        new_contact_dto,
        user.id
    );

    // Validate the new contact DTO
    new_contact_dto.validate()?;

    let result = sqlx::query_as!(
        ContactDto,
        r#"
        INSERT INTO contacts (user_id, name, email, age, subscribed, contact_type)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, name, email, age, subscribed, contact_type;
        "#,
        user.id, // Add the user_id here
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

#[utoipa::path(
    get,
    path = "/api/v1/contacts/{id}",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "Contact ID")
    ),
    responses(
        (status = 200, body = ContactDto),
        (status = 404, description = "Contact not found"),
        (status = 401, description = "Authentication required")
    )
)]
#[debug_handler]
async fn get_contact(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    user: AuthUser,
) -> Result<Json<ContactDto>, AppError> {
    tracing::info!(
        "Fetching single contact with id: {} for user {}",
        id,
        user.id
    );

    let result = sqlx::query_as!(
        ContactDto,
        "SELECT id, name, email, age, subscribed, contact_type FROM contacts WHERE id = $1 AND user_id = $2",
        id,
        user.id
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

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[utoipa::path(
    get,
    path = "/api/v1/contacts",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "List of contacts", body = Vec<ContactDto>),
        (status = 401, description = "Authentication required")
    )
)]
#[debug_handler]
async fn get_contacts(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Query(pagination): axum::extract::Query<Pagination>, // <-- Add this
) -> Result<Json<Vec<ContactDto>>, AppError> {
    // Set default values for pagination
    let page = pagination.page.unwrap_or(1) as i64;
    let per_page = pagination.per_page.unwrap_or(20) as i64;
    let offset = (page - 1) * per_page;

    tracing::info!(
        "Fetching contacts for user {}, page: {}, per_page: {}",
        user.id,
        page,
        per_page
    );

    let result = sqlx::query_as!(
        ContactDto,
        "SELECT id, name, email, age, subscribed, contact_type
         FROM contacts
         WHERE user_id = $1
         LIMIT $2 OFFSET $3",
        user.id,
        per_page,
        offset
    )
    .fetch_all(&state.db_pool)
    .await;

    // ... rest of the handler remains the same
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
    user: AuthUser,
    Json(updated_contact): Json<ContactDto>,
) -> Result<Json<ContactDto>, AppError> {
    tracing::info!("Updating contact with id: {} for user {}", id, user.id);

    updated_contact.validate()?;

    let result = sqlx::query(
        r#"
        UPDATE contacts
        SET name = $1, email = $2, age = $3, subscribed = $4, contact_type = $5
        WHERE id = $6 AND user_id = $7
        "#,
    )
    .bind(&updated_contact.name)
    .bind(&updated_contact.email)
    .bind(updated_contact.age)
    .bind(updated_contact.subscribed)
    .bind(&updated_contact.contact_type)
    .bind(id)
    .bind(user.id)
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
    user: AuthUser,
) -> Result<StatusCode, AppError> {
    tracing::info!("Deleting contact with id: {} for user {}", id, user.id);

    let result = sqlx::query("DELETE FROM contacts WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
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
