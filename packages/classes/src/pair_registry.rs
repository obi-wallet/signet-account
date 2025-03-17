#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(Binary, Coin, Uint128);
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::pair_contract::PairContract;

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
pub struct InstantiateMsg {
    pub legacy_owner: Option<String>,
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
pub enum ExecuteMsg {
    /// Proposes a new owner for the proxy contract – must be called by the existing owner
    UpdateLegacyOwner { new_owner: String },
    UpsertPair {
        token0: String,
        token1: String,
        routes: Vec<PairContract>,
    },
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
pub enum QueryMsg {
    /// Shows owner; always mutable
    LegacyOwner {},
    SwapRoute {
        token0: String,
        token1: String,
    },
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
pub struct WasmExecuteMsg {
    contract_addr: String,
    /// msg is the json-encoded ExecuteMsg struct (as raw Binary)
    pub msg: Binary,
    funds: Vec<Coin>,
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
pub struct TestExecuteMsg {
    pub foo: String,
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
pub struct TestFieldsExecuteMsg {
    pub recipient: String,
    pub strategy: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct UpdateDelayResponse {
    pub update_delay_hours: u16,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub enum Cw20ExecuteMsg {
    Transfer { recipient: String, amount: Uint128 },
}
