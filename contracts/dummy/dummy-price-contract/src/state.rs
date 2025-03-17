#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(Uint128);

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
pub struct State {
    pub token0: String,
    pub token1: String,
    pub price: Uint128,
}

#[cfg(feature = "cosmwasm")]
pub const STATE: Item<State> = Item::new("state");
#[cfg(feature = "secretwasm")]
pub const STATE: Item<State, Json> = Item::new(b"state");
