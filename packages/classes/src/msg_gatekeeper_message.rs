#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
use cosmwasm_schema::QueryResponses;
macros::cosmwasm_imports!(Binary, Coin);

use crate::gatekeeper_common::{CheckTxAgainstRuleResponse, GatekeeperInfo};
use crate::rule::Rule;
use common::coin256::Coin256;
use common::universal_msg::UniversalMsg;

#[uniserde::uniserde]
pub enum ExecuteMsg {}

#[uniserde::uniserde]
pub struct WasmExecuteMsg {
    contract_addr: String,
    /// msg is the json-encoded ExecuteMsg struct (as raw Binary)
    pub msg: Binary,
    funds: Vec<Coin>,
}

#[uniserde::uniserde]
#[allow(clippy::large_enum_variant)]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GatekeeperInfo)]
    GatekeeperInfo {},
    // Returns authorizations filtered by whatever is not None
    // Doesn't work as previous since state was moved to user_state;
    // may be restored later.
    /* Authorizations {
        user_entry: String,
        identifier: Option<u16>, // overrides all
        actor: Option<String>,
        target_contract: Option<String>,
        message_name: Option<String>,
        wasmaction_name: Option<String>,
        fields: Option<Vec<(String, String)>>,
        limit: Option<u32>,
        start_after: Option<String>,
    }, */
    // Check whether specific message(s) is/are authorized
    #[returns(CheckTxAgainstRuleResponse)]
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
