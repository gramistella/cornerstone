use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[cfg(feature = "ts_export")]
use ts_rs::TS;

#[cfg(not(target_arch = "wasm32"))]
use sqlx::FromRow;
use validator::Validate;
pub mod utils;

#[cfg_attr(not(target_arch = "wasm32"), derive(FromRow))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Validate, ToSchema)]
#[cfg_attr(feature = "ts_export", derive(TS))] // Conditionally derive TS
#[serde(rename_all = "camelCase")]
pub struct ContactDto {
    #[schema(example = 1)]
    #[cfg_attr(feature = "ts_export", ts(type = "number"))]
    pub id: Option<i64>,
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    #[schema(example = "John Doe")]
    pub name: String,
    #[validate(email(message = "Email must be a valid email address"))]
    #[schema(example = "john.doe@example.com")]
    pub email: String,
    #[cfg_attr(feature = "ts_export", ts(type = "number"))]
    #[schema(example = 30)]
    pub age: i64,
    pub subscribed: bool,
    #[validate(length(min = 1, message = "Contact type cannot be empty"))]
    #[schema(example = "Friend")]
    pub contact_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate, ToSchema)]
#[cfg_attr(feature = "ts_export", derive(TS))]
pub struct Credentials {
    #[validate(email)]
    #[schema(example = "test@example.com")]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    #[schema(example = "password123")]
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
#[cfg_attr(feature = "ts_export", derive(TS))]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
}
