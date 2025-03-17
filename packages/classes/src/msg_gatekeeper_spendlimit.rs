#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
use cosmwasm_schema::QueryResponses;
macros::cosmwasm_imports!(Binary, Coin, CosmosMsg, Uint128);

use crate::{
    gatekeeper_common::{CheckTxAgainstRuleResponse, GatekeeperInfo},
    rule::Rule,
};
use common::{coin256::Coin256, common_execute_reasons::CanExecute, universal_msg::UniversalMsg};

#[cfg(feature = "cosmwasm")]
#[cfg_attr(feature = "cosmwasm", cw_serde)]
pub struct InstantiateMsg {
    pub asset_unifier_contract: String,
}

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
pub struct InstantiateMsg {
    pub asset_unifier_contract: String,
    pub asset_unifier_code_hash: String,
}

#[uniserde::uniserde]
pub enum ExecuteMsg {}

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
    serde(rename_all = "snake_case")
)]
#[allow(clippy::large_enum_variant)]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GatekeeperInfo)]
    GatekeeperInfo {},
    /// Returns true if address 1) is admin, 2) is permissioned address and msg is spendable
    /// by permissioned address, or 3) is one of approved cw20s (no funds attached tho).
    /// Notice that the spendlimit gatekeeper isn't aware of its account address at this time,
    /// so this query must specify it.
    #[returns(CheckTxAgainstRuleResponse)]
    CheckTxAgainstRule {
        msg: UniversalMsg,
        sender: String,
        funds: Vec<Coin256>,
        rule: Rule,
        rule_id: u16,
        user_account: String,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        user_account_code_hash: String,
    },
}

#[cfg(feature = "cosmwasm")]
#[cfg_attr(feature = "cosmwasm", cw_serde)]
pub enum QueryMsg {
    GatekeeperInfo {},
    /// Returns true if address 1) is admin, 2) is permissioned address and msg is spendable
    /// by permissioned address, or 3) is one of approved cw20s (no funds attached tho).
    /// Notice that the spendlimit gatekeeper isn't aware of its account address at this time,
    /// so this query must specify it.
    // Check whether specific message(s) is/are authorized
    CheckTxAgainstRule {
        msg: UniversalMsg,
        sender: String,
        funds: Vec<Coin256>,
        rule: Rule,
        rule_id: u16,
        user_account: String,
        user_account_code_hash: Option<String>,
    },
}

#[uniserde::uniserde]
pub struct CanSpendResponse {
    pub can_spend: CanExecute,
    pub repay_msg: Option<CosmosMsg>,
}

#[uniserde::uniserde]
pub struct WasmExecuteMsg {
    contract_addr: String,
    /// msg is the json-encoded ExecuteMsg struct (as raw Binary)
    pub msg: Binary,
    funds: Vec<Coin>,
}

#[uniserde::uniserde]
pub struct TestExecuteMsg {
    pub foo: String,
}

#[uniserde::uniserde]
pub struct TestFieldsExecuteMsg {
    pub recipient: String,
    pub strategy: String,
}

#[uniserde::uniserde]
pub struct UpdateDelayResponse {
    pub update_delay_hours: u16,
}

#[uniserde::uniserde]
pub enum Cw20ExecuteMsg {
    Transfer { recipient: String, amount: Uint128 },
}
