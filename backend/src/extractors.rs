use axum::{
    extract::{FromRequestParts},
    http::{request::Parts},
};
use crate::{auth::Claims, error::AppError, web_server::AppState};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::{decode, DecodingKey, Validation};

// This struct will be the extractor
pub struct AuthUser {
    pub id: i64,
    pub email: String, // You could add more user fields if needed
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::Unauthorized)?;

        // Decode the user data
        let token_data = decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(state.app_config.jwt_secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        // Find the user in the database to ensure they still exist
        let user_id: i64 = token_data.claims.sub.parse()
            .map_err(|_| AppError::InternalServerError("Invalid user ID in token".to_string()))?;

        let user = sqlx::query_as!(
            crate::auth::User, // Assuming User struct is public in auth module
            "SELECT id, email, password_hash FROM users WHERE id = ?",
            user_id
        )
        .fetch_optional(&state.db_pool)
        .await?
        .ok_or(AppError::Unauthorized)?; // User not found, token is invalid

        Ok(AuthUser { id: user.id, email: user.email })
    }
}