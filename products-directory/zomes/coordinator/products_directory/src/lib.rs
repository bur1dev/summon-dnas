use hdk::prelude::*;
use products_directory_integrity::*;

#[hdk_extern]
pub fn init() -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}

// Get the currently active catalog network seed
#[hdk_extern]
pub fn get_active_catalog(_: ()) -> ExternResult<Option<String>> {
    let anchor_path = Path::from("active_product_catalog");
    let anchor_hash = anchor_path.path_entry_hash()?;
    
    let links = get_links(
        GetLinksInputBuilder::try_new(anchor_hash, LinkTypes::Catalog)?
            .tag_prefix(LinkTag::new("active"))
            .build()
    )?;
    
    if let Some(link) = links.into_iter().next() {
        if let Some(target_hash) = link.target.into_action_hash() {
            if let Some(record) = get(target_hash, GetOptions::default())? {
                let entry: ActiveProductCatalog = record
                    .entry()
                    .to_app_option()
                    .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to deserialize entry: {:?}", e))))?
                    .ok_or(wasm_error!(WasmErrorInner::Guest("Entry not found".to_string())))?;
                
                return Ok(Some(entry.network_seed));
            }
        }
    }
    
    Ok(None)
}

// Update the active catalog seed
#[hdk_extern]
pub fn update_active_catalog(seed: String) -> ExternResult<ActionHash> {
    if seed.trim().is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Network seed cannot be empty".to_string()
        )));
    }
    
    let anchor_path = Path::from("active_product_catalog");
    let anchor_hash = anchor_path.path_entry_hash()?;
    
    // Delete old links
    let existing_links = get_links(
        GetLinksInputBuilder::try_new(anchor_hash.clone(), LinkTypes::Catalog)?
            .tag_prefix(LinkTag::new("active"))
            .build()
    )?;
    
    for link in existing_links {
        delete_link(link.create_link_hash)?;
    }
    
    // Create new entry
    let catalog_entry = ActiveProductCatalog {
        network_seed: seed.clone(),
    };
    
    let action_hash = create_entry(&EntryTypes::ActiveProductCatalog(catalog_entry))?;
    
    create_link(
        anchor_hash,
        action_hash.clone(),
        LinkTypes::Catalog,
        LinkTag::new("active")
    )?;
    
    info!("Updated active catalog: {}", seed);
    
    Ok(action_hash)
}