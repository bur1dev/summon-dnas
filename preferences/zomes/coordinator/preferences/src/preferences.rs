use preferences_integrity::*;
use hdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SavePreferenceInput {
    pub upc: String,
    pub note: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetPreferenceInput {
    pub upc: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeletePreferenceInput {
    pub upc: String,
}

// Get UPC-specific path for direct targeting
fn get_upc_path(upc: &str) -> ExternResult<Path> {
    Ok(Path::try_from(format!("upc_{}", upc))?)
}

// Save or update preference implementation (ultra-simple)
pub fn save_preference_impl(input: SavePreferenceInput) -> ExternResult<ActionHash> {
    let preference = UpcPreference {
        upc: input.upc.clone(),
        note: input.note,
    };

    let path = get_upc_path(&input.upc)?;
    let path_hash = path.path_entry_hash()?;
    
    // Check if preference already exists
    let links = get_links(
        GetLinksInputBuilder::try_new(path_hash.clone(), LinkTypes::UpcPathToPreference)?
            .build()
    )?;

    if let Some(link) = links.into_iter().last() {
        // Update existing preference - CORRECT PATTERN: delete old link, create new entry, create new link
        let target_hash = link.target.into_action_hash().ok_or(wasm_error!("Invalid target hash"))?;
        let updated_hash = update_entry(target_hash, EntryTypes::UpcPreference(preference))?;
        
        // Delete old link and create new link to updated entry
        delete_link(link.create_link_hash)?;
        create_link(path_hash, updated_hash.clone(), LinkTypes::UpcPathToPreference, ())?;
        
        Ok(updated_hash)
    } else {
        // Create new preference + link
        let hash = create_entry(EntryTypes::UpcPreference(preference))?;
        create_link(path_hash, hash.clone(), LinkTypes::UpcPathToPreference, ())?;
        Ok(hash)
    }
}

// Get preference by UPC implementation  
pub fn get_preference_impl(input: GetPreferenceInput) -> ExternResult<Option<UpcPreference>> {
    let path = get_upc_path(&input.upc)?;
    let links = get_links(
        GetLinksInputBuilder::try_new(path.path_entry_hash()?, LinkTypes::UpcPathToPreference)?
            .build()
    )?;

    // Get the latest preference
    if let Some(link) = links.into_iter().last() {
        let target_hash = link.target.into_action_hash().ok_or(wasm_error!("Invalid target hash"))?;
        if let Some(record) = get(target_hash, GetOptions::default())? {
            let preference: Option<UpcPreference> = record.entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?;
            return Ok(preference);
        }
    }
    Ok(None)
}

// Delete preference implementation
pub fn delete_preference_impl(input: DeletePreferenceInput) -> ExternResult<()> {
    let path = get_upc_path(&input.upc)?;
    let links = get_links(
        GetLinksInputBuilder::try_new(path.path_entry_hash()?, LinkTypes::UpcPathToPreference)?
            .build()
    )?;

    // Delete link and entry (keep it simple)
    if let Some(link) = links.into_iter().last() {
        delete_link(link.create_link_hash)?;
        let target_hash = link.target.into_action_hash().ok_or(wasm_error!("Invalid target hash"))?;
        delete_entry(target_hash)?;
    }
    Ok(())
}