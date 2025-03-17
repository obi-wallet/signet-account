#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(Uint128);

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
    pub token0: String,
    pub token1: String,
    pub price: Uint128,
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
    // maybe some price update functions added later
}

#[cfg(feature = "cosmwasm")]
#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    Simulation {
        offer_asset: Asset,
    },
    ReverseSimulation {
        ask_asset: Asset,
    },
    Token1ForToken2Price {
        token1_amount: Uint128,
    },
    Token2ForToken1Price {
        token2_amount: Uint128,
    },
    // osmo
    EstimateSinglePoolSwapExactAmountInRequest {
        pool_id: u64,
        token_in: String,
        token_out_denom: String,
    },
}

#[cfg(feature = "secretwasm")]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    Simulation { offer_asset: Asset },
    ReverseSimulation { ask_asset: Asset },
    Token1ForToken2Price { token1_amount: Uint128 },
    Token2ForToken1Price { token2_amount: Uint128 },
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
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
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
pub enum AssetInfo {
    Token { contract_addr: String },
    NativeToken { denom: String },
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
pub struct SimulationResponse {
    pub commission_amount: Uint128,
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
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
pub struct ReverseSimulationResponse {
    pub commission_amount: Uint128,
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
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
pub struct Token1ForToken2Msg {
    pub token1_amount: Uint128,
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
pub struct Token2ForToken1Msg {
    pub token2_amount: Uint128,
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
pub struct Token1ForToken2Response {
    pub token2_amount: Uint128,
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
pub struct Token2ForToken1Response {
    pub token1_amount: Uint128,
}
