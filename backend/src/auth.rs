// --- File: backend/src/auth.rs ---

use axum::{extract::State, http::StatusCode, Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use common::Credentials;
use common::LoginResponse;

use axum::{
    extract::{Request },
    middleware::Next,
    response::Response,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::{decode, DecodingKey, Validation};

use crate::web_server::AppState;

// --- User & Payload Structs ---

#[derive(sqlx::FromRow, Debug)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user id)
    pub exp: usize,  // Expiration time
}

// --- API Handlers ---

/// ## Register a new user
/// Takes email and password, hashes the password, and stores the user in the database.
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<Credentials>,
) -> Result<StatusCode, (StatusCode, String)> {
    tracing::info!("Registering user with email: {}", payload.email);
    // Check if user already exists
    let existing_user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = ?")
        .bind(&payload.email)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    if existing_user.is_some() {
        return Err((StatusCode::CONFLICT, "User with this email already exists".to_string()));
    }

    // Hash the password
    let password_hash = hash(&payload.password, DEFAULT_COST)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to hash password".to_string()))?;

    // Insert new user into the database
    sqlx::query("INSERT INTO users (email, password_hash) VALUES (?, ?)")
        .bind(&payload.email)
        .bind(&password_hash)
        .execute(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create user: {}", e)))?;

    Ok(StatusCode::CREATED)
}

/// ## Login an existing user
/// Takes email and password, verifies them, and returns a JWT if successful.
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<Credentials>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    tracing::info!("Logging in user with email: {}", payload.email);
    // Find the user by email
    let user: User = sqlx::query_as("SELECT * FROM users WHERE email = ?")
        .bind(&payload.email)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

    // Verify the password
    let valid_password = verify(&payload.password, &user.password_hash)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Password verification error".to_string()))?;

    if !valid_password {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()));
    }

    // Create JWT claims
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    let expiration = duration_since_epoch.as_secs() + 3600; // Expires in 1 hour

    let claims = Claims {
        sub: user.id.to_string(),
        exp: expiration as usize,
    };

    // Encode the token
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_ref()),
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create token".to_string()))?;

    Ok(Json(LoginResponse { token }))
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    // Use TypedHeader to extract the token
    TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
    // You can also add the request as a parameter if you need to access it
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    // Decode and validate the token
    let _claims = decode::<Claims>(
        auth_header.token(),
        &DecodingKey::from_secret(state.jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|e| {
        // Log the specific error for debugging
        tracing::warn!("Token validation failed: {}", e);
        (StatusCode::UNAUTHORIZED, "Invalid or expired token".to_string())
    })?;

    // If the token is valid, we can proceed to the next middleware or the handler
    Ok(next.run(request).await)
}