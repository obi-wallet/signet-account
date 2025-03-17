use crate::{
    permissioned_address::PermissionedAddressParams,
    signers::{Signers, SignersUnparsed},
};
use common::authorization::Authorization;
#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;

macros::cosmwasm_imports!(Binary);

#[uniserde::uniserde]
pub struct Any {
    pub type_url: String,
    pub value: Binary,
}

#[uniserde::uniserde]
pub struct GatekeeperAuthorizations {
    pub spendlimit_auths: Vec<PermissionedAddressParams>,
    pub beneficiary_auths: Vec<PermissionedAddressParams>,
    // pub blanket_contracts: Vec<BlanketContract>,
    pub message_auths: Vec<Authorization>,
}

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[derive(
    Default, serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
pub struct Addresses {
    pub debt: Option<String>,
    pub owner: String,
    pub user_account: Option<String>,
    pub user_entry: Option<String>,
    pub user_state: Option<String>,
}

#[cfg(feature = "cosmwasm")]
#[derive(Default)]
#[cfg_attr(feature = "cosmwasm", cw_serde)]
pub struct Addresses {
    pub debt: Option<String>,
    pub owner: String,
    pub user_account: Option<String>,
    pub user_entry: Option<String>,
    pub user_state: Option<String>,
}

#[cfg(feature = "cosmwasm")]
macro_rules! config_struct {
    ($struct_name:ident, $update_struct_name:ident, $($field:ident: $field_type:ty),*) => {
        #[cosmwasm_schema::cw_serde]
        pub struct $struct_name {
            $(
                pub $field: $field_type,
            )*
        }

        #[cosmwasm_schema::cw_serde]
        pub struct $update_struct_name {
            $(
                pub $field: Option<$field_type>,
            )*
        }

        impl $struct_name {
            pub fn update(&mut self, new_config: $update_struct_name) {
                $(
                    if let Some(new_value) = new_config.$field {
                        self.$field = new_value;
                    }
                )*
            }
        }
    }
}

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
macro_rules! config_struct {
    ($struct_name:ident, $update_struct_name:ident, $($field:ident: $field_type:ty),*) => {
        #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema)]
        pub struct $struct_name {
            $(
                pub $field: $field_type,
            )*
        }

        #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema)]
        pub struct $update_struct_name {
            $(
                pub $field: Option<$field_type>,
            )*
        }

        impl $struct_name {
            pub fn update(&mut self, new_config: $update_struct_name) {
                $(
                    if let Some(new_value) = new_config.$field {
                        self.$field = new_value;
                    }
                )*
            }
        }
    }
}

#[cfg(feature = "cosmwasm")]
config_struct!(
    Config,
    ConfigUpdate,
    asset_unifier_address: String,
    debt_repay_address: String,
    fee_pay_address: String,
    // code id, address
    default_gatekeepers: Vec<(u64, String)>,
    user_account_code_id: u64,
    debtkeeper_code_id: u64,
    user_entry_code_id: u64,
    user_state_code_id: u64
);

#[cfg(feature = "secretwasm")]
config_struct!(
    Config,
    ConfigUpdate,
    asset_unifier_address: String,
    asset_unifier_code_hash: String,
    debt_repay_address: String,
    fee_pay_address: String,
    // code id, code hash, address
    default_gatekeepers: Vec<(u64, String, String)>,
    user_account_code_id: u64,
    user_account_code_hash: String,
    debtkeeper_code_id: u64,
    debtkeeper_code_hash: String,
    user_entry_code_id: u64,
    user_entry_code_hash: String,
    user_state_code_id: u64,
    user_state_code_hash: String
);

#[uniserde::uniserde]
pub struct InstantiateMsg {
    pub owner: String,
    pub config: Config,
}

#[uniserde::uniserde]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
    UpdateLegacyOwner {
        new_legacy_owner: String,
    },
    UpdateConfig {
        new_config: ConfigUpdate,
    },
    NewAccount {
        owner: String,
        signers: SignersUnparsed,
        fee_debt: u64,
        update_delay: u64,
        user_state: Option<String>, // for migration; use existing user state contract
        user_state_code_hash: Option<String>,
        // let the client provide randomness for this
        next_hash_seed: String,
    },
    InitDebt {
        owner: String,
        fee_debt: u64,
    },
    SetupUserAccount {
        owner: String,
        signers: Signers,
        update_delay: u64,
        user_state: Option<String>,
        user_state_code_hash: Option<String>,
        next_hash_seed: String,
    },
    SetupUserState {
        owner: String,
    },
}

#[uniserde::uniserde]
pub struct MigrateMsg {}

#[uniserde::uniserde]
pub enum QueryMsg {
    LegacyOwner {},
    Config {},
}

#[uniserde::uniserde]
pub struct ReplyMsg {}

#[uniserde::uniserde]
pub enum AccountSudoMsg {
    /// Called by the AnteHandler's BeforeTxDecorator before a tx is executed.
    BeforeTx {
        /// Messages the tx contains
        msgs: Vec<Any>,

        /// The tx serialized into binary format.
        ///
        /// If the tx authentication requires a signature, this is the bytes to
        /// be signed.
        tx_bytes: Binary,

        /// The credential to prove this tx is authenticated.
        ///
        /// This is taken from the tx's "signature" field, but in the case of
        /// AbstractAccounts, this is not necessarily a cryptographic signature.
        /// The contract is free to interpret this as any data type.
        cred_bytes: Option<Binary>,

        /// Whether the tx is being run in the simulation mode.
        simulate: bool,
    },

    /// Called by the PostHandler's AfterTxDecorator after the tx is executed.
    AfterTx {
        /// Whether the tx is being run in the simulation mode.
        simulate: bool,
    },
}
