#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde; //use cw_multi_test::Contract;
macros::cosmwasm_imports!(ensure, Addr, Coin, Deps, Env, StdError, StdResult, Timestamp);
#[cfg(feature = "cosmwasm")]
use cw_storage_plus::Item;
#[cfg(feature = "secretwasm")]
use secret_toolkit::{serialization::Json, storage::Item};

#[cfg(feature = "secretwasm")]
pub const STATE: Item<State, Json> = Item::new(b"state");
#[cfg(feature = "cosmwasm")]
pub const STATE: Item<State> = Item::new("state");

#[cfg(feature = "cosmwasm")]
#[cfg_attr(feature = "cosmwasm", cw_serde)]
pub struct State {
    pub asset_unifier_contract: String,
}

#[cfg(feature = "secretwasm")]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
pub struct State {
    pub asset_unifier_contract: String,
    pub asset_unifier_code_hash: String,
}
