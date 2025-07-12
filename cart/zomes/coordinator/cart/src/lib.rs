use cart_integrity::*;
use hdk::prelude::*;
use serde::{Deserialize, Serialize};

mod cart;

// Input struct for updating delivery address
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateDeliveryAddressInput {
    pub previous_address_hash: ActionHash,
    pub new_address: Address,
}

// Input struct for adding cart item with quantity
#[derive(Serialize, Deserialize, Debug)]
pub struct AddCartItemInput {
    pub product: CartProduct,
    pub quantity: f64,
}

// Input struct for removing cart item by product_id and quantity
#[derive(Serialize, Deserialize, Debug)]
pub struct RemoveCartItemInput {
    pub product_id: String,
    pub quantity: f64,
}

// OPTIMIZED: Add cart item with quantity (new recommended function)
#[hdk_extern]
pub fn add_cart_item(input: AddCartItemInput) -> ExternResult<ActionHash> {
    cart::add_item_impl(input.product, input.quantity)
}

// OPTIMIZED: Remove cart item by product_id and quantity (new recommended function)  
#[hdk_extern]
pub fn remove_cart_item(input: RemoveCartItemInput) -> ExternResult<ActionHash> {
    cart::remove_item_impl(input.product_id, input.quantity)
}


// Get all current cart items using query()
#[hdk_extern]
pub fn get_current_items(_: ()) -> ExternResult<Vec<cart::CartProductWithHash>> {
    cart::get_current_items_impl()
}

// Get session status using query()
#[hdk_extern]
pub fn get_session_status(_: ()) -> ExternResult<Option<Record>> {
    cart::get_session_status_impl()
}

// Update session status to "AwaitingShopper"
#[hdk_extern]
pub fn publish_order(_: ()) -> ExternResult<ActionHash> {
    cart::publish_order_impl()
}

// Update session status back to "Building"
#[hdk_extern]
pub fn recall_order(_: ()) -> ExternResult<ActionHash> {
    cart::recall_order_impl()
}

// Set delivery address for first time
#[hdk_extern]
pub fn set_delivery_address(address: Address) -> ExternResult<ActionHash> {
    cart::set_delivery_address_impl(address)
}

// Update delivery address
#[hdk_extern]
pub fn update_delivery_address(input: UpdateDeliveryAddressInput) -> ExternResult<ActionHash> {
    cart::update_delivery_address_impl(input.previous_address_hash, input.new_address)
}

// Set delivery time slot
#[hdk_extern]
pub fn set_delivery_time_slot(time_slot: DeliveryTimeSlot) -> ExternResult<ActionHash> {
    cart::set_delivery_time_slot_impl(time_slot)
}

// Set delivery instructions
#[hdk_extern]
pub fn set_delivery_instructions(instructions: DeliveryInstructions) -> ExternResult<ActionHash> {
    cart::set_delivery_instructions_impl(instructions)
}

// Get all session data in one call
#[hdk_extern]
pub fn get_session_data(_: ()) -> ExternResult<cart::CartSessionData> {
    cart::get_session_data_impl()
}


