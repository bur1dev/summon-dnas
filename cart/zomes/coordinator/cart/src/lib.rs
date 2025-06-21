use cart_integrity::*;
use hdk::prelude::*;

mod address;
mod cart;

#[derive(Serialize, Deserialize, Debug)]
pub struct AddToPrivateCartInput {
    pub product_id: String,
    pub product_name: String,
    pub product_image_url: Option<String>,
    pub price_at_checkout: f64,
    pub promo_price: Option<f64>,
    pub quantity: f64,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CartItemInput {
    pub product_id: String,
    pub product_name: String,
    pub product_image_url: Option<String>,
    pub price_at_checkout: f64,
    pub promo_price: Option<f64>,
    pub quantity: f64,
    pub timestamp: u64,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReplacePrivateCartInput {
    pub items: Vec<CartItemInput>,
    pub last_updated: u64,
}

// Return type for get_checked_out_carts
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckedOutCartWithHash {
    pub cart_hash: ActionHash,
    pub cart: CheckedOutCart,
}

// Extended checkout input with delivery details and cart products
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckoutCartInput {
    pub private_address_hash: ActionHash,
    pub delivery_time: Option<DeliveryTimeSlot>,
    pub cart_products: Option<Vec<CartProduct>>, // Added to pass cart products
}

// NEW: Replace the entire private cart (optimized approach)
#[hdk_extern]
pub fn replace_private_cart(input: ReplacePrivateCartInput) -> ExternResult<()> {
    cart::replace_private_cart_impl(input)
}

// Add product to private cart (new function)
#[hdk_extern]
pub fn add_to_private_cart(input: AddToPrivateCartInput) -> ExternResult<()> {
    cart::add_to_private_cart_impl(input)
}

// Get private cart (new function)
#[hdk_extern]
pub fn get_private_cart(_: ()) -> ExternResult<PrivateCart> {
    cart::get_private_cart_impl()
}

// Check out all items in the cart with delivery details
#[hdk_extern]
pub fn checkout_cart(input: CheckoutCartInput) -> ExternResult<ActionHash> {
    cart::checkout_cart_impl(input)
}

// Get all checked out carts
#[hdk_extern]
pub fn get_checked_out_carts(_: ()) -> ExternResult<Vec<CheckedOutCartWithHash>> {
    cart::get_checked_out_carts_impl()
}

// Helper to get a single checked out cart
#[hdk_extern]
pub fn get_checked_out_cart(action_hash: ActionHash) -> ExternResult<Option<CheckedOutCart>> {
    cart::get_checked_out_cart_impl(action_hash)
}

// Return a checked out cart to shopping
#[hdk_extern]
pub fn return_to_shopping(cart_hash: ActionHash) -> ExternResult<()> {
    cart::return_to_shopping_impl(cart_hash)
}

// Address-related functions
#[hdk_extern]
pub fn create_address(address: Address) -> ExternResult<ActionHash> {
    address::create_address_impl(address)
}

#[hdk_extern]
pub fn get_addresses(_: ()) -> ExternResult<Vec<(ActionHash, Address)>> {
    address::get_addresses_impl()
}

#[hdk_extern]
pub fn get_address(action_hash: ActionHash) -> ExternResult<Option<Address>> {
    address::get_address_impl(action_hash)
}

#[hdk_extern]
pub fn update_address(input: (ActionHash, Address)) -> ExternResult<ActionHash> {
    address::update_address_impl(input.0, input.1)
}

#[hdk_extern]
pub fn delete_address(action_hash: ActionHash) -> ExternResult<ActionHash> {
    address::delete_address_impl(action_hash)
}


// NEW: Customer address retrieval for orders
#[hdk_extern]
pub fn get_address_for_order(order_hash: ActionHash) -> ExternResult<Address> {
    cart::get_address_for_order_impl(order_hash)
}

// SPECIALIZED: Create order address copy - for immutable shipping labels only
#[hdk_extern]
pub fn create_order_address_copy(address: Address) -> ExternResult<ActionHash> {
    address::create_order_address_copy_impl(address)
}
