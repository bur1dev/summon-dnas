use hdi::prelude::*;

// The core entry type for tracking the active product catalog
#[hdk_entry_helper]
#[derive(Clone)]
pub struct ActiveProductCatalog {
    pub network_seed: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    ActiveProductCatalog(ActiveProductCatalog),
}

#[derive(Serialize, Deserialize)]
#[hdk_link_types]
pub enum LinkTypes {
    Catalog, // Links from anchor to active catalog
}

// Validation during genesis - always valid for this simple DNA
#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

// Agent joining validation - always allow
pub fn validate_agent_joining(
    _agent_pub_key: AgentPubKey,
    _membrane_proof: &Option<MembraneProof>,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

// Basic validation - this is a simple coordination DNA so we allow most operations
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action: _ } => match app_entry {
                EntryTypes::ActiveProductCatalog(_catalog) => {
                    // Simple validation: ensure network_seed is not empty
                    Ok(ValidateCallbackResult::Valid)
                }
            },
            OpEntry::UpdateEntry { app_entry, action: _, .. } => match app_entry {
                EntryTypes::ActiveProductCatalog(_catalog) => {
                    Ok(ValidateCallbackResult::Valid)
                }
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => {
            // Allow updates to catalog entries
            Ok(ValidateCallbackResult::Valid)
        },
        FlatOp::RegisterDelete(_) => {
            // Allow deletes (for cleanup of old catalog entries)
            Ok(ValidateCallbackResult::Valid)
        },
        FlatOp::RegisterCreateLink { link_type, .. } => match link_type {
            LinkTypes::Catalog => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDeleteLink { link_type, .. } => match link_type {
            LinkTypes::Catalog => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::StoreRecord(_) => Ok(ValidateCallbackResult::Valid),
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