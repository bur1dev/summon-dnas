use preferences_integrity::*;
use hdk::prelude::*;

mod preferences;

// Save or update preference for a UPC (handles both create and update)
#[hdk_extern]
pub fn save_preference(input: preferences::SavePreferenceInput) -> ExternResult<ActionHash> {
    preferences::save_preference_impl(input)
}

// Get preference by UPC  
#[hdk_extern]
pub fn get_preference(input: preferences::GetPreferenceInput) -> ExternResult<Option<UpcPreference>> {
    preferences::get_preference_impl(input)
}

// Delete preference by UPC
#[hdk_extern]
pub fn delete_preference(input: preferences::DeletePreferenceInput) -> ExternResult<()> {
    preferences::delete_preference_impl(input)
}