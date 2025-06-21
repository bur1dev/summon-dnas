pub mod product;
pub mod products_by_category;
pub mod search;
pub mod preference;
mod utils;

use hdk::prelude::*;
use products_integrity::*;

#[derive(Serialize, Deserialize, Debug)]
struct DnaProperties {
    admin_pub_key_str: String,
}

#[hdk_extern]
fn is_admin(_: ()) -> ExternResult<bool> {
    let agent_info = agent_info()?;
    let caller_pub_key = agent_info.agent_initial_pubkey;

    let dna_info = dna_info()?;
    let properties_sb = dna_info.modifiers.properties; // This is SerializedBytes
    let properties: DnaProperties = hdk::prelude::decode(properties_sb.bytes())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode DNA properties: {:?}", e))))?;

    let caller_str = caller_pub_key.to_string();
    let admin_str = properties.admin_pub_key_str.clone(); // Clone for logging if needed, or use as is.

    debug!("[is_admin] Caller PubKey: {}", caller_str);
    debug!("[is_admin] Admin PubKey from Props: {}", admin_str);

    let is_match = caller_str == admin_str;
    debug!("[is_admin] Comparison result (is_match): {}", is_match);

    Ok(is_match)
}

// Called the first time a zome call is made to the cell containing this zome
#[hdk_extern]
pub fn init() -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}

// Don't modify this enum if you want the scaffolding tool to generate appropriate signals for your entries and links
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
        create_link_action: SignedActionHashed,
        link_type: LinkTypes,
    },
}

// Whenever an action is committed, we emit a signal to the UI elements to reactively update them
#[hdk_extern(infallible)]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) {
    // Don't modify the for loop if you want the scaffolding tool to generate appropriate signals for your entries and links
    for action in committed_actions {
        if let Err(err) = signal_action(action) {
            error!("Error signaling new action: {:?}", err);
        }
    }
}

// Don't modify this function if you want the scaffolding tool to generate appropriate signals for your entries and links
fn signal_action(action: SignedActionHashed) -> ExternResult<()> {
    match action.hashed.content.clone() {
        Action::Create(_create) => {
            if let Ok(Some(app_entry)) = get_entry_for_action(&action.hashed.hash) {
                emit_signal(Signal::EntryCreated { action, app_entry })?;
            }
            Ok(())
        }
        Action::Update(update) => {
            if let Ok(Some(app_entry)) = get_entry_for_action(&action.hashed.hash) {
                if let Ok(Some(original_app_entry)) =
                    get_entry_for_action(&update.original_action_address)
                {
                    emit_signal(Signal::EntryUpdated {
                        action,
                        app_entry,
                        original_app_entry,
                    })?;
                }
            }
            Ok(())
        }
        Action::Delete(delete) => {
            if let Ok(Some(original_app_entry)) = get_entry_for_action(&delete.deletes_address) {
                emit_signal(Signal::EntryDeleted {
                    action,
                    original_app_entry,
                })?;
            }
            Ok(())
        }
        Action::CreateLink(create_link) => {
            if let Ok(Some(link_type)) =
                LinkTypes::from_type(create_link.zome_index, create_link.link_type)
            {
                emit_signal(Signal::LinkCreated { action, link_type })?;
            }
            Ok(())
        }
        Action::DeleteLink(delete_link) => {
            let record = get(delete_link.link_add_address.clone(), GetOptions::default())?.ok_or(
                wasm_error!(WasmErrorInner::Guest(
                    "Failed to fetch CreateLink action".to_string()
                )),
            )?;
            match record.action() {
                Action::CreateLink(create_link) => {
                    if let Ok(Some(link_type)) =
                        LinkTypes::from_type(create_link.zome_index, create_link.link_type)
                    {
                        emit_signal(Signal::LinkDeleted {
                            action,
                            link_type,
                            create_link_action: record.signed_action.clone(),
                        })?;
                    }
                    Ok(())
                }
                _ => Err(wasm_error!(WasmErrorInner::Guest(
                    "Create Link should exist".to_string()
                ))),
            }
        }
        _ => Ok(()),
    }
}

fn get_entry_for_action(action_hash: &ActionHash) -> ExternResult<Option<EntryTypes>> {
    let record = match get_details(action_hash.clone(), GetOptions::default())? {
        Some(Details::Record(record_details)) => record_details.record,
        _ => return Ok(None),
    };
    let entry = match record.entry().as_option() {
        Some(entry) => entry,
        None => return Ok(None),
    };
    let (zome_index, entry_index) = match record.action().entry_type() {
        Some(EntryType::App(AppEntryDef {
            zome_index,
            entry_index,
            ..
        })) => (zome_index, entry_index),
        _ => return Ok(None),
    };
    EntryTypes::deserialize_from_type(*zome_index, *entry_index, entry)
}

// Preference-related functions
#[derive(Serialize, Deserialize, Debug)]
pub struct GetPreferenceInput {
    pub group_hash: ActionHash,
    pub product_index: u32,
}

#[hdk_extern]
pub fn save_product_preference(preference: ProductPreference) -> ExternResult<ActionHash> {
    preference::save_product_preference_impl(preference)
}

#[hdk_extern]
pub fn get_product_preferences(_: ()) -> ExternResult<Vec<(ActionHash, ProductPreference)>> {
    preference::get_product_preferences_impl()
}

#[hdk_extern]
pub fn get_product_preference_by_product(input: GetPreferenceInput) -> ExternResult<Option<(ActionHash, ProductPreference)>> {
    preference::get_product_preference_by_product_impl(input.group_hash, input.product_index)
}

#[hdk_extern]
pub fn update_product_preference(input: (ActionHash, ProductPreference)) -> ExternResult<ActionHash> {
    preference::update_product_preference_impl(input.0, input.1)
}

#[hdk_extern]
pub fn delete_product_preference(action_hash: ActionHash) -> ExternResult<ActionHash> {
    preference::delete_product_preference_impl(action_hash)
}
