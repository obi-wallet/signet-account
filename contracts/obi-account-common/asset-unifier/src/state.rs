#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
#[cfg(feature = "cosmwasm")]
use cw_storage_plus::Item;
#[cfg(feature = "secretwasm")]
use secret_toolkit::{serialization::Json, storage::Item};

#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
#[cfg_attr(
    feature = "secretwasm",
    derive(
        serde::Serialize,
        serde::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        schemars::JsonSchema
    )
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
#[cfg(feature = "cosmwasm")]
pub struct State {
    pub default_asset_unifier: String,
    pub home_network: String,
    pub pair_contract_registry: String,
}

#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
#[cfg_attr(
    feature = "secretwasm",
    derive(
        serde::Serialize,
        serde::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        schemars::JsonSchema
    )
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
#[cfg(feature = "secretwasm")]
pub struct State {
    pub default_asset_unifier: String,
    pub home_network: String,
    pub pair_contract_registry: String,
    pub pair_contract_registry_code_hash: String,
}

#[cfg(feature = "cosmwasm")]
pub const STATE: Item<State> = Item::new("state");
#[cfg(feature = "secretwasm")]
pub const STATE: Item<State, Json> = Item::new(b"state");
