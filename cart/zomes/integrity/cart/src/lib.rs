use hdi::prelude::*;

mod cart;
pub use cart::*;

mod address;
pub use address::*;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    CartProduct(CartProduct),
    SessionStatus(SessionStatus),
    Address(Address),
    DeliveryTimeSlot(DeliveryTimeSlot),
    DeliveryInstructions(DeliveryInstructions),
}

#[derive(Serialize, Deserialize)]
#[hdk_link_types]
pub enum LinkTypes {
    // Single LinkType for all public cart data - everything uses same path
    PublicPathToCartData,
}

// Genesis validation
#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

// Agent joining validation
pub fn validate_agent_joining(
    _agent_pub_key: AgentPubKey,
    _membrane_proof: &Option<MembraneProof>,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

// Main validation callback
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { .. } => Ok(ValidateCallbackResult::Valid),
            OpEntry::UpdateEntry { .. } => Ok(ValidateCallbackResult::Valid),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(_) => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterDelete(_) => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterCreateLink { link_type, .. } => match link_type {
            LinkTypes::PublicPathToCartData => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDeleteLink { link_type, .. } => match link_type {
            LinkTypes::PublicPathToCartData => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::StoreRecord(_) => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterAgentActivity(agent_activity) => match agent_activity {
            OpActivity::CreateAgent { agent, action } => {
                let previous_action = must_get_action(action.prev_action)?;
                match previous_action.action() {
                    Action::AgentValidationPkg(AgentValidationPkg { membrane_proof, .. }) => {
                        validate_agent_joining(agent, membrane_proof)
                    }
                    _ => Ok(ValidateCallbackResult::Invalid(
                        "The previous action for a `CreateAgent` action must be an `AgentValidationPkg`"
                            .to_string(),
                    )),
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
    }
}