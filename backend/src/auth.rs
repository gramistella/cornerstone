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

use crate::config::JwtConfig;
use crate::error::AppError;
use crate::web_server::AppState;
use crate::{db::DbPool, extractors::AuthUser};
use rand::Rng;
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use validator::Validate;

// --- User & Payload Structs ---

#[derive(sqlx::FromRow, Debug)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // Subject (user id)
    pub exp: usize,    // Expiration time
    pub nonce: String, // Nonce for access token uniqueness
}

// --- Struct for the refresh token payload ---
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

// --- Token Helper ---

/// Creates a new access token and a new refresh token for a user.
/// It stores the hashed refresh token in the database, replacing any existing one for the user.
/// Optionally, if an `old_token_hash` is provided, it will be deleted as part of the transaction,
/// ensuring old refresh tokens are invalidated upon use.
async fn issue_tokens(
    user_id: i64,
    db_pool: &DbPool,
    jwt_config: &JwtConfig,
    old_token_hash: Option<&str>,
) -> Result<LoginResponse, AppError> {
    // Generate a random nonce for the access token to ensure uniqueness
    let nonce: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();

    // --- Create short-lived access token (15 minutes) ---
    let access_token_exp = (Utc::now() + Duration::minutes(jwt_config.access_token_expires_minutes))
        .timestamp() as usize;
    let access_claims = Claims {
        sub: user_id.to_string(),
        exp: access_token_exp,
        nonce,
    };
    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(jwt_config.secret.as_ref()),
    )?;

    // --- Create a new long-lived refresh token (7 days) ---
    let mut refresh_token_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut refresh_token_bytes);
    let new_refresh_token = general_purpose::URL_SAFE_NO_PAD.encode(refresh_token_bytes);

    // Hash the new token for database storage
    let mut new_hasher = Sha256::new();
    new_hasher.update(new_refresh_token.as_bytes());
    let new_refresh_token_hash = hex::encode(new_hasher.finalize());
    let new_refresh_token_exp =
        (Utc::now() + Duration::days(jwt_config.refresh_token_expires_days)).naive_utc();

    // --- Database Operations in a Transaction ---
    let mut tx = db_pool.begin().await?;

    // If an old token was used (in a refresh operation), delete it.
    if let Some(old_hash) = old_token_hash {
        sqlx::query!("DELETE FROM refresh_tokens WHERE token_hash = $1", old_hash)
            .execute(&mut *tx)
            .await?;
    }

    // Insert the new refresh token, replacing any existing token for the user.
    // This invalidates any other sessions if the user logs in again.

    sqlx::query!(
		"INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)
         ON CONFLICT(user_id) DO UPDATE SET token_hash=excluded.token_hash, expires_at=excluded.expires_at",
		user_id,
		new_refresh_token_hash,
		new_refresh_token_exp
	)
	.execute(&mut *tx)
	.await?;

    tx.commit().await?;

    // Return the new pair of tokens to the client.
    Ok(LoginResponse {
        access_token,
        refresh_token: new_refresh_token,
    })
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
    // Validate the incoming payload
    payload.validate()?;

    tracing::info!("Registering user with email: {}", &payload.email);
    // Check if user already exists
    let existing_user: Option<User> = sqlx::query_as!(
        User,
        "SELECT id as \"id!\", email, password_hash FROM users WHERE email = $1",
        payload.email
    )
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
    sqlx::query!(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2)",
        payload.email,
        password_hash
    )
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
    // Validate the incoming payload
    payload.validate()?;

    tracing::info!("Logging in user with email: {}", &payload.email);
    let user: User = sqlx::query_as!(
        User,
        "SELECT id as \"id!\", email, password_hash FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if !verify(&payload.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let tokens = issue_tokens(user.id, &state.db_pool, &state.app_config.jwt, None).await?;

    Ok(Json(tokens))
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
    // Hash the incoming refresh token to find it in the database.
    let mut hasher = Sha256::new();
    hasher.update(payload.refresh_token.as_bytes());
    let incoming_token_hash = hex::encode(hasher.finalize());

    // Find the token in the database by its hash.
    let record: RefreshTokenRecord = sqlx::query_as!(
        RefreshTokenRecord,
        "SELECT user_id, expires_at FROM refresh_tokens WHERE token_hash = $1",
        incoming_token_hash
    )
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    // Check if the database token has expired.
    if record.expires_at < Utc::now().naive_utc() {
        // As a cleanup, remove the expired token from the DB
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE token_hash = $1",
            incoming_token_hash
        )
        .execute(&state.db_pool)
        .await
        .ok(); // We don't care about the result of the cleanup
        return Err(AppError::Unauthorized);
    }

    // All checks passed. Rotate tokens: issue a new pair and invalidate the old refresh token.
    let tokens = issue_tokens(
        record.user_id,
        &state.db_pool,
        &state.app_config.jwt,
        Some(&incoming_token_hash), // Pass the old token hash to be deleted
    )
    .await?;

    Ok(Json(tokens))
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
    sqlx::query!("DELETE FROM refresh_tokens WHERE user_id = $1", user.id)
        .execute(&state.db_pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Middleware for JWT Authentication ---

pub async fn auth_middleware(
    State(state): State<AppState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    mut request: Request, // Note: changed to mutable
    next: Next,
) -> Result<Response, AppError> {
    let token = auth_header
        .ok_or(AppError::Unauthorized)?
        .token()
        .to_owned();

    let mut validation = Validation::default();
    validation.validate_exp = true;
    validation.leeway = 0;

    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(state.app_config.jwt.secret.as_ref()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized)?;

    let user_id: i64 = token_data
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::InternalServerError("Invalid user ID in token".to_string()))?;

    // Fetch the user from the database ONCE in the middleware
    let user = sqlx::query_as!(
        User,
        "SELECT id, email, password_hash FROM users WHERE id = $1",
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
