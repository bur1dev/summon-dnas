use cart_integrity::*;
use hdk::prelude::*;

use crate::CheckoutCartInput;
use crate::CheckedOutCartWithHash;
use crate::AddToPrivateCartInput;
use crate::ReplacePrivateCartInput;

// Helper function removed - no longer needed with new CartProduct structure


// Implementation of replace_private_cart - NEW function to replace entire cart with single operation
pub(crate) fn replace_private_cart_impl(input: ReplacePrivateCartInput) -> ExternResult<()> {
    warn!("START replace_private_cart_impl: Replacing cart with {} items, timestamp: {}", 
          input.items.len(), input.last_updated);
    
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    warn!("Agent pubkey: {:?}", agent_pub_key);
    
    // Convert input items directly to CartProduct - no hash decoding needed
    let cart_items: Vec<CartProduct> = input.items.into_iter().map(|item| CartProduct {
        product_id: item.product_id,
        product_name: item.product_name,
        product_image_url: item.product_image_url,
        price_at_checkout: item.price_at_checkout,
        promo_price: item.promo_price,
        quantity: item.quantity,
        timestamp: item.timestamp,
        note: item.note,
    }).collect();
    
    warn!("Converted {} cart items", cart_items.len());
    
    // Create PrivateCart from the converted items
    let cart = PrivateCart {
        items: cart_items,
        last_updated: input.last_updated,
    };
    
    // Create the entry
    match create_entry(EntryTypes::PrivateCart(cart)) {
        Ok(hash) => {
            warn!("SUCCESS: Created new PrivateCart entry with hash: {:?}", hash);
            
            // Get links to existing cart
            let links = match get_links(
                GetLinksInputBuilder::try_new(agent_pub_key.clone(), LinkTypes::AgentToPrivateCart)?.build(),
            ) {
                Ok(links) => {
                    warn!("Found {} existing cart links", links.len());
                    links
                },
                Err(e) => {
                    warn!("ERROR getting links: {:?}", e);
                    return Err(e);
                }
            };
            
            // Delete existing links
            let mut delete_success = 0;
            let mut delete_errors = 0;
            for link in links {
                warn!("Deleting link: {:?}", link.create_link_hash);
                match delete_link(link.create_link_hash.clone()) {
                    Ok(_) => delete_success += 1,
                    Err(e) => {
                        warn!("ERROR deleting link {:?}: {:?}", link.create_link_hash, e);
                        delete_errors += 1;
                    }
                }
            }
            warn!("Deleted {} links successfully, {} failed", delete_success, delete_errors);
            
            // Create new link to the updated cart
            match create_link(
                agent_pub_key,
                hash.clone(),
                LinkTypes::AgentToPrivateCart,
                LinkTag::new(""),
            ) {
                Ok(link_hash) => {
                    warn!("SUCCESS: Created new link with hash: {:?}", link_hash);
                },
                Err(e) => {
                    warn!("ERROR creating link: {:?}", e);
                    return Err(e);
                }
            }
            
            warn!("END replace_private_cart_impl: Successfully replaced cart");
            Ok(())
        },
        Err(e) => {
            warn!("ERROR creating cart entry: {:?}", e);
            Err(e)
        }
    }
}

// Implementation of get_private_cart - retrieves the agent's private cart
pub(crate) fn get_private_cart_impl() -> ExternResult<PrivateCart> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;

    // Get links to private cart from the agent
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key, LinkTypes::AgentToPrivateCart)?.build(),
    )?;

    // If a cart exists, retrieve it
    if let Some(link) = links.first() {
        if let Some(cart_hash) = link.target.clone().into_action_hash() {
            match get(cart_hash.clone(), GetOptions::default())? {
                Some(record) => {
                    let cart: PrivateCart = record
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

                    return Ok(cart);
                }
                None => {
                    // Cart not found, create a new one
                    return Ok(PrivateCart {
                        items: Vec::new(),
                        last_updated: sys_time()?.as_micros() as u64,
                    });
                }
            }
        }
    }

    // No cart found, return empty cart
    Ok(PrivateCart {
        items: Vec::new(),
        last_updated: sys_time()?.as_micros() as u64,
    })
}

// Implementation of add_to_private_cart - adds or updates an item in the private cart
pub(crate) fn add_to_private_cart_impl(input: AddToPrivateCartInput) -> ExternResult<()> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;

    // Get the current private cart
    let mut cart = get_private_cart_impl()?;
    let current_time = sys_time()?.as_micros() as u64;

    // Find if the item already exists in the cart using product_id
    let item_index = cart.items.iter().position(|item|
        item.product_id == input.product_id
    );

    if input.quantity == 0.0 {
        // Remove item if quantity is 0
        if let Some(index) = item_index {
            cart.items.remove(index);
        }
    } else {
        // Update item if it exists or add a new one with full product snapshot
        if let Some(index) = item_index {
            cart.items[index].quantity = input.quantity;
            cart.items[index].timestamp = current_time;
            cart.items[index].note = input.note;
        } else {
            cart.items.push(CartProduct {
                product_id: input.product_id,
                product_name: input.product_name,
                product_image_url: input.product_image_url,
                price_at_checkout: input.price_at_checkout,
                promo_price: input.promo_price,
                quantity: input.quantity,
                timestamp: current_time,
                note: input.note,
            });
        }
    }

    // Update the last_updated timestamp
    cart.last_updated = current_time;

    // Save the updated cart
    let cart_hash = create_entry(EntryTypes::PrivateCart(cart))?;

    // Get links to existing private cart
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key.clone(), LinkTypes::AgentToPrivateCart)?.build(),
    )?;

    // Delete existing links
    for link in links {
        delete_link(link.create_link_hash)?;
    }

    // Create new link to the updated cart
    create_link(
        agent_pub_key,
        cart_hash,
        LinkTypes::AgentToPrivateCart,
        LinkTag::new(""),
    )?;

    Ok(())
}

// Implementation of checkout_cart - creates public order entry with private address link
pub(crate) fn checkout_cart_impl(input: CheckoutCartInput) -> ExternResult<ActionHash> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    let current_time = sys_time()?.as_micros() as u64;

    // Get cart products from input instead of DHT
    let cart_products = input.cart_products.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest("Cart products required".to_string()))
    })?;

    if cart_products.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Cart is empty".to_string()
        )));
    }

    // Create a checked out cart entry (public order) with private address link
    let checked_out_cart = CheckedOutCart {
        id: current_time.to_string(),
        products: cart_products,
        total: 0.0, // Frontend calculates total
        created_at: current_time,
        status: "processing".to_string(), // Standard processing status
        delivery_time: input.delivery_time,
        customer_pub_key: agent_pub_key.clone(), // Keep for future shopper workflow
        general_location: None, // Not needed for customer-only workflow
    };
    warn!("checkout_cart_impl: Creating CheckedOutCart with status: {}", checked_out_cart.status);

    // Create the public order entry
    let cart_hash = create_entry(EntryTypes::CheckedOutCart(checked_out_cart))?;
    warn!("checkout_cart_impl: Created CheckedOutCart entry with hash: {:?}", cart_hash);

    // Link customer to this order
    create_link(
        agent_pub_key.clone(),
        cart_hash.clone(),
        LinkTypes::AgentToCheckedOutCart,
        LinkTag::new("customer"),
    )?;

    // Create private link from order to address (key security feature)
    create_link(
        cart_hash.clone(),
        input.private_address_hash,
        LinkTypes::OrderToPrivateAddress,
        LinkTag::new(""),
    )?;
    warn!("checkout_cart_impl: Created private link from order to address");

    // Clear the private cart after successful checkout
    let empty_cart = PrivateCart {
        items: Vec::new(),
        last_updated: current_time,
    };

    let empty_cart_hash = create_entry(EntryTypes::PrivateCart(empty_cart))?;

    // Delete existing links to private cart
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key.clone(), LinkTypes::AgentToPrivateCart)?.build(),
    )?;

    for link in links {
        delete_link(link.create_link_hash)?;
    }

    // Create new link to the empty cart
    create_link(
        agent_pub_key,
        empty_cart_hash,
        LinkTypes::AgentToPrivateCart,
        LinkTag::new(""),
    )?;

    Ok(cart_hash)
}

// Implementation of get_checked_out_carts - returns only caller's orders
pub(crate) fn get_checked_out_carts_impl() -> ExternResult<Vec<CheckedOutCartWithHash>> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    warn!("get_checked_out_carts_impl: Fetching carts for agent: {:?}", agent_pub_key);

    // Get all links from this agent to checked out carts
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key, LinkTypes::AgentToCheckedOutCart)?.build(),
    )?;
    warn!("get_checked_out_carts_impl: Found {} links to carts.", links.len());

    let mut checked_out_carts = Vec::new();

    for link in links {
        if let Some(cart_hash) = link.target.clone().into_action_hash() {
            warn!("get_checked_out_carts_impl: Processing cart link with target hash: {:?}", cart_hash);
            match get_checked_out_cart_impl(cart_hash.clone())? {
                Some(cart) => {
                    warn!("get_checked_out_carts_impl: Retrieved cart with hash {:?}, status: '{}'", cart_hash, cart.status);
                    // Filter out returned carts
                    if cart.status != "returned" {
                        warn!("get_checked_out_carts_impl: Cart status is NOT 'returned', adding to results.");
                        checked_out_carts.push(CheckedOutCartWithHash {
                            cart_hash,
                            cart,
                        });
                    } else {
                         warn!("get_checked_out_carts_impl: Cart status IS 'returned', filtering out.");
                    }
                }
                None => {
                    warn!("get_checked_out_carts_impl: Could not retrieve cart details for hash: {:?}", cart_hash);
                },
            }
        } else {
            warn!("get_checked_out_carts_impl: Link target is not an ActionHash: {:?}", link.target);
        }
    }

    // Sort by creation time (newest first)
    checked_out_carts.sort_by(|a, b| b.cart.created_at.cmp(&a.cart.created_at));
    warn!("get_checked_out_carts_impl: Returning {} filtered carts.", checked_out_carts.len());

    Ok(checked_out_carts)
}

// Implementation of get_checked_out_cart - used by get_checked_out_carts_impl
pub(crate) fn get_checked_out_cart_impl(
    action_hash: ActionHash,
) -> ExternResult<Option<CheckedOutCart>> {
    match get(action_hash.clone(), GetOptions::default())? {
        Some(record) => {
            let entry = record
                .entry()
                .as_option()
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Expected entry".to_string()
                )))?;

            match entry {
                Entry::App(_) => {
                    let checked_out_cart: CheckedOutCart = record
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

                    Ok(Some(checked_out_cart))
                }
                _ => Err(wasm_error!(WasmErrorInner::Guest(
                    "Expected app entry".to_string()
                ))),
            }
        }
        None => Ok(None),
    }
}

// Implementation of return_to_shopping
pub(crate) fn return_to_shopping_impl(cart_hash: ActionHash) -> ExternResult<()> {
    // Get agent pubkey
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    
    warn!("ENTRY POINT: return_to_shopping_impl with hash: {:?}", cart_hash);
    
    // Get the cart with error handling
    let cart = match get_checked_out_cart_impl(cart_hash.clone()) {
        Ok(Some(cart)) => {
            warn!("SUCCESS: Found cart with status: {}", cart.status);
            cart
        },
        Ok(None) => {
            warn!("ERROR: Cart not found");
            return Err(wasm_error!(WasmErrorInner::Guest("Cart not found".to_string())));
        },
        Err(e) => {
            warn!("ERROR getting cart: {:?}", e);
            return Err(e);
        }
    };
    
    // Update cart status
    let mut updated_cart = cart.clone();
    updated_cart.status = "returned".to_string();
    warn!("UPDATING: Setting status to 'returned'");
    
    // Update entry and get new hash
    let update_hash = match update_entry(cart_hash.clone(), updated_cart) {
        Ok(hash) => {
            warn!("SUCCESS: Updated entry, new hash: {:?}", hash);
            hash
        },
        Err(e) => {
            warn!("ERROR updating entry: {:?}", e);
            return Err(e);
        }
    };
    
    // Find and delete link to old cart hash
    warn!("Getting links from agent to checked out cart");
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key.clone(), LinkTypes::AgentToCheckedOutCart)?.build(),
    )?;
    
    let mut found_link = false;
    for link in links {
        if let Some(target) = link.target.clone().into_action_hash() {
            if target == cart_hash {
                warn!("Deleting old link: {:?}", link.create_link_hash);
                found_link = true;
                delete_link(link.create_link_hash)?;
            }
        }
    }
    
    if !found_link {
        warn!("WARNING: No link found to original cart hash");
    }
    
    // Create new link to updated cart
    warn!("Creating new link to updated cart hash: {:?}", update_hash);
    create_link(
        agent_pub_key,
        update_hash,
        LinkTypes::AgentToCheckedOutCart,
        LinkTag::new("customer"),
    )?;
    
    warn!("Return to shopping completed successfully");
    Ok(())
}

// Implementation of get_address_for_order - customer retrieves their own delivery address
pub(crate) fn get_address_for_order_impl(order_hash: ActionHash) -> ExternResult<Address> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    warn!("get_address_for_order_impl: Customer {:?} retrieving address for order {:?}", agent_pub_key, order_hash);
    
    // First, verify this order belongs to the current customer
    let order = match get_checked_out_cart_impl(order_hash.clone())? {
        Some(order) => order,
        None => return Err(wasm_error!(WasmErrorInner::Guest("Order not found".to_string()))),
    };
    
    if order.customer_pub_key != agent_pub_key {
        return Err(wasm_error!(WasmErrorInner::Guest("Order does not belong to this customer".to_string())));
    }
    
    // Get links from order to private address
    let links = get_links(
        GetLinksInputBuilder::try_new(order_hash.clone(), LinkTypes::OrderToPrivateAddress)?.build(),
    )?;
    
    if links.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest("No address found for this order".to_string())));
    }
    
    // Get the address from the first link
    let address_link = &links[0];
    if let Some(address_hash) = address_link.target.clone().into_action_hash() {
        match get(address_hash, GetOptions::default())? {
            Some(record) => {
                let address: Address = record
                    .entry()
                    .to_app_option()
                    .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to deserialize address: {}", e))))?
                    .ok_or(wasm_error!(WasmErrorInner::Guest("Expected address entry".to_string())))?;
                
                warn!("get_address_for_order_impl: Successfully retrieved address for customer");
                Ok(address)
            }
            None => Err(wasm_error!(WasmErrorInner::Guest("Address record not found".to_string()))),
        }
    } else {
        Err(wasm_error!(WasmErrorInner::Guest("Invalid address hash in link".to_string())))
    }
}
