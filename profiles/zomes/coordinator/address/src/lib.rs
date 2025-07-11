use address_integrity::*;
use hdk::prelude::*;

mod address;

// Address functions
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
pub fn delete_address(address_hash: ActionHash) -> ExternResult<ActionHash> {
    address::delete_address_impl(address_hash)
}