use crate::signers::SignersUnparsed;
#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
use cosmwasm_schema::QueryResponses;
macros::cosmwasm_imports!(Binary);
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
use schemars::JsonSchema;

#[cfg(feature = "cosmwasm")]
#[cfg_attr(feature = "cosmwasm", cw_serde)]
pub struct InstantiateMsg {
    pub user_account_address: String,
}

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
    serde(rename_all = "snake_case")
)]
pub struct InstantiateMsg {
    pub user_account_address: String,
    pub user_account_code_hash: String,
}

// we pass through executes since the user_entry address is the user's account
// address, but admin actions can be taken directly and so do not need passthrough
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
    serde(rename_all = "snake_case")
)]
pub enum ExecuteMsg {
    /// Execute a message, if it passes the checks
    Execute {
        /// The message to execute. A serialized UniversalMsg
        /// Entry point doesn't have to know about the UniversalMsg, so it can be updated
        msg: Binary,
        /// Hex signatures for alternate verification method
        signatures: Option<Vec<String>>,
    },
    FirstUpdateOwner {
        first_owner: String,
        evm_contract_address: Option<String>,
        evm_signing_address: Option<String>,
        signers: SignersUnparsed,
    },
    UpdateUserAccountAddress {
        new_address: String,
        new_code_hash: Option<String>,
    },
    /// A WrappedMigrate callable by owner (self) avoids needing to update
    /// code admins whenever owner is updated
    WrappedMigrate {
        account_code_id: Option<u64>,
        account_code_hash: Option<String>,
        entry_code_id: Option<u64>,
        entry_code_hash: Option<String>,
        state_code_id: Option<u64>,
        state_code_hash: Option<String>,
        migrate_msg_binary: Option<Binary>,
    },
}

#[cfg(feature = "cosmwasm")]
#[cfg_attr(feature = "cosmwasm", cw_serde)]
pub enum ExecuteMsg {
    /// Execute a message, if it passes the checks
    Execute {
        /// The message to execute. A serialized UniversalMsg
        /// Entry point doesn't have to know about the UniversalMsg, so it can be updated
        msg: Binary,
        /// Hex signatures for alternate verification method
        signatures: Option<Vec<String>>,
    },
    UpdateUserAccountAddress {
        new_address: String,
        new_code_hash: Option<String>,
    },
    /// A WrappedMigrate callable by owner (self) avoids needing to update
    /// code admins whenever owner is updated
    WrappedMigrate {
        account_code_id: Option<u64>,
        account_code_hash: Option<String>,
        entry_code_id: Option<u64>,
        entry_code_hash: Option<String>,
        state_code_id: Option<u64>,
        state_code_hash: Option<String>,
        migrate_msg_binary: Option<Binary>,
    },
}

#[uniserde::uniserde]
pub struct MigrateMsg {}

#[uniserde::uniserde]
#[allow(clippy::large_enum_variant)]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// for simplicity and longevity, queries and admin actions not passed through. We can
    /// query the user account address and then interact with that directly. Notice that
    /// the Passport signer should use this so that user accounts which are migrated away
    /// from can no longer authorize Passport signing
    #[returns(UserAccountAddressResponse)]
    UserAccountAddress {},
}

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
    serde(rename_all = "snake_case")
)]
pub struct UserAccountAddressResponse {
    pub user_account_address: String,
    // for expediency
    pub user_account_code_hash: String,
}

#[cfg(feature = "cosmwasm")]
#[cfg_attr(feature = "cosmwasm", cw_serde)]
pub struct UserAccountAddressResponse {
    pub user_account_address: String,
}
