use crate::{error::AppError, web_server::AppState};
use axum::{extract::FromRequestParts, http::request::Parts};

// The struct is the same
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub id: i64,
    pub email: String,
}

// But the extractor logic changes completely
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // The middleware is responsible for putting AuthUser in extensions.
        // If it's not there, it's a 500 Internal Server Error because
        // the middleware should have been run.
        let user = parts.extensions.get::<AuthUser>().ok_or_else(|| {
            AppError::InternalServerError(
                "AuthUser not found in request extensions. Is the auth middleware missing?".into(),
            )
        })?;

        Ok(user.clone())
    }
}
