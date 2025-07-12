use cart_integrity::*;
use hdk::prelude::*;
use serde::{Deserialize, Serialize};

// Helper struct that includes both the cart product and its hash for removal
// Now includes quantity and timestamp from link tags
#[derive(Serialize, Deserialize, Debug)]
pub struct CartProductWithHash {
    #[serde(flatten)]
    pub product: CartProduct,
    pub action_hash: ActionHash,
    pub quantity: f64,     // Read from link tag
    pub timestamp: u64,    // Read from link tag
}

// Complete cart session data structure
#[derive(Serialize, Deserialize, Debug)]
pub struct CartSessionData {
    pub cart_products: Vec<CartProductWithHash>,
    pub session_status: Option<Record>,
    pub address: Option<Record>,
    pub delivery_time_slot: Option<Record>,
    pub delivery_instructions: Option<Record>,
}

// Helper function to get PUBLIC path that all agents can see
fn get_public_cart_path() -> ExternResult<Path> {
    let path_str = "active_carts";
    Path::try_from(path_str).map_err(|_| wasm_error!(WasmErrorInner::Guest("Failed to create public path".to_string())))
}

// Add individual cart item - OPTIMIZED: create entry only if needed, update quantity via link tags
pub(crate) fn add_item_impl(item: CartProduct, quantity: f64) -> ExternResult<ActionHash> {
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    let timestamp = sys_time()?.as_micros() as u64;
    
    // Check if this product already exists in the cart
    let existing_entry = find_existing_cart_product(&item.product_id)?;
    
    let cart_product_hash = if let Some((existing_hash, current_quantity, _)) = existing_entry {
        // Product exists - delete old link and create new link with updated quantity
        delete_quantity_link(&public_hash, &existing_hash)?;
        
        let new_quantity = current_quantity + quantity;
        let quantity_tag = CartQuantityTag {
            quantity: new_quantity,
            timestamp,
        };
        
        create_link(
            public_hash,
            existing_hash.clone(),
            LinkTypes::PublicPathToCartData,
            quantity_tag.to_link_tag()
        )?;
        
        existing_hash
    } else {
        // Product doesn't exist - create new entry and link with quantity tag
        let cart_product_hash = create_entry(EntryTypes::CartProduct(item))?;
        
        let quantity_tag = CartQuantityTag {
            quantity,
            timestamp,
        };
        
        create_link(
            public_hash,
            cart_product_hash.clone(),
            LinkTypes::PublicPathToCartData,
            quantity_tag.to_link_tag()
        )?;
        
        cart_product_hash
    };
    
    Ok(cart_product_hash)
}

// Helper function to find existing cart product by product_id
fn find_existing_cart_product(product_id: &str) -> ExternResult<Option<(ActionHash, f64, u64)>> {
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    
    let links = get_links(
        GetLinksInputBuilder::try_new(public_hash, LinkTypes::PublicPathToCartData)?.build()
    )?;
    
    for link in links {
        if let Some(target_hash) = link.target.into_action_hash() {
            if let Some(record) = get(target_hash.clone(), GetOptions::default())? {
                if let Ok(cart_product) = CartProduct::try_from(record) {
                    if cart_product.product_id == product_id {
                        let (quantity, timestamp) = CartQuantityTag::from_link_tag(&link.tag);
                        return Ok(Some((target_hash, quantity, timestamp)));
                    }
                }
            }
        }
    }
    
    Ok(None)
}

// Helper function to delete existing quantity link for a cart product
fn delete_quantity_link(public_hash: &EntryHash, target_hash: &ActionHash) -> ExternResult<()> {
    let links = get_links(
        GetLinksInputBuilder::try_new(public_hash.clone(), LinkTypes::PublicPathToCartData)?.build()
    )?;
    
    for link in links {
        if let Some(link_target_hash) = link.target.into_action_hash() {
            if link_target_hash == *target_hash {
                delete_link(link.create_link_hash)?;
                break;
            }
        }
    }
    
    Ok(())
}

// Remove cart item quantity - OPTIMIZED: reduce quantity via link tags, delete link if quantity reaches zero
pub(crate) fn remove_item_impl(product_id: String, quantity_to_remove: f64) -> ExternResult<ActionHash> {
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    let timestamp = sys_time()?.as_micros() as u64;
    
    // Find the existing cart product
    if let Some((existing_hash, current_quantity, _)) = find_existing_cart_product(&product_id)? {
        // Delete the old link
        delete_quantity_link(&public_hash, &existing_hash)?;
        
        let new_quantity = current_quantity - quantity_to_remove;
        
        if new_quantity > 0.0 {
            // Create new link with reduced quantity
            let quantity_tag = CartQuantityTag {
                quantity: new_quantity,
                timestamp,
            };
            
            create_link(
                public_hash,
                existing_hash.clone(),
                LinkTypes::PublicPathToCartData,
                quantity_tag.to_link_tag()
            )?;
        }
        // If new_quantity <= 0, we don't create a new link (item removed from cart)
        
        Ok(existing_hash)
    } else {
        Err(wasm_error!(WasmErrorInner::Guest("Cart item not found".to_string())))
    }
}


// Get all current cart items using PUBLIC path - OPTIMIZED: reads quantities from link tags
pub(crate) fn get_current_items_impl() -> ExternResult<Vec<CartProductWithHash>> {
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    
    let links = get_links(
        GetLinksInputBuilder::try_new(public_hash, LinkTypes::PublicPathToCartData)?.build()
    )?;
    
    let mut cart_items = Vec::new();
    for link in links {
        if let Some(target_hash) = link.target.into_action_hash() {
            if let Some(record) = get(target_hash.clone(), GetOptions::default())? {
                if let Ok(cart_product) = CartProduct::try_from(record) {
                    // Read quantity and timestamp from link tag
                    let (quantity, timestamp) = CartQuantityTag::from_link_tag(&link.tag);
                    
                    // Only include items with positive quantity (skip orphaned entries)
                    if quantity > 0.0 {
                        cart_items.push(CartProductWithHash {
                            product: cart_product,
                            action_hash: target_hash,
                            quantity,
                            timestamp,
                        });
                    }
                }
            }
        }
    }
    
    Ok(cart_items)
}

// Get session status using PUBLIC path - ALL session statuses are public
pub(crate) fn get_session_status_impl() -> ExternResult<Option<Record>> {
    warn!("🔎 GET SESSION STATUS: Looking for SessionStatus entries");
    
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    
    let links = get_links(
        GetLinksInputBuilder::try_new(public_hash, LinkTypes::PublicPathToCartData)?.build()
    )?;
    
    // Find the SessionStatus record specifically
    for link in links {
        if let Some(target_hash) = link.target.into_action_hash() {
            if let Some(record) = get(target_hash, GetOptions::default())? {
                // Only return if this is actually a SessionStatus record
                if SessionStatus::try_from(record.clone()).is_ok() {
                    warn!("✅ GET SESSION STATUS: Found SessionStatus record");
                    return Ok(Some(record));
                }
            }
        }
    }
    
    warn!("❌ GET SESSION STATUS: No SessionStatus found");
    Ok(None)
}

// Update session status to "Checkout" using PUBLIC path - ALL status changes are public
pub(crate) fn publish_order_impl() -> ExternResult<ActionHash> {
    warn!("🚀 PUBLISH ORDER: Starting publish_order_impl");
    
    let current_time = sys_time()?.as_micros() as u64;
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    
    let new_status = SessionStatus {
        status: "Checkout".to_string(),
        last_updated: current_time,
    };
    
    warn!("📝 PUBLISH ORDER: Creating SessionStatus with status: Checkout, timestamp: {}", current_time);
    
    if let Some(status_record) = get_session_status_impl()? {
        let new_hash = update_entry(status_record.action_address().clone(), new_status)?;
        
        // Find and delete only the link pointing to the old SessionStatus
        let links = get_links(
            GetLinksInputBuilder::try_new(public_hash.clone(), LinkTypes::PublicPathToCartData)?.build()
        )?;
        for link in links {
            if let Some(target_hash) = link.target.clone().into_action_hash() {
                if target_hash == *status_record.action_address() {
                    delete_link(link.create_link_hash)?;
                    break;
                }
            }
        }
        create_link(public_hash, new_hash.clone(), LinkTypes::PublicPathToCartData, ())?;
        
        warn!("✅ PUBLISH ORDER: SessionStatus updated with hash: {:?}", new_hash);
        Ok(new_hash)
    } else {
        let status_hash = create_entry(EntryTypes::SessionStatus(new_status))?;
        
        create_link(
            public_hash,
            status_hash.clone(),
            LinkTypes::PublicPathToCartData,
            ()
        )?;
        
        warn!("✅ PUBLISH ORDER: SessionStatus created with hash: {:?}", status_hash);
        Ok(status_hash)
    }
}

// Update session status back to "Shopping" using PUBLIC path - ALL status changes are public
pub(crate) fn recall_order_impl() -> ExternResult<ActionHash> {
    let current_time = sys_time()?.as_micros() as u64;
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    
    let new_status = SessionStatus {
        status: "Shopping".to_string(),
        last_updated: current_time,
    };
    
    if let Some(status_record) = get_session_status_impl()? {
        let new_hash = update_entry(status_record.action_address().clone(), new_status)?;
        
        // Find and delete only the link pointing to the old SessionStatus
        let links = get_links(
            GetLinksInputBuilder::try_new(public_hash.clone(), LinkTypes::PublicPathToCartData)?.build()
        )?;
        for link in links {
            if let Some(target_hash) = link.target.clone().into_action_hash() {
                if target_hash == *status_record.action_address() {
                    delete_link(link.create_link_hash)?;
                    break;
                }
            }
        }
        create_link(public_hash, new_hash.clone(), LinkTypes::PublicPathToCartData, ())?;
        
        warn!("✅ PUBLISH ORDER: SessionStatus updated with hash: {:?}", new_hash);
        Ok(new_hash)
    } else {
        let status_hash = create_entry(EntryTypes::SessionStatus(new_status))?;
        
        create_link(
            public_hash,
            status_hash.clone(),
            LinkTypes::PublicPathToCartData,
            ()
        )?;
        
        Ok(status_hash)
    }
}

// Set delivery address for first time - create_entry + create_link to PUBLIC path
pub(crate) fn set_delivery_address_impl(address: Address) -> ExternResult<ActionHash> {
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    
    warn!("🛒 CART DNA: Creating PUBLIC address entry for cart session: {} {}, {}", 
           address.street, address.city, address.state);
    
    // Create the Address entry
    let address_hash = create_entry(EntryTypes::Address(address))?;
    
    warn!("✅ CART DNA: Public address created with hash: {:?}", address_hash);
    
    // Link it to the PUBLIC path so all agents can see it
    create_link(
        public_hash,
        address_hash.clone(),
        LinkTypes::PublicPathToCartData,
        ()
    )?;
    
    Ok(address_hash)
}

// Update delivery address - delete old link, create new entry, create new link
pub(crate) fn update_delivery_address_impl(previous_address_hash: ActionHash, new_address: Address) -> ExternResult<ActionHash> {
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    
    warn!("🔄 CART DNA: Updating PUBLIC address from {:?} to: {} {}, {}", 
           previous_address_hash, new_address.street, new_address.city, new_address.state);
    
    // Find and delete the link pointing to the previous address
    let links = get_links(
        GetLinksInputBuilder::try_new(public_hash.clone(), LinkTypes::PublicPathToCartData)?.build()
    )?;
    
    for link in links {
        if let Some(target_hash) = link.target.clone().into_action_hash() {
            if target_hash == previous_address_hash {
                delete_link(link.create_link_hash)?;
                break;
            }
        }
    }
    
    // Create new address entry
    let new_address_hash = create_entry(EntryTypes::Address(new_address))?;
    
    warn!("✅ CART DNA: Updated public address created with hash: {:?}", new_address_hash);
    
    // Link new address to PUBLIC path
    create_link(
        public_hash,
        new_address_hash.clone(),
        LinkTypes::PublicPathToCartData,
        ()
    )?;
    
    Ok(new_address_hash)
}

// Set delivery time slot - create_entry + create_link to PUBLIC path
pub(crate) fn set_delivery_time_slot_impl(time_slot: DeliveryTimeSlot) -> ExternResult<ActionHash> {
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    
    warn!("🛒 CART DNA: Creating PUBLIC delivery time slot entry: {} at {}", 
           time_slot.time_slot, time_slot.date);
    
    // Check if time slot already exists and delete old link
    let links = get_links(
        GetLinksInputBuilder::try_new(public_hash.clone(), LinkTypes::PublicPathToCartData)?.build()
    )?;
    for link in links {
        if let Some(target_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(target_hash, GetOptions::default())? {
                if DeliveryTimeSlot::try_from(record).is_ok() {
                    delete_link(link.create_link_hash)?;
                    break;
                }
            }
        }
    }
    
    // Create the DeliveryTimeSlot entry
    let time_slot_hash = create_entry(EntryTypes::DeliveryTimeSlot(time_slot))?;
    
    warn!("✅ CART DNA: Public delivery time slot created with hash: {:?}", time_slot_hash);
    
    // Link it to the PUBLIC path so all agents can see it
    create_link(
        public_hash,
        time_slot_hash.clone(),
        LinkTypes::PublicPathToCartData,
        ()
    )?;
    
    Ok(time_slot_hash)
}

// Set delivery instructions - create_entry + create_link to PUBLIC path
pub(crate) fn set_delivery_instructions_impl(instructions: DeliveryInstructions) -> ExternResult<ActionHash> {
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    
    warn!("🛒 CART DNA: Creating PUBLIC delivery instructions entry: {}", 
           instructions.instructions);
    
    // Check if instructions already exist and delete old link
    let links = get_links(
        GetLinksInputBuilder::try_new(public_hash.clone(), LinkTypes::PublicPathToCartData)?.build()
    )?;
    for link in links {
        if let Some(target_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(target_hash, GetOptions::default())? {
                if DeliveryInstructions::try_from(record).is_ok() {
                    delete_link(link.create_link_hash)?;
                    break;
                }
            }
        }
    }
    
    // Create the DeliveryInstructions entry
    let instructions_hash = create_entry(EntryTypes::DeliveryInstructions(instructions))?;
    
    warn!("✅ CART DNA: Public delivery instructions created with hash: {:?}", instructions_hash);
    
    // Link it to the PUBLIC path so all agents can see it
    create_link(
        public_hash,
        instructions_hash.clone(),
        LinkTypes::PublicPathToCartData,
        ()
    )?;
    
    Ok(instructions_hash)
}

// Consolidated function to get all session data in one call using PUBLIC path - ALL DATA IS PUBLIC
pub(crate) fn get_session_data_impl() -> ExternResult<CartSessionData> {
    warn!("🔍 GET SESSION DATA: Starting get_session_data_impl");
    
    let public_path = get_public_cart_path()?;
    let public_hash = public_path.path_entry_hash()?;
    
    // Get all links from PUBLIC path - ALL ENTRIES ARE PUBLIC
    let all_links = get_links(
        GetLinksInputBuilder::try_new(public_hash, LinkTypes::PublicPathToCartData)?.build()
    )?;
    
    // Process all links and categorize by entry type
    let mut cart_products = Vec::new();
    let mut session_status = None;
    let mut address = None;
    let mut delivery_time_slot = None;
    let mut delivery_instructions = None;
    
    for link in all_links {
        if let Some(target_hash) = link.target.into_action_hash() {
            if let Some(record) = get(target_hash.clone(), GetOptions::default())? {
                // Try to parse based on entry content using .is_ok() to avoid crashes
                if let Ok(cart_product) = CartProduct::try_from(record.clone()) {
                    // Read quantity and timestamp from link tag
                    let (quantity, timestamp) = CartQuantityTag::from_link_tag(&link.tag);
                    
                    // Only include items with positive quantity (skip orphaned entries)
                    if quantity > 0.0 {
                        cart_products.push(CartProductWithHash {
                            product: cart_product,
                            action_hash: target_hash,
                            quantity,
                            timestamp,
                        });
                    }
                } else if SessionStatus::try_from(record.clone()).is_ok() {
                    session_status = Some(record);
                } else if Address::try_from(record.clone()).is_ok() {
                    address = Some(record);
                } else if DeliveryTimeSlot::try_from(record.clone()).is_ok() {
                    delivery_time_slot = Some(record);
                } else if DeliveryInstructions::try_from(record.clone()).is_ok() {
                    delivery_instructions = Some(record);
                }
            }
        }
    }
    
    warn!("📊 GET SESSION DATA: Found {} cart_products, session_status: {:?}", 
          cart_products.len(), 
          session_status.as_ref().and_then(|r| SessionStatus::try_from(r.clone()).ok()));
    
    Ok(CartSessionData {
        cart_products,
        session_status,
        address,
        delivery_time_slot,
        delivery_instructions,
    })
}

