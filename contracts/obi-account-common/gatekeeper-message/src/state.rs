#[cfg(feature = "cosmwasm")]
use cw_storage_plus::Item;
#[cfg(feature = "secretwasm")]
use secret_toolkit::storage::Item;

#[cfg(feature = "cosmwasm")]
pub const ETH_INTERPRETER_ADDRESS: Item<String> = Item::new("eth_interpreter_address");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const ETH_INTERPRETER_ADDRESS: Item<String> = Item::new(b"eth_interpreter_address");

#[cfg(feature = "cosmwasm")]
pub const ETH_INTERPRETER_CODE_HASH: Item<String> = Item::new("eth_interpreter_code_hash");
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const ETH_INTERPRETER_CODE_HASH: Item<String> = Item::new(b"eth_interpreter_code_hash");
