#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
use cosmwasm_schema::QueryResponses;
macros::cosmwasm_imports!(to_binary, Addr, Binary, Coin, CosmosMsg, StdResult, WasmMsg);
use schemars::JsonSchema;

use crate::{
    gatekeeper_common::{GatekeeperType, LegacyOwnerResponse},
    signers::{Signers, SignersUnparsed},
    user_account::{
        CanExecuteResponse, GatekeeperContractsResponse, NextHashResponse, PendingOwnerResponse,
        SignersResponse, UpdateDelayResponse, UserAccount,
    },
    user_state::AbstractionRule,
};
use common::universal_msg::UniversalMsg;

#[uniserde::uniserde]
pub struct InstantiateMsg {
    pub account: UserAccount,
}

#[uniserde::uniserde]
pub enum ExecuteMsg {
    /// Pass through to user_state, after auth checking
    AddAbstractionRule {
        new_rule: AbstractionRule,
    },
    /// Called during factory setup to attach the instantiated
    /// debtkeeper to the user account; reply goes to factory, so it
    /// must be handled with an execute on the user-account contract
    AttachDebtkeeper {
        debtkeeper_addr: String,
    },
    /// Called only during factory setup: if user state is already
    /// attached, attaching will fail
    AttachUserState {
        user_state_addr: Option<String>,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        user_state_code_hash: Option<String>,
    },
    /// Update the owner of the contract, possibly with a delay
    ProposeUpdateOwner {
        /// The new owner
        new_owner: String,
        /// The new multisig signers
        signers: SignersUnparsed,
        /// nexthash signed by old signers
        signatures: Option<Vec<String>>,
    },
    /// Confirm the update, from the new account
    ConfirmUpdateOwner {
        /// nexthash signed by pending signers
        signatures: Option<Vec<String>>,
    },
    /// Cancel a pending owner update
    CancelUpdateOwner {},
    /// Updates a pre-created owner account to user owner. Only one time, and only
    /// if magic_update is true.
    FirstUpdateOwner {
        first_owner: String,
        evm_contract_address: String,
        evm_signing_address: String,
        signers: Signers,
    },
    /// Change the delay for owner updates, cannot be done if update is pending
    ChangeOwnerUpdatesDelay {
        /// The new delay in seconds
        new_delay: u64,
    },
    /// Execute a message, if it passes the checks
    Execute {
        /// The message to execute. A serialized UniversalMsg
        msg: Binary,
        /// The sender address, since user_entry will usually be the sender. Since execution
        /// can't happen at this account (assets are in user_entry), fake execute calls directly
        /// to this contract may succeed but will only burn fees. (However, they may potentially
        /// reduce spendlimits under the current setup)
        sender: String,
        /// Used for alternate verification (by signers)
        signatures: Option<Vec<String>>,
    },
    RmAbstractionRule {
        ty: GatekeeperType,
        rule_id: u16,
    },
    /// Save user entry address to the attached user state contract
    SetUserStateEntry {
        new_user_entry: String,
    },
    UpdateUserStateAccountAddress {
        new_user_account: String,
        new_user_account_code_hash: Option<String>,
    },
    UpsertAbstractionRule {
        id: u16,
        updated_rule: AbstractionRule,
    },
}

impl ExecuteMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = self;
        to_binary(&msg)
    }
    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg<T: Into<String>, C>(
        self,
        contract_addr: T,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] code_hash: String,
    ) -> StdResult<CosmosMsg<C>>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema,
    {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            code_hash,
            contract_addr: contract_addr.into(),
            msg,
            funds: vec![],
        };
        Ok(execute.into())
    }
}

#[uniserde::uniserde]
pub enum MigrateMsg {}

#[uniserde::uniserde]
#[allow(clippy::large_enum_variant)]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(NextHashResponse)]
    NextHash {},
    /// Query whether the given address can execute the given message,
    /// and in most cases, why or why not. Returns a `CanExecuteResponse`.
    #[returns(CanExecuteResponse)]
    CanExecute {
        /// The address to check
        address: String,
        /// The message to check
        msg: UniversalMsg,
        /// Attached funds
        funds: Vec<Coin>,
    },
    /// Query the current delay when updating owner (disabled currently).
    #[returns(UpdateDelayResponse)]
    UpdateDelay {},
    /// Query the current legacy owner. In frameworks where ownership is
    /// handled elsewhere, such as Andromeda, legacy_owner may be None.
    #[returns(LegacyOwnerResponse)]
    LegacyOwner {},
    /// Query the new owner in a pending update, if applicable.
    #[returns(PendingOwnerResponse)]
    PendingOwner {},
    /// Return all the user's attached gatekeeper contract addresses.
    #[returns(GatekeeperContractsResponse)]
    GatekeeperContracts {},
    /// Return the stored signers, which are only used for information
    /// during recovery from scratch.
    #[returns(SignersResponse)]
    Signers {},
}
