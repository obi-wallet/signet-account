use common::coin256::Coin256;

use crate::sources::Sources;
#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;

#[cfg(feature = "cosmwasm")]
#[cfg_attr(not(feature = "secretwasm"), cw_serde)]
pub struct InstantiateMsg {
    pub default_asset_unifier: String,
    pub home_network: String,
    pub legacy_owner: Option<String>,
    pub pair_contract_registry: String,
}

#[cfg(feature = "secretwasm")]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
pub struct InstantiateMsg {
    pub default_asset_unifier: String,
    pub home_network: String,
    pub legacy_owner: Option<String>,
    pub pair_contract_registry: String,
    pub pair_contract_registry_code_hash: String,
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
    UpdateLegacyOwner { new_owner: String },
    UpdatePairRegistry { addr: String },
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
    LegacyOwner {},
    /// Get the array of assets denominated in the home asset
    UnifyAssets(UnifyAssetsMsg),
    DefaultUnifiedAsset {},
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
pub struct UnifyAssetsMsg {
    pub target_asset: Option<String>,
    pub assets: Vec<Coin256>,
    pub assets_are_target_amount: bool,
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
pub struct UnifiedAssetsResponse {
    pub asset_unifier: Coin256,
    pub sources: Sources,
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
pub struct LegacyOwnerResponse {
    pub legacy_owner: String,
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
pub struct DefaultUnifiedAssetResponse {
    pub default_asset_unifier: String,
}
