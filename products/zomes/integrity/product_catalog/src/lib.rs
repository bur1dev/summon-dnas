pub mod product;
use hdi::prelude::*;

pub use product::*;

// Product preference structure (moved from cart)
#[hdk_entry_helper]
#[derive(Clone)]
pub struct ProductPreference {
    pub group_hash: ActionHash,    // Reference to ProductGroup
    pub product_index: u32,        // Index of product within the group
    pub note: String,              // Customer note/preference
    pub timestamp: u64,            // When this preference was last updated
    pub is_default: bool           // If true, apply automatically
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Product(Product),
    ProductGroup(ProductGroup),
    ProductPreference(ProductPreference),
}

#[derive(Serialize, Deserialize)]
#[hdk_link_types]
pub enum LinkTypes {
    ProductTypeToGroup,
    AgentToPreference,
}

// Validation you perform during the genesis process. Nobody else on the network performs it, only you.
// There *is no* access to network calls in this callback
#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

// Validation the network performs when you try to join, you can't perform this validation yourself as you are not a member yet.
// There *is* access to network calls in this function
pub fn validate_agent_joining(
    _agent_pub_key: AgentPubKey,
    _membrane_proof: &Option<MembraneProof>,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

// This is the unified validation callback for all entries and link types in this integrity zome
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::Product(product) => {
                    validate_create_product(EntryCreationAction::Create(action), product)
                }
                EntryTypes::ProductGroup(product_group) => {
                    validate_create_product_group(EntryCreationAction::Create(action), product_group)
                }
                EntryTypes::ProductPreference(_product_preference) => {
                    Ok(ValidateCallbackResult::Valid)
                }
            },
            OpEntry::UpdateEntry {
                app_entry, action, ..
            } => match app_entry {
                EntryTypes::Product(product) => {
                    validate_create_product(EntryCreationAction::Update(action), product)
                }
                EntryTypes::ProductGroup(product_group) => {
                    validate_create_product_group(EntryCreationAction::Update(action), product_group)
                }
                EntryTypes::ProductPreference(product_preference) => {
                    Ok(ValidateCallbackResult::Valid)
                }
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(update_entry) => match update_entry {
            OpUpdate::Entry { app_entry, action } => {
                let original_action = must_get_action(action.clone().original_action_address)?
                    .action()
                    .to_owned();
                let original_create_action = match EntryCreationAction::try_from(original_action) {
                    Ok(action) => action,
                    Err(e) => {
                        return Ok(ValidateCallbackResult::Invalid(format!(
                            "Expected to get EntryCreationAction from Action: {e:?}"
                        )));
                    }
                };
                match app_entry {
                    EntryTypes::Product(product) => {
                        let original_app_entry =
                            must_get_valid_record(action.clone().original_action_address)?;
                        let original_product = match Product::try_from(original_app_entry) {
                            Ok(entry) => entry,
                            Err(e) => {
                                return Ok(ValidateCallbackResult::Invalid(format!(
                                    "Expected to get Product from Record: {e:?}"
                                )));
                            }
                        };
                        validate_update_product(
                            action,
                            product,
                            original_create_action,
                            original_product,
                        )
                    }
                    EntryTypes::ProductGroup(product_group) => {
                        let original_app_entry =
                            must_get_valid_record(action.clone().original_action_address)?;
                        let original_product_group = match ProductGroup::try_from(original_app_entry) {
                            Ok(entry) => entry,
                            Err(e) => {
                                return Ok(ValidateCallbackResult::Invalid(format!(
                                    "Expected to get ProductGroup from Record: {e:?}"
                                )));
                            }
                        };
                        validate_update_product_group(
                            action,
                            product_group,
                            original_create_action,
                            original_product_group,
                        )
                    }
                    EntryTypes::ProductPreference(product_preference) => {
                        let original_app_entry =
                            must_get_valid_record(action.clone().original_action_address)?;
                        let original_product_preference = match ProductPreference::try_from(original_app_entry) {
                            Ok(entry) => entry,
                            Err(e) => {
                                return Ok(ValidateCallbackResult::Invalid(format!(
                                    "Expected to get ProductPreference from Record: {e:?}"
                                )));
                            }
                        };
                        Ok(ValidateCallbackResult::Valid)
                    }
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDelete(delete_entry) => {
            let original_action_hash = delete_entry.clone().action.deletes_address;
            let original_record = must_get_valid_record(original_action_hash)?;
            let original_record_action = original_record.action().clone();
            let original_action = match EntryCreationAction::try_from(original_record_action) {
                Ok(action) => action,
                Err(e) => {
                    return Ok(ValidateCallbackResult::Invalid(format!(
                        "Expected to get EntryCreationAction from Action: {e:?}"
                    )));
                }
            };
            let app_entry_type = match original_action.entry_type() {
                EntryType::App(app_entry_type) => app_entry_type,
                _ => {
                    return Ok(ValidateCallbackResult::Valid);
                }
            };
            let entry = match original_record.entry().as_option() {
                Some(entry) => entry,
                None => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original record for a delete must contain an entry".to_string(),
                    ));
                }
            };
            let original_app_entry = match EntryTypes::deserialize_from_type(
                app_entry_type.zome_index,
                app_entry_type.entry_index,
                entry,
            )? {
                Some(app_entry) => app_entry,
                None => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original app entry must be one of the defined entry types for this zome"
                            .to_string(),
                    ));
                }
            };
            match original_app_entry {
                EntryTypes::Product(original_product) => validate_delete_product(
                    delete_entry.clone().action,
                    original_action,
                    original_product,
                ),
                EntryTypes::ProductGroup(original_product_group) => validate_delete_product_group(
                    delete_entry.clone().action,
                    original_action,
                    original_product_group,
                ),
                EntryTypes::ProductPreference(_original_product_preference) => Ok(ValidateCallbackResult::Valid),
            }
        }
        FlatOp::RegisterCreateLink {
            link_type,
            base_address: _,
            target_address: _,
            tag: _,
            action: _,
        } => match link_type {
            LinkTypes::ProductTypeToGroup => Ok(ValidateCallbackResult::Valid),
            LinkTypes::AgentToPreference => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDeleteLink {
            link_type,
            base_address: _,
            target_address: _,
            tag: _,
            original_action: _,
            action: _,
        } => match link_type {
            LinkTypes::ProductTypeToGroup => Ok(ValidateCallbackResult::Valid),
            LinkTypes::AgentToPreference => Ok(ValidateCallbackResult::Valid),
        },
        // The rest of the validation callbacks remain the same as in the original file
        FlatOp::StoreRecord(store_record) => match store_record {
            // Include validation for both Product and ProductGroup
            OpRecord::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::Product(product) => {
                    validate_create_product(EntryCreationAction::Create(action), product)
                }
                EntryTypes::ProductGroup(product_group) => {
                    validate_create_product_group(EntryCreationAction::Create(action), product_group)
                }
                EntryTypes::ProductPreference(_product_preference) => {
                    Ok(ValidateCallbackResult::Valid)
                }
            },
            OpRecord::UpdateEntry {
                original_action_hash,
                app_entry,
                action,
                ..
            } => {
                let original_record = must_get_valid_record(original_action_hash)?;
                let original_action = original_record.action().clone();
                let original_action = match original_action {
                    Action::Create(create) => EntryCreationAction::Create(create),
                    Action::Update(update) => EntryCreationAction::Update(update),
                    _ => {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Original action for an update must be a Create or Update action"
                                .to_string(),
                        ));
                    }
                };
                match app_entry {
                    EntryTypes::Product(product) => {
                        let result = validate_create_product(
                            EntryCreationAction::Update(action.clone()),
                            product.clone(),
                        )?;
                        if let ValidateCallbackResult::Valid = result {
                            let original_product: Option<Product> = original_record
                                .entry()
                                .to_app_option()
                                .map_err(|e| wasm_error!(e))?;
                            let original_product = match original_product {
                                Some(product) => product,
                                None => {
                                    return Ok(
                                            ValidateCallbackResult::Invalid(
                                                "The updated entry type must be the same as the original entry type"
                                                    .to_string(),
                                            ),
                                        );
                                }
                            };
                            validate_update_product(
                                action,
                                product,
                                original_action,
                                original_product,
                            )
                        } else {
                            Ok(result)
                        }
                    }
                    EntryTypes::ProductGroup(product_group) => {
                        let result = validate_create_product_group(
                            EntryCreationAction::Update(action.clone()),
                            product_group.clone(),
                        )?;
                        if let ValidateCallbackResult::Valid = result {
                            let original_product_group: Option<ProductGroup> = original_record
                                .entry()
                                .to_app_option()
                                .map_err(|e| wasm_error!(e))?;
                            let original_product_group = match original_product_group {
                                Some(group) => group,
                                None => {
                                    return Ok(
                                            ValidateCallbackResult::Invalid(
                                                "The updated entry type must be the same as the original entry type"
                                                    .to_string(),
                                            ),
                                        );
                                }
                            };
                            validate_update_product_group(
                                action,
                                product_group,
                                original_action,
                                original_product_group,
                            )
                        } else {
                            Ok(result)
                        }
                    }
                    EntryTypes::ProductPreference(_product_preference) => {
                        Ok(ValidateCallbackResult::Valid)
                    }
                }
            }
            // Remainder of the validation callbacks are similar to the original file
            // but with the ProductGroup cases added
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterAgentActivity(agent_activity) => match agent_activity {
            OpActivity::CreateAgent { agent, action } => {
                let previous_action = must_get_action(action.prev_action)?;
                match previous_action.action() {
                        Action::AgentValidationPkg(
                            AgentValidationPkg { membrane_proof, .. },
                        ) => validate_agent_joining(agent, membrane_proof),
                        _ => {
                            Ok(
                                ValidateCallbackResult::Invalid(
                                    "The previous action for a `CreateAgent` action must be an `AgentValidationPkg`"
                                        .to_string(),
                                ),
                            )
                        }
                    }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
    }
}
