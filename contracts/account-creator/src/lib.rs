#[cfg(test)]
pub mod colors;
pub mod contract;
pub mod sudo;
#[cfg(test)]
#[cfg(feature = "cosmwasm")]
pub mod tests_helpers;
#[cfg(test)]
#[cfg(feature = "cosmwasm")]
pub mod tests_integration;
#[cfg(test)]
#[cfg(feature = "cosmwasm")]
pub mod tests_pair_registry;
#[cfg(test)]
#[cfg(feature = "cosmwasm")]
pub mod tests_setup;

#[cfg(feature = "cosmwasm")]
use cw_storage_plus::Item;
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
use secret_toolkit::{serialization::Json, storage::Item};

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub mod parse_reply_instantiate;

use classes::account_creator::{Addresses, Config};

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const CONFIG: Item<Config, Json> = Item::new(b"config");
#[cfg(feature = "cosmwasm")]
pub const CONFIG: Item<Config> = Item::new("config");

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub const ADDRESSES: Item<Addresses, Json> = Item::new(b"addresses");
#[cfg(feature = "cosmwasm")]
pub const ADDRESSES: Item<Addresses> = Item::new("addresses");
