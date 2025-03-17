use classes::user_state::AbstractionRule;
#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
#[cfg(feature = "cosmwasm")]
use cw_storage_plus::{Item, Map};
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
use secret_toolkit::{
    serialization::Json,
    storage::{Item, Keymap, KeymapBuilder},
};
macros::cosmwasm_imports!(ensure, Addr, Coin, StdError, Uint128);

#[cfg(feature = "cosmwasm")]
pub const ABSTRACTION_RULES: Map<u16, AbstractionRule> = Map::new("rules");
// pub static is stylistically better, but breaks something
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[allow(clippy::declare_interior_mutable_const)]
pub const ABSTRACTION_RULES: Keymap<u16, AbstractionRule, Json> =
    KeymapBuilder::new(b"rules").build();

#[cfg(feature = "cosmwasm")]
pub const COUNTER: Item<u16> = Item::new("counter");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const COUNTER: Item<u16> = Item::new(b"counter");

#[cfg(feature = "cosmwasm")]
pub const ETH_INTERPRETER_ADDRESS: Item<String> = Item::new("eth_interpreter_address");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const ETH_INTERPRETER_ADDRESS: Item<String> = Item::new(b"eth_interpreter_address");

#[cfg(feature = "cosmwasm")]
pub const ETH_INTERPRETER_CODE_HASH: Item<String> = Item::new("eth_interpreter_code_hash");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const ETH_INTERPRETER_CODE_HASH: Item<String> = Item::new(b"eth_interpreter_code_hash");

#[cfg(feature = "cosmwasm")]
pub const LAST_ACTIVITY: Item<u64> = Item::new("last_activity");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const LAST_ACTIVITY: Item<u64> = Item::new(b"last_activity");

#[cfg(feature = "cosmwasm")]
pub const USER_ENTRY: Item<String> = Item::new("user_entry");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const USER_ENTRY: Item<String> = Item::new(b"user_entry");
