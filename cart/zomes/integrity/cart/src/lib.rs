use hdi::prelude::*;

mod cart;
pub use cart::*;

mod address;
pub use address::*;

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    #[entry_type(visibility = "public")]
    CheckedOutCart(CheckedOutCart),
    #[entry_type(visibility = "private")]
    PrivateCart(PrivateCart),
    #[entry_type(visibility = "private")]
    Address(Address),
}

#[hdk_link_types]
pub enum LinkTypes {
    AgentToProduct,
    AgentToCheckedOutCart,
    AgentToAddress,
    AgentToPrivateCart,
    OrderToPrivateAddress,
}