use serde::{Serialize, Deserialize};

#[cfg(not(target_arch = "wasm32"))]
use sqlx::FromRow;

pub mod utils;

#[cfg_attr(not(target_arch = "wasm32"), derive(FromRow))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)] // Added Debug and PartialEq for convenience
pub struct ContactDto {
    pub id: Option<i64>, 
    pub name: String,
    pub email: String,
    pub age: i64,       // Added age
    pub subscribed: bool, // Added subscribed
    pub contact_type: String, // Added contact_type
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginResponse {
    pub token: String,
}
