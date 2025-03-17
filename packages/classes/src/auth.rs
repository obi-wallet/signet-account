#[allow(unused_imports)]
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
use crate::storage_items::USER_ACCOUNT_CODE_HASH;
use crate::{
    gatekeeper_common::LegacyOwnerResponse,
    msg_user_state::UserEntryResponse,
    storage_items::{ACCOUNT, USER_ACCOUNT_ADDRESS},
};
use common::common_error::{AuthorizationError, ContractError};
macros::cosmwasm_imports!(
    ensure,
    Addr,
    Coin,
    Deps,
    Env,
    MessageInfo,
    StdResult,
    Uint128,
    Uint256,
    Uint64
);

/// `BasicAuth` allows us to have standardized, mandatory auth checks with every
/// ExecuteMsg match arm
pub enum BasicAuth {
    // inter-contract call from the attached user account contract
    AttachedUserAccount,
    // inter-contract call from the attached user account contract
    // or a direct call from the actor of a rule (remove only)
    AttachedUserAccountOrRuleActor,
    // authorization check can be explicitly postponed, such as for ExecuteMsg::Execute,
    // where any kind of abstraction rule could be involved
    Deferred,
    // account owner or a known gatekeeper is allowed (to be expanded+restricted later)
    GatekeeperOrOwnerOfLocalUserAccount,
    // anyone can use this entry point
    Open,
    // only the owner of the USER_ACCOUNT_ADDRESS contract in deps.storage
    OwnerOfAttachedUserAccount,
    // only the owner of ACCOUNT in deps.storage can use this entry point
    OwnerOfLocalUserAccount,
    // for entry points only callable in self-executes by their own contract
    SelfExecute,
    // ensure the user entry address is calling; used only for FirstUpdateOwner
    UserEntry,
}

/// Used to standardize authorization checking
pub fn auth_ensure(
    allowed_auth: &BasicAuth,
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    call_location: String,
    // in some cases the authorized actor for a rule can remove the rule,
    // such as a self-destructing session key
    rule_actor: Option<String>,
) -> Result<(), ContractError> {
    match allowed_auth {
        BasicAuth::AttachedUserAccount => {
            ensure!(
                info.sender == USER_ACCOUNT_ADDRESS.load(deps.storage)?,
                ContractError::Auth(AuthorizationError::UnauthorizedInfo(
                    USER_ACCOUNT_ADDRESS.load(deps.storage)?,
                    Some(info.sender.to_string()),
                    call_location,
                ))
            );
            Ok(())
        }
        BasicAuth::AttachedUserAccountOrRuleActor => {
            // Must be user account if rule_actor is None or is not the sender
            if rule_actor.map_or(true, |actor| actor != info.sender) {
                auth_ensure(
                    &BasicAuth::AttachedUserAccount,
                    deps,
                    env,
                    info,
                    call_location,
                    None,
                )?;
            }
            Ok(())
        }
        BasicAuth::Deferred => Ok(()),
        BasicAuth::Open => Ok(()),
        BasicAuth::SelfExecute => {
            ensure!(
                info.sender == env.contract.address,
                ContractError::Auth(AuthorizationError::UnauthorizedInfo(
                    env.contract.address.to_string(),
                    Some(info.sender.to_string()),
                    call_location,
                ))
            );
            Ok(())
        }
        BasicAuth::OwnerOfLocalUserAccount => {
            ensure!(
                info.sender == ACCOUNT.load(deps.storage)?.legacy_owner.unwrap_or_default(),
                ContractError::Auth(AuthorizationError::UnauthorizedInfo(
                    ACCOUNT.load(deps.storage)?.legacy_owner.unwrap_or_default(),
                    Some(info.sender.to_string()),
                    call_location,
                ))
            );
            Ok(())
        }
        BasicAuth::OwnerOfAttachedUserAccount => {
            #[cfg(not(test))]
            let current_account_owner_res: LegacyOwnerResponse = deps.querier.query_wasm_smart(
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                USER_ACCOUNT_CODE_HASH.load(deps.storage)?,
                USER_ACCOUNT_ADDRESS.load(deps.storage)?,
                &crate::msg_user_account::QueryMsg::LegacyOwner {},
            )?;
            #[cfg(test)]
            let current_account_owner_res = LegacyOwnerResponse {
                legacy_owner: info.sender.to_string(),
            };
            ensure!(
                current_account_owner_res.legacy_owner == info.sender,
                AuthorizationError::UnauthorizedInfo(
                    current_account_owner_res.legacy_owner,
                    Some(info.sender.to_string()),
                    call_location
                )
            );
            Ok(())
        }
        BasicAuth::UserEntry => {
            let account = &ACCOUNT.load(deps.storage)?;
            let user_state =
                account
                    .user_state_contract_addr
                    .clone()
                    .ok_or(ContractError::Auth(
                        AuthorizationError::UserStateContractNotSet {},
                    ))?;
            let user_state_code_hash =
                account
                    .user_state_code_hash
                    .clone()
                    .ok_or(ContractError::Auth(
                        AuthorizationError::UserStateCodeHashNotSet {},
                    ))?;
            // query user state to ask for its UserEntry
            let res: StdResult<UserEntryResponse> = deps.querier.query_wasm_smart(
                user_state_code_hash,
                user_state,
                &crate::msg_user_state::QueryMsg::UserEntry {},
            );
            let user_entry_addr = res?.user_entry;
            ensure!(
                user_entry_addr == info.sender,
                AuthorizationError::UnauthorizedInfo(
                    user_entry_addr,
                    Some(info.sender.to_string()),
                    call_location
                )
            );
            Ok(())
        }
        BasicAuth::GatekeeperOrOwnerOfLocalUserAccount => {
            let account = &ACCOUNT.load(deps.storage)?;
            if !account.gatekeepers.iter().any(
                #[cfg(feature = "cosmwasm")]
                |gatekeeper| gatekeeper == &info.sender,
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                |(_, gatekeeper)| gatekeeper == &info.sender.to_string(),
            ) {
                auth_ensure(
                    &BasicAuth::OwnerOfLocalUserAccount,
                    deps,
                    env,
                    info,
                    call_location,
                    None,
                )?;
            }
            Ok(())
        }
    }
}
