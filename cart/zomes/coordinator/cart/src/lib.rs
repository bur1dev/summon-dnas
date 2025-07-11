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

// Add individual cart item
#[hdk_extern]
pub fn add_item(item: CartProduct) -> ExternResult<ActionHash> {
    cart::add_item_impl(item)
}

// Remove individual cart item
#[hdk_extern]
pub fn remove_item(item_hash: ActionHash) -> ExternResult<ActionHash> {
    cart::remove_item_impl(item_hash)
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


