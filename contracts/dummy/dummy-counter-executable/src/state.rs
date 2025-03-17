#[cfg(feature = "cosmwasm")]
use cw_storage_plus::Item;
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
use secret_toolkit::{serialization::Json, storage::Item};

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const CHEATER_DETECTED: Item<bool, Json> = Item::new(b"cheater_detected");
#[cfg(feature = "cosmwasm")]
pub const CHEATER_DETECTED: Item<bool> = Item::new("cheater_detected");
