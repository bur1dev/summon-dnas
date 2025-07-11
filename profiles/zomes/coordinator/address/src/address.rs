use address_integrity::*;
use hdk::prelude::*;

pub fn create_address_impl(address: Address) -> ExternResult<ActionHash> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;

    warn!("🏠 PROFILES DNA: Creating private address entry: {} {}, {}", 
           address.street, address.city, address.state);

    // Create the address entry
    let address_hash = create_entry(EntryTypes::Address(address.clone()))?;
    
    warn!("✅ PROFILES DNA: Private address created with hash: {:?}", address_hash);

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
    warn!("🔄 PROFILES DNA: Updating private address with hash: {:?}", action_hash);
    
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    
    // Find and delete the old link pointing to the current address
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key.clone(), LinkTypes::AgentToAddress)?.build(),
    )?;
    
    for link in links {
        if let Some(target_hash) = link.target.into_action_hash() {
            if target_hash == action_hash {
                warn!("🔗 PROFILES DNA: Deleting old link to address");
                delete_link(link.create_link_hash)?;
                break;
            }
        }
    }
    
    // Create new address entry
    let new_address_hash = create_entry(EntryTypes::Address(address.clone()))?;
    
    warn!("✅ PROFILES DNA: Updated private address created with hash: {:?}", new_address_hash);
    
    // If this is the default address, we need to update other addresses
    if address.is_default {
        // Find existing default addresses and remove their default status
        let all_links = get_links(
            GetLinksInputBuilder::try_new(agent_pub_key.clone(), LinkTypes::AgentToAddress)?
                .build(),
        )?;

        for link in all_links {
            if let Some(target_hash) = link.target.into_action_hash() {
                if target_hash != new_address_hash {
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
                            // Create a new non-default version following immutability
                            let mut updated = existing_address.clone();
                            updated.is_default = false;
                            
                            // Delete old link
                            delete_link(link.create_link_hash)?;
                            
                            // Create new entry
                            let updated_hash = create_entry(EntryTypes::Address(updated))?;
                            
                            // Create new link
                            create_link(
                                agent_pub_key.clone(),
                                updated_hash,
                                LinkTypes::AgentToAddress,
                                LinkTag::new(""),
                            )?;
                        }
                    }
                }
            }
        }
    }
    
    // Create new link to the updated address
    create_link(
        agent_pub_key,
        new_address_hash.clone(),
        LinkTypes::AgentToAddress,
        LinkTag::new(""),
    )?;
    
    Ok(new_address_hash)
}

pub fn delete_address_impl(action_hash: ActionHash) -> ExternResult<ActionHash> {
    warn!("🗑️ PROFILES DNA: Deleting private address with hash: {:?}", action_hash);
    
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    
    // First, find and delete the link from agent to this address
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key, LinkTypes::AgentToAddress)?.build(),
    )?;
    
    for link in links {
        if let Some(target_hash) = link.target.into_action_hash() {
            if target_hash == action_hash {
                warn!("🔗 PROFILES DNA: Deleting link to address");
                delete_link(link.create_link_hash)?;
                break;
            }
        }
    }
    
    // Entry remains in DHT but becomes unreachable
    warn!("✅ PROFILES DNA: Private address link deleted successfully (entry remains in DHT)");
    Ok(action_hash)
}