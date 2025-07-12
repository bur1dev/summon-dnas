use hdi::prelude::*;

// Link tag structure for storing cart quantity and timestamp data
// Following the established pattern from products.rs
pub struct CartQuantityTag {
    pub quantity: f64,    // 8 bytes - supports both unit counts (1, 2, 3) and weight (0.25, 0.50, 0.75)
    pub timestamp: u64,   // 8 bytes - when the quantity was last updated
    // Total: 16 bytes (well under Holochain's 500-byte link tag limit)
}

impl CartQuantityTag {
    // Serialize quantity tag to bytes following products.rs little-endian pattern
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut tag_bytes = Vec::new();
        tag_bytes.extend_from_slice(&self.quantity.to_le_bytes());
        tag_bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        tag_bytes
    }
    
    // Deserialize quantity tag from bytes with error handling (like products.rs)
    pub fn from_bytes(bytes: &[u8]) -> (f64, u64) {
        if bytes.len() >= 16 {
            let qty_bytes: [u8; 8] = bytes[0..8].try_into().unwrap_or([0; 8]);
            let time_bytes: [u8; 8] = bytes[8..16].try_into().unwrap_or([0; 8]);
            (f64::from_le_bytes(qty_bytes), u64::from_le_bytes(time_bytes))
        } else {
            (0.0, 0)  // Default values for malformed tags
        }
    }
    
    // Helper to create LinkTag from CartQuantityTag
    pub fn to_link_tag(&self) -> LinkTag {
        LinkTag::new(self.to_bytes())
    }
    
    // Helper to read CartQuantityTag from LinkTag 
    pub fn from_link_tag(link_tag: &LinkTag) -> (f64, u64) {
        Self::from_bytes(&link_tag.0)
    }
}

// Individual cart item - PUBLIC DHT entry (one per unique product)
// QUANTITY AND TIMESTAMP NOW STORED IN LINK TAGS for performance optimization
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

    // This field will store any snapshotted product preferences or customer notes.
    pub note: Option<String>,
    
    // --- CART-SPECIFIC DATA MOVED TO LINK TAGS ---
    // quantity: f64,    // NOW IN LINK TAG via CartQuantityTag
    // timestamp: u64,   // NOW IN LINK TAG via CartQuantityTag
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

