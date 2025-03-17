use cosmwasm_schema::QueryResponses;

#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(Addr);

#[allow(unused_imports)]
use crate::{
    gatekeeper_common::GatekeeperType,
    gatekeeper_common::LegacyOwnerResponse,
    user_state::{AbstractionRule, AbstractionRules},
};

#[uniserde::uniserde]
pub struct MigrateMsg {}

#[uniserde::uniserde]
pub struct InstantiateMsg {
    pub user_account_address: String,
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
    pub user_account_code_hash: String,
}

#[uniserde::uniserde]
pub enum ExecuteMsg {
    AddAbstractionRule {
        new_rule: AbstractionRule,
    },
    RmAbstractionRule {
        ty: GatekeeperType,
        rule_id: u16,
    },
    SetUserEntry {
        new_user_entry: String,
    },
    UpsertAbstractionRule {
        id: u16,
        updated_rule: AbstractionRule,
    },
    UpdateLastActivity {},
    UpdateUserAccount {
        new_user_account: String,
        new_user_account_code_hash: Option<String>,
    },
}

#[uniserde::uniserde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AbstractionRules)]
    AbstractionRules {
        actor: Option<Addr>,
        ty: Vec<GatekeeperType>,
    },
    #[returns(LastActivityResponse)]
    LastActivity {},
    #[returns(UserEntryResponse)]
    UserEntry {},
}

#[uniserde::uniserde]
pub struct LastActivityResponse {
    pub last_activity: u64,
}

#[uniserde::uniserde]
pub struct UserEntryResponse {
    pub user_entry: String,
}
