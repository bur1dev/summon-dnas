use hdi::prelude::*;
use crate::DeliveryTimeSlot;

// For storing checked out carts
#[hdk_entry_helper]
#[derive(Clone)]
pub struct CheckedOutCart {
    pub id: String,
    pub products: Vec<CartProduct>,
    pub total: f64,
    pub created_at: u64,
    pub status: String, // "processing", "completed", "returned", "claimed", "awaiting_shopper"
    pub delivery_time: Option<DeliveryTimeSlot>,
    // New fields for secure workflow
    pub customer_pub_key: AgentPubKey,
    pub general_location: Option<String>,
}

#[hdk_entry_helper]
#[derive(Clone)]
pub struct CartProduct {
    // A unique, permanent string identifier for the product.
    // This will be created on the frontend as `${group_hash}:${product_index}`.
    pub product_id: String,

    // --- ALL DATA BELOW IS A SNAPSHOT ---
    pub product_name: String,
    pub product_image_url: Option<String>,

    // The price is frozen at the time of adding to the cart. This is the source of truth.
    pub price_at_checkout: f64,
    pub promo_price: Option<f64>,

    // --- CART-SPECIFIC DATA (Unchanged) ---
    pub quantity: f64,
    pub timestamp: u64,

    // This field will store any snapshotted product preferences or customer notes.
    pub note: Option<String>,
}

// New structure for the private cart (stored as private entry)
#[hdk_entry_helper]
#[derive(Clone)]
pub struct PrivateCart {
    pub items: Vec<CartProduct>,
    pub last_updated: u64,
}

