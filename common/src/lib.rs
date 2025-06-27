use serde::{Serialize, Deserialize};

pub mod utils;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)] // Added Debug and PartialEq for convenience
pub struct ContactDto {
    pub id: Option<u32>, // Using Option<u32> for flexibility (e.g., when creating a new contact without an ID yet)
    pub name: String,
    pub email: String,
    pub age: u32,       // Added age
    pub subscribed: bool, // Added subscribed
    pub contact_type: String, // Added contact_type
}

