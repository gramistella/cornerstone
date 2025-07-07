use serde::{Deserialize, Serialize};

#[cfg(feature = "ts_export")]
use ts_rs::TS;

#[cfg(not(target_arch = "wasm32"))]
use sqlx::FromRow;
use validator::Validate;
pub mod utils;

#[cfg_attr(not(target_arch = "wasm32"), derive(FromRow))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Validate)]
#[cfg_attr(feature = "ts_export", derive(TS))] // Conditionally derive TS
#[cfg_attr(feature = "ts_export", ts(export))] // Just 'export', not 'export_to'
pub struct ContactDto {
    #[cfg_attr(feature = "ts_export", ts(type = "number"))]
    pub id: Option<i64>,
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,
    #[validate(email(message = "Email must be a valid email address"))]
    pub email: String,
    #[cfg_attr(feature = "ts_export", ts(type = "number"))]
    pub age: i64,
    pub subscribed: bool,
    #[validate(length(min = 1, message = "Contact type cannot be empty"))]
    pub contact_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
pub struct Credentials {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
}
