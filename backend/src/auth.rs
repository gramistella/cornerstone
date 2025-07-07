use axum::{extract::State, http::StatusCode, Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use common::Credentials;
use common::LoginResponse;
use serde::{Deserialize, Serialize};

use base64::engine::{general_purpose, Engine as _};
use chrono::{Duration, Utc}; // Use chrono for time
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore; // Import RngCore for random token generation

use axum::{extract::Request, middleware::Next, response::Response};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};

use crate::error::AppError;
use crate::extractors::AuthUser;
use crate::web_server::AppState;
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

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

// --- NEW: Struct for the refresh token payload ---
#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshPayload {
    pub refresh_token: String,
}

// --- Helper struct for reading the token from the database ---
#[derive(sqlx::FromRow)]
struct RefreshTokenRecord {
    user_id: i64,
    expires_at: chrono::NaiveDateTime,
}

// --- API Handlers ---

/// ## Register a new user
/// Takes email and password, hashes the password, and stores the user in the database.
#[utoipa::path(
    post,
    path = "/api/v1/register",
    request_body = Credentials,
    responses(
        (status = 201, description = "User created successfully"),
        (status = 409, description = "User with this email already exists"),
        (status = 422, description = "Invalid data provided"),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<Credentials>,
) -> Result<StatusCode, AppError> {
    tracing::info!("Registering user with email: {}", payload.email);
    // Check if user already exists
    let existing_user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = ?")
        .bind(&payload.email)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|_| (AppError::InternalServerError("Database error".to_string())))?;

    if existing_user.is_some() {
        return Err(AppError::Conflict(
            "User with this email already exists".to_string(),
        ));
    }

    // Hash the password
    let password_hash = hash(&payload.password, DEFAULT_COST).map_err(|e| {
        tracing::error!("Failed to hash password: {}", e);
        AppError::InternalServerError("Password hashing error".to_string())
    })?;

    // Insert new user into the database
    sqlx::query("INSERT INTO users (email, password_hash) VALUES (?, ?)")
        .bind(&payload.email)
        .bind(&password_hash)
        .execute(&state.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create user: {}", e);
            AppError::InternalServerError("Failed to create user".to_string())
        })?;

    Ok(StatusCode::CREATED)
}

/// ## Login an existing user
/// Takes email and password, verifies them, and returns a JWT if successful.
#[utoipa::path(
    post,
    path = "/api/v1/login",
    request_body = Credentials,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<Credentials>,
) -> Result<Json<LoginResponse>, AppError> {
    tracing::info!("Logging in user with email: {}", payload.email);
    let user: User = sqlx::query_as("SELECT * FROM users WHERE email = ?")
        .bind(&payload.email)
        .fetch_optional(&state.db_pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !verify(&payload.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    // --- Create short-lived access token (15 minutes) ---
    let access_token_exp = (Utc::now() + Duration::minutes(15)).timestamp() as usize;
    let access_claims = Claims {
        sub: user.id.to_string(),
        exp: access_token_exp,
    };
    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(state.app_config.jwt_secret.as_ref()),
    )?;

    // --- Create long-lived refresh token (7 days) ---
    let mut refresh_token_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut refresh_token_bytes);
    let refresh_token = general_purpose::URL_SAFE_NO_PAD.encode(refresh_token_bytes);

    // --- Hash the refresh token using SHA-256 for DB lookup ---
    let mut hasher = Sha256::new();
    hasher.update(refresh_token.as_bytes());
    let refresh_token_hash = hex::encode(hasher.finalize());

    let refresh_token_exp = (Utc::now() + Duration::days(7)).naive_utc();

    // --- Store hashed refresh token in the database ---
    // Use ON CONFLICT to update the token if the user is already logged in,
    // effectively invalidating the old refresh token.
    sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES (?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET token_hash=excluded.token_hash, expires_at=excluded.expires_at",
    )
    .bind(user.id)
    .bind(&refresh_token_hash)
    .bind(refresh_token_exp)
    .execute(&state.db_pool)
    .await?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token, // Return the unhashed refresh token to the client
    }))
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
    mut request: Request, // Note: changed to mutable
    next: Next,
) -> Result<Response, AppError> {
    let token_data = decode::<Claims>(
        auth_header.token(),
        &DecodingKey::from_secret(state.app_config.jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized)?;

    let user_id: i64 = token_data
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::InternalServerError("Invalid user ID in token".to_string()))?;

    // Fetch the user from the database ONCE in the middleware
    let user = sqlx::query_as!(
        crate::auth::User,
        "SELECT id, email, password_hash FROM users WHERE id = ?",
        user_id
    )
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or(AppError::Unauthorized)?; // User not found, token is for a deleted user

    // Add the authenticated user data to the request extensions
    request.extensions_mut().insert(AuthUser {
        id: user.id,
        email: user.email,
    });

    Ok(next.run(request).await)
}

// --- Refresh Token Handler ---
#[utoipa::path(
    post,
    path = "/api/v1/refresh",
    request_body = RefreshPayload,
    responses(
        (status = 200, description = "Token refreshed successfully", body = LoginResponse),
        (status = 401, description = "Invalid or expired refresh token")
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshPayload>,
) -> Result<Json<LoginResponse>, AppError> {
    // 1. Hash the incoming refresh token to find it in the database.
    let mut hasher = Sha256::new();
    hasher.update(payload.refresh_token.as_bytes());
    let incoming_token_hash = hex::encode(hasher.finalize());

    // 2. Find the token in the database by its hash.
    // NOTE: For performance, you should add a database index to the `token_hash` column.
    let record: RefreshTokenRecord =
        sqlx::query_as("SELECT user_id, expires_at FROM refresh_tokens WHERE token_hash = ?")
            .bind(&incoming_token_hash)
            .fetch_optional(&state.db_pool)
            .await?
            .ok_or(AppError::Unauthorized)?; // If no such token, it's invalid.

    // 3. Check if the database token has expired.
    if record.expires_at < Utc::now().naive_utc() {
        // As a cleanup, remove the expired token from the DB
        sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = ?")
            .bind(&incoming_token_hash)
            .execute(&state.db_pool)
            .await
            .ok(); // We don't care about the result of the cleanup
        return Err(AppError::Unauthorized);
    }

    // --- All checks passed, we have a valid user. Now, rotate the tokens. ---
    let user_id = record.user_id;

    // 4. Issue a new short-lived access token.
    let access_token_exp = (Utc::now() + Duration::minutes(15)).timestamp() as usize;
    let access_claims = Claims {
        sub: user_id.to_string(),
        exp: access_token_exp,
    };
    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(state.app_config.jwt_secret.as_ref()),
    )?;

    // 5. Issue a brand new long-lived refresh token (Token Rotation).
    let mut new_refresh_token_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut new_refresh_token_bytes);
    let new_refresh_token = general_purpose::URL_SAFE_NO_PAD.encode(new_refresh_token_bytes);

    // Hash the new token for database storage
    let mut new_hasher = Sha256::new();
    new_hasher.update(new_refresh_token.as_bytes());
    let new_refresh_token_hash = hex::encode(new_hasher.finalize());
    let new_refresh_token_exp = (Utc::now() + Duration::days(7)).naive_utc();

    // 6. Update the database with the new refresh token hash and expiry,
    // replacing the one that was just used. This invalidates the old token.
    sqlx::query("UPDATE refresh_tokens SET token_hash = ?, expires_at = ? WHERE user_id = ?")
        .bind(&new_refresh_token_hash)
        .bind(new_refresh_token_exp)
        .bind(user_id)
        .execute(&state.db_pool)
        .await?;

    // 7. Return the new pair of tokens to the client.
    Ok(Json(LoginResponse {
        access_token,
        refresh_token: new_refresh_token, // This is the new, un-hashed refresh token.
    }))
}

// --- Logout Handler ---
#[utoipa::path(
    post,
    path = "/api/v1/logout",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 204, description = "Logout successful"),
        (status = 401, description = "Authentication required")
    )
)]
pub async fn logout(State(state): State<AppState>, user: AuthUser) -> Result<StatusCode, AppError> {
    // Simply delete the refresh token from the database
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
        .bind(user.id)
        .execute(&state.db_pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
