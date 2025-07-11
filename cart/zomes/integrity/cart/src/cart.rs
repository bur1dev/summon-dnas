use hdi::prelude::*;

// Individual cart item - PUBLIC DHT entry (each "Add to Cart" creates one)
#[hdk_entry_helper]
#[derive(Clone)]
pub struct CartProduct {
    // A unique, permanent string identifier for the product.
    // This will be created on the frontend as `${group_hash}:${product_index}`.
    pub product_id: String,
    pub upc: Option<String>,

    // --- ALL DATA BELOW IS A SNAPSHOT ---
    pub product_name: String,
    pub product_image_url: Option<String>,

    // The price is frozen at the time of adding to the cart. This is the source of truth.
    pub price_at_checkout: f64,
    pub promo_price: Option<f64>,
    
    // How the product is sold - "UNIT" or "WEIGHT" - needed for correct increment/decrement behavior
    pub sold_by: Option<String>,

    // --- CART-SPECIFIC DATA ---
    pub quantity: f64,
    pub timestamp: u64,

    // This field will store any snapshotted product preferences or customer notes.
    pub note: Option<String>,
}

// Simple entry to track cart session state - PUBLIC DHT entry
#[hdk_entry_helper]
#[derive(Clone)]
pub struct SessionStatus {
    pub status: String, // "Building" or "AwaitingShopper"
    pub last_updated: u64,
}

// Delivery instructions - PUBLIC DHT entry
#[hdk_entry_helper]
#[derive(Clone)]
pub struct DeliveryInstructions {
    pub instructions: String,
    pub timestamp: u64,
}

// Delivery time slot - PUBLIC DHT entry
#[hdk_entry_helper]
#[derive(Clone)]
pub struct DeliveryTimeSlot {
    pub date: u64,         // Unix timestamp for the date
    pub time_slot: String, // e.g., "2pm-4pm"
}

