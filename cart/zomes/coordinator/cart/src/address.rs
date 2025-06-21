use cart_integrity::*;
use hdk::prelude::*;

pub fn create_address_impl(address: Address) -> ExternResult<ActionHash> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;

    // Create the address entry
    let address_hash = create_entry(EntryTypes::Address(address.clone()))?;

    // If this is the default address, we need to update other addresses
    if address.is_default {
        // Find existing default addresses and remove their default status
        let links = get_links(
            GetLinksInputBuilder::try_new(agent_pub_key.clone(), LinkTypes::AgentToAddress)?
                .build(),
        )?;

        for link in links {
            if let Some(target_hash) = link.target.into_action_hash() {
                if target_hash != address_hash {
                    // Get the address entry directly using must_get_valid_record
                    if let Ok(record) = must_get_valid_record(target_hash.clone()) {
                        // Manually handle the error conversion
                        let existing_address = match record.entry().to_app_option::<Address>() {
                            Ok(Some(addr)) => addr,
                            Ok(None) => continue,
                            Err(e) => {
                                return Err(wasm_error!(WasmErrorInner::Guest(format!(
                                    "Failed to deserialize: {}",
                                    e
                                ))))
                            }
                        };

                        if existing_address.is_default {
                            // Create a new non-default version
                            let mut updated = existing_address.clone();
                            updated.is_default = false;
                            update_entry(target_hash, updated)?;
                        }
                    }
                }
            }
        }
    }

    // Link the agent to this address
    create_link(
        agent_pub_key,
        address_hash.clone(),
        LinkTypes::AgentToAddress,
        LinkTag::new(""),
    )?;

    Ok(address_hash)
}

pub fn get_addresses_impl() -> ExternResult<Vec<(ActionHash, Address)>> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;

    // Get all address links
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key, LinkTypes::AgentToAddress)?.build(),
    )?;

    let mut addresses = Vec::new();

    for link in links {
        if let Some(target_hash) = link.target.into_action_hash() {
            // Get the address entry directly using must_get_valid_record
            if let Ok(record) = must_get_valid_record(target_hash.clone()) {
                // Manually handle the error conversion
                let address = match record.entry().to_app_option::<Address>() {
                    Ok(Some(addr)) => addr,
                    Ok(None) => continue,
                    Err(e) => {
                        return Err(wasm_error!(WasmErrorInner::Guest(format!(
                            "Failed to deserialize: {}",
                            e
                        ))))
                    }
                };

                addresses.push((target_hash, address));
            }
        }
    }

    Ok(addresses)
}

pub fn get_address_impl(action_hash: ActionHash) -> ExternResult<Option<Address>> {
    // Get the record
    match get(action_hash.clone(), GetOptions::default())? {
        Some(record) => {
            // Manually handle the error conversion
            match record.entry().to_app_option::<Address>() {
                Ok(maybe_address) => Ok(maybe_address),
                Err(e) => Err(wasm_error!(WasmErrorInner::Guest(format!(
                    "Failed to deserialize: {}",
                    e
                )))),
            }
        }
        None => Ok(None),
    }
}

pub fn update_address_impl(action_hash: ActionHash, address: Address) -> ExternResult<ActionHash> {
    update_entry(action_hash, address)
}

pub fn delete_address_impl(action_hash: ActionHash) -> ExternResult<ActionHash> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    
    // Remove the link from agent to address (entries are immutable in Holochain)
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key.clone(), LinkTypes::AgentToAddress)?.build(),
    )?;
    
    for link in links {
        if let Some(target_hash) = link.target.into_action_hash() {
            if target_hash == action_hash {
                delete_link(link.create_link_hash)?;
                break;
            }
        }
    }
    
    Ok(action_hash)
}

// SPECIALIZED: Create order address copy - creates private address without linking to agent
// This creates immutable "shipping label" addresses 
pub fn create_order_address_copy_impl(address: Address) -> ExternResult<ActionHash> {
    // Create the private address entry - NO agent linking
    let address_hash = create_entry(EntryTypes::Address(address))?;
    
    // Return hash for OrderToPrivateAddress linking in checkout
    Ok(address_hash)
}
