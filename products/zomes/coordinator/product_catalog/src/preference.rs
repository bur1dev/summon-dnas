use products_integrity::*;
use hdk::prelude::*;

// Implementation of save_product_preference
pub(crate) fn save_product_preference_impl(preference: ProductPreference) -> ExternResult<ActionHash> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    
    // Create the preference entry
    let preference_hash = create_entry(EntryTypes::ProductPreference(preference.clone()))?;
    
    // Create a tag with group_hash and product_index for efficient queries
    let tag = LinkTag::new(format!("{}-{}", 
        preference.group_hash.to_string(), 
        preference.product_index.to_string()));
    
    // Get existing preferences for this product to check if we need to update
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key.clone(), LinkTypes::AgentToPreference)?
            .tag_prefix(tag.clone())
            .build(),
    )?;
    
    // Delete any existing links for this product
    for link in links {
        delete_link(link.create_link_hash)?;
    }
    
    // Link the agent to this preference
    create_link(
        agent_pub_key,
        preference_hash.clone(),
        LinkTypes::AgentToPreference,
        tag,
    )?;
    
    Ok(preference_hash)
}

// Implementation of get_product_preferences - gets all preferences
pub(crate) fn get_product_preferences_impl() -> ExternResult<Vec<(ActionHash, ProductPreference)>> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    
    // Get all preference links
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key, LinkTypes::AgentToPreference)?.build(),
    )?;
    
    let mut preferences = Vec::new();
    
    for link in links {
        if let Some(target_hash) = link.target.into_action_hash() {
            match get(target_hash.clone(), GetOptions::default())? {
                Some(record) => {
                    let preference: ProductPreference = record
                        .entry()
                        .to_app_option()
                        .map_err(|e| {
                            wasm_error!(WasmErrorInner::Guest(format!(
                                "Failed to deserialize: {}",
                                e
                            )))
                        })?
                        .ok_or(wasm_error!(WasmErrorInner::Guest(
                            "Expected app entry".to_string()
                        )))?;
                    
                    preferences.push((target_hash, preference));
                }
                None => continue,
            }
        }
    }
    
    Ok(preferences)
}

// Implementation of get_product_preference_by_product - get preference for specific product
pub(crate) fn get_product_preference_by_product_impl(
    group_hash: ActionHash,
    product_index: u32,
) -> ExternResult<Option<(ActionHash, ProductPreference)>> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    
    // Create tag to query specifically for this product
    let tag = LinkTag::new(format!("{}-{}", 
        group_hash.to_string(), 
        product_index.to_string()));
    
    // Get links with the specific tag for this product
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key, LinkTypes::AgentToPreference)?
            .tag_prefix(tag)
            .build(),
    )?;
    
    // Return the first matching preference (should only be one)
    if let Some(link) = links.first() {
        if let Some(target_hash) = link.target.clone().into_action_hash() {
            match get(target_hash.clone(), GetOptions::default())? {
                Some(record) => {
                    let preference: ProductPreference = record
                        .entry()
                        .to_app_option()
                        .map_err(|e| {
                            wasm_error!(WasmErrorInner::Guest(format!(
                                "Failed to deserialize: {}",
                                e
                            )))
                        })?
                        .ok_or(wasm_error!(WasmErrorInner::Guest(
                            "Expected app entry".to_string()
                        )))?;
                    
                    return Ok(Some((target_hash, preference)));
                }
                None => return Ok(None),
            }
        }
    }
    
    Ok(None)
}

// Implementation of update_product_preference
pub(crate) fn update_product_preference_impl(
    action_hash: ActionHash,
    preference: ProductPreference,
) -> ExternResult<ActionHash> {
    // Simply update the entry
    let updated_hash = update_entry(action_hash, preference.clone())?;
    
    // We don't need to update links since the product identifiers haven't changed
    
    Ok(updated_hash)
}

// Implementation of delete_product_preference
pub(crate) fn delete_product_preference_impl(action_hash: ActionHash) -> ExternResult<ActionHash> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    
    // Get the preference record to find the group_hash and product_index
    let preference_record = must_get_valid_record(action_hash.clone())?;
    let preference: ProductPreference = preference_record
        .entry()
        .to_app_option()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to deserialize: {}",
                e
            )))
        })?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Expected app entry".to_string()
        )))?;
    
    // Create the tag that would have been used
    let tag = LinkTag::new(format!("{}-{}", 
        preference.group_hash.to_string(), 
        preference.product_index.to_string()));
    
    // Get links to this preference
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key, LinkTypes::AgentToPreference)?
            .tag_prefix(tag)
            .build(),
    )?;
    
    // Delete all matching links
    for link in links {
        if let Some(target_hash) = link.target.into_action_hash() {
            if target_hash == action_hash {
                delete_link(link.create_link_hash)?;
                break;
            }
        }
    }
    
    // Delete the entry itself
    delete_entry(action_hash)
}