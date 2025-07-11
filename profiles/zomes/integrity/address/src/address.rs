use hdi::prelude::*;

#[hdk_entry_helper]
#[derive(Clone)]
pub struct Address {
    pub street: String,
    pub unit: Option<String>,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub lat: f64,
    pub lng: f64,
    pub is_default: bool,
    pub label: Option<String>, // "Home", "Work", etc.
}