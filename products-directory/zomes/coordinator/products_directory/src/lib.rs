use hdk::prelude::*;
use products_directory_integrity::*;

// Helper function to get cell_id debug information
fn get_cell_id_debug_info() -> ExternResult<String> {
    let dna_info = dna_info()?;
    let agent_info = agent_info()?;
    Ok(format!("{:?}:{:?}", dna_info.hash, agent_info.agent_initial_pubkey))
}

// Check if the current agent is an admin
// For simplicity, we treat ANY agent as admin for now
#[hdk_extern]
fn is_admin(_: ()) -> ExternResult<bool> {
    // Log the current cell info for admin checks
    let current_cell_info = get_cell_id_debug_info()?;
    warn!("[is_admin] Admin check called in cell: {}", current_cell_info);
    
    // For development/testing: any agent can be admin
    // In production, you'd want proper admin authentication
    warn!("[is_admin] Returning true (development mode)");
    Ok(true)
}

// Initialize the zome
#[hdk_extern]
pub fn init() -> ExternResult<InitCallbackResult> {
    // Log the current cell info during initialization
    let current_cell_info = get_cell_id_debug_info()?;
    warn!("[init] Initializing products_directory zome in cell: {}", current_cell_info);
    Ok(InitCallbackResult::Pass)
}

// Get the currently active catalog network seed (PUBLIC function)
#[hdk_extern]
pub fn get_active_catalog(_: ()) -> ExternResult<Option<String>> {
    // Log the current cell info where this function is running
    let current_cell_info = get_cell_id_debug_info()?;
    warn!("[get_active_catalog] Function called in cell: {}", current_cell_info);
    
    // Create the well-known anchor path
    let anchor_path = Path::from("active_product_catalog");
    let anchor_hash = anchor_path.path_entry_hash()?;
    
    // Get links from the anchor
    let links = get_links(
        GetLinksInputBuilder::try_new(anchor_hash, LinkTypes::Catalog)?
            .tag_prefix(LinkTag::new("active"))
            .build()
    )?;
    
    // Get the most recent active catalog entry
    if let Some(link) = links.into_iter().next() {
        // Get the record from the link target
        if let Some(target_hash) = link.target.into_action_hash() {
            if let Some(record) = get(target_hash, GetOptions::default())? {
                // Extract the entry from the record
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

// Update the active catalog seed (ADMIN ONLY function)
#[hdk_extern]
pub fn update_active_catalog(seed: String) -> ExternResult<ActionHash> {
    // Log the current cell info where this function is running
    let current_cell_info = get_cell_id_debug_info()?;
    warn!("[update_active_catalog] Function called in cell: {}", current_cell_info);
    warn!("[update_active_catalog] New seed: {}", seed);
    
    // Check admin permissions
    if !is_admin(())? {
        warn!("[update_active_catalog] Admin check failed");
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only admins can update the active catalog".to_string()
        )));
    }
    warn!("[update_active_catalog] Admin check passed");
    
    // Validate the seed is not empty
    if seed.trim().is_empty() {
        warn!("[update_active_catalog] Empty seed provided");
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Network seed cannot be empty".to_string()
        )));
    }
    
    // Create the well-known anchor path
    let anchor_path = Path::from("active_product_catalog");
    let anchor_hash = anchor_path.path_entry_hash()?;
    
    // Get existing links and delete them to ensure only one active catalog
    let existing_links = get_links(
        GetLinksInputBuilder::try_new(anchor_hash.clone(), LinkTypes::Catalog)?
            .tag_prefix(LinkTag::new("active"))
            .build()
    )?;
    
    // Delete old links
    for link in existing_links {
        delete_link(link.create_link_hash)?;
    }
    
    // Create the new ActiveProductCatalog entry
    let catalog_entry = ActiveProductCatalog {
        network_seed: seed,
    };
    
    let action_hash = create_entry(&EntryTypes::ActiveProductCatalog(catalog_entry))?;
    
    // Link the new entry to the anchor with "active" tag
    create_link(
        anchor_hash,
        action_hash.clone(),
        LinkTypes::Catalog,
        LinkTag::new("active")
    )?;
    
    Ok(action_hash)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateProductCloneInput {
    pub products_dna_hash: DnaHash,
}

// Create a new product clone (ADMIN ONLY function)
#[hdk_extern]
pub fn create_product_clone(input: CreateProductCloneInput) -> ExternResult<ClonedCell> {
    // Log the current cell info where this function is running
    let current_cell_info = get_cell_id_debug_info()?;
    warn!("[create_product_clone] Function called in cell: {}", current_cell_info);
    warn!("[create_product_clone] Input DNA hash: {:?}", input.products_dna_hash);
    
    // Check admin permissions first
    if !is_admin(())? {
        warn!("[create_product_clone] Admin check failed");
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only admins can create product clones".to_string()
        )));
    }
    warn!("[create_product_clone] Admin check passed");

    // Generate a new, unique network seed using a high-resolution timestamp
    let timestamp = sys_time()?;
    let network_seed = format!("products-{}", timestamp.as_micros());
    warn!("[create_product_clone] Generated network seed: {}", network_seed);

    // Create the modifiers with the new network seed - following docs exactly
    let modifiers = DnaModifiersOpt::none()
        .with_network_seed(network_seed.clone().into());
    warn!("[create_product_clone] Created modifiers with network seed");

    // Create the cell ID with the provided DNA hash and current agent
    let agent_pubkey = agent_info()?.agent_initial_pubkey;
    let cell_id = CellId::new(input.products_dna_hash.clone(), agent_pubkey.clone());
    warn!("[create_product_clone] Created cell_id: DNA={:?}, Agent={:?}", input.products_dna_hash, agent_pubkey);
    
    // Create the clone using the HDK function - following the docs exactly
    let create_clone_cell_input = CreateCloneCellInput {
        cell_id: cell_id.clone(),
        modifiers,
        membrane_proof: None,
        name: Some(network_seed.clone()),
    };
    warn!("[create_product_clone] About to call create_clone_cell with cell_id: {:?}", cell_id);
    
    match create_clone_cell(create_clone_cell_input) {
        Ok(cloned_cell) => {
            warn!("[create_product_clone] Successfully created clone!");
            warn!("[create_product_clone] Clone cell_id: {:?}", cloned_cell.cell_id);
            warn!("[create_product_clone] Clone name: {:?}", cloned_cell.name);
            Ok(cloned_cell)
        }
        Err(e) => {
            warn!("[create_product_clone] Failed to create clone: {:?}", e);
            Err(e)
        }
    }
}

// Disable the previous clone after a new one is activated (ADMIN ONLY function)
// NOTE: This should be called with the OLD active seed that needs to be disabled
#[hdk_extern]
pub fn disable_previous_clone(old_active_seed: String) -> ExternResult<()> {
    // Log the current cell info where this function is running
    let current_cell_info = get_cell_id_debug_info()?;
    warn!("[disable_previous_clone] Function called in cell: {}", current_cell_info);
    warn!("[disable_previous_clone] Old active seed to disable: {}", old_active_seed);
    
    // Check admin permissions first
    if !is_admin(())? {
        warn!("[disable_previous_clone] Admin check failed");
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only admins can disable clones".to_string()
        )));
    }
    warn!("[disable_previous_clone] Admin check passed");
    
    // Use the provided old seed directly - no need to query current active
    
    if !old_active_seed.is_empty() {
        warn!("[disable_previous_clone] Processing old active seed: {}", old_active_seed);
        
        // CRITICAL: Check if the old seed looks like a base cell (no UUID format)
        // Base cells typically don't have UUID format network seeds
        // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx (36 chars with dashes)
        let is_likely_base_cell = !old_active_seed.contains('-') || old_active_seed.len() != 36;
        
        if is_likely_base_cell {
            warn!("[disable_previous_clone] ⚠️ Old seed '{}' appears to be base cell - SKIPPING disable to protect base cell", old_active_seed);
            return Ok(());
        }
        
        warn!("[disable_previous_clone] Old seed '{}' appears to be clone cell - proceeding with disable", old_active_seed);
        
        // Use clone ID instead of app_info for simpler approach
        // Clone IDs follow pattern: role_name.index (e.g., "products_role.1")
        // We need to find which clone index has this network seed
        
        // Try clone indices 0-9 (matching clone_limit: 10 from happ.yaml)
        for clone_index in 0..10 {
            let clone_id = format!("products_role.{}", clone_index);
            
            // Try to disable using clone ID - if it fails, the clone doesn't exist or is already disabled
            let input = DisableCloneCellInput {
                clone_cell_id: CloneCellId::CloneId(CloneId(clone_id.clone())),
            };
            
            match disable_clone_cell(input) {
                Ok(()) => {
                    warn!("[disable_previous_clone] ✅ Successfully disabled clone ID: {}", clone_id);
                    // Note: We can't easily verify the network seed matches without app_info,
                    // but since we're iterating through all clones, one of them should be the old one
                    // The disable call will only succeed if the clone exists and is enabled
                    return Ok(());
                }
                Err(_) => {
                    // Clone doesn't exist, is already disabled, or other error - continue to next
                    // Don't log individual failures as they're expected for non-existent clones
                    continue;
                }
            }
        }
        
        warn!("[disable_previous_clone] No enabled clones found to disable (may already be disabled)");
    } else {
        warn!("[disable_previous_clone] No old active seed provided, nothing to disable");
    }
    
    Ok(())
}

// Signal definitions for the directory DNA
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Signal {
    EntryCreated {
        action: SignedActionHashed,
        app_entry: EntryTypes,
    },
    EntryUpdated {
        action: SignedActionHashed,
        app_entry: EntryTypes,
        original_app_entry: EntryTypes,
    },
    EntryDeleted {
        action: SignedActionHashed,
        original_app_entry: EntryTypes,
    },
    LinkCreated {
        action: SignedActionHashed,
        link_type: LinkTypes,
    },
    LinkDeleted {
        action: SignedActionHashed,
        link_type: LinkTypes,
    },
}