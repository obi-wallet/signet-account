// === Imports Start ===
macros::cosmwasm_imports!(
    ensure,
    from_binary,
    Addr,
    CosmosMsg,
    ReplyOn,
    SubMsg,
    Uint128,
    Uint256,
    WasmMsg,
    to_binary,
    Binary,
    Deps,
    DepsMut,
    Env,
    MessageInfo,
    Response,
    StdError,
    StdResult,
);
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;

use classes::{
    auth::{auth_ensure, BasicAuth},
    gatekeeper_common::GatekeeperType,
    msg_user_account::MigrateMsg,
    signers::Signers,
    user_account::{CanExecuteResponse, NextHashResponse, SignersResponse},
    user_state::AbstractionRule,
};
use common::{
    common_error::{AccountError, AuthorizationError, ContractError, MessageError, UpdateError},
    common_execute_reasons::{readable, CanExecute, CanExecuteReason},
    universal_msg::UniversalMsg,
};

use classes::{
    gatekeeper_common::LegacyOwnerResponse,
    msg_user_account::{ExecuteMsg, InstantiateMsg, QueryMsg},
    storage_items::{ACCOUNT, PENDING},
    user_account::{
        GatekeeperContractsResponse, PendingOwnerResponse, UpdateDelayResponse, UserAccount,
    },
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
// === Imports End ===

const DEFAULT_UPDATE_DELAY: u64 = 0;

// === Entry Points Start ===
#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let account: UserAccount = msg.account;
    ACCOUNT.save(deps.storage, &account)?;
    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddAbstractionRule { new_rule } => {
            // for backward compatibility, we currently allow gatekeepers to add abstraction rules,
            // and they enforce owner verification. This should be redesigned later.
            // here we will at least add restrictive logic; gatekeepers shouldn't have complete upsert access on state
            auth_ensure(
                &BasicAuth::GatekeeperOrOwnerOfLocalUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;

            let account: UserAccount = ACCOUNT.load(deps.storage)?;
            execute_attach_submsg_add_rule(account, new_rule)
        }
        ExecuteMsg::AttachDebtkeeper { debtkeeper_addr } => {
            // no auth check, since this is upon factory creation,
            // TODO: need to re-institute code where debtkeeper
            // cannot be replaced/dropped if there is debt
            let mut account = ACCOUNT.load(deps.storage)?;
            // fail if debtkeeper is attached
            if matches!(account.debtkeeper_contract_addr, Some(_)) {
                Err(ContractError::Std(StdError::generic_err(format!(
                    "Debtkeeper already attached: {:?}",
                    account.debtkeeper_contract_addr
                ))))
            } else {
                deps.api.debug("Setting debtkeeper...");
                account.set_debtkeeper_contract(Some(debtkeeper_addr));
                ACCOUNT
                    .save(deps.storage, &account)
                    .map_err(|_| ContractError::Account(AccountError::CannotSaveAccount {}))?;
                deps.api.debug("Debtkeeper set!");
                Ok(Response::default())
            }
        }
        ExecuteMsg::AttachUserState {
            user_state_addr,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_state_code_hash,
        } => {
            let mut account = ACCOUNT.load(deps.storage)?;
            // important to fail if user state is attached
            if matches!(account.user_state_contract_addr, Some(_)) {
                Err(ContractError::Std(StdError::generic_err(format!(
                    "User state already attached: {:?}",
                    account.user_state_contract_addr
                ))))
            } else {
                deps.api.debug(&format!(
                    "Attaching user state contract {:?}",
                    user_state_addr
                ));
                account.set_user_state_contract(user_state_addr);
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                account.set_user_state_code_hash(user_state_code_hash);
                ACCOUNT
                    .save(deps.storage, &account)
                    .map_err(|_| ContractError::Account(AccountError::CannotSaveAccount {}))?;
                deps.api.debug(&format!(
                    "User state contract is now {:?}",
                    account.user_state_contract_addr
                ));
                Ok(Response::default())
            }
        }
        ExecuteMsg::ChangeOwnerUpdatesDelay { new_delay } => {
            auth_ensure(
                &BasicAuth::OwnerOfLocalUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let mut account = ACCOUNT.load(deps.storage)?;
            account.owner_updates_delay_secs = Some(new_delay);
            ACCOUNT
                .save(deps.storage, &account)
                .map_err(|_| ContractError::Account(AccountError::CannotSaveAccount {}))?;
            Ok(Response::default())
        }
        ExecuteMsg::Execute {
            msg,
            sender,
            signatures,
        } => {
            auth_ensure(
                &BasicAuth::Deferred,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            execute_execute(deps, env, info, from_binary(&msg)?, sender, signatures)
        }
        ExecuteMsg::ProposeUpdateOwner {
            new_owner,
            signers,
            signatures,
        } => {
            // special auth check option by signatures; we are incrementally supporting
            // contract-driven multisig as an alternative to native multisig
            let mut account = ACCOUNT.load(deps.storage)?;
            let old_hash = account.nexthash.clone();
            match signatures {
                Some(signatures) => {
                    ensure!(
                        account
                            .verify_signatures(deps.branch(), signatures)
                            .unwrap(),
                        ContractError::Auth(AuthorizationError::Unauthorized(
                            macros::loc_string!()
                        ))
                    );
                }
                None => {
                    account.nexthash = common::keccak256hash(
                        (account.nexthash.clone() + info.sender.as_ref()).as_bytes(),
                    );
                    auth_ensure(
                        &BasicAuth::OwnerOfLocalUserAccount,
                        deps.as_ref(),
                        &env,
                        &info,
                        macros::loc_string!(),
                        None,
                    )?;
                }
            }
            ensure!(
                account.nexthash != old_hash,
                ContractError::Update(UpdateError::NextHashNotUpdated {})
            );
            ACCOUNT.save(deps.storage, &account)?;
            let mut pending = account.clone();
            pending.legacy_owner = Some(new_owner);
            pending.signers = Signers::new(signers.signers, signers.threshold)?;
            PENDING.save(deps.storage, &pending)?;
            Ok(Response::default())
        }
        ExecuteMsg::ConfirmUpdateOwner { signatures } => {
            let mut pending = PENDING.load(deps.storage)?;
            let old_hash = pending.nexthash.clone();
            match signatures {
                Some(signatures) => {
                    ensure!(
                        pending
                            .verify_signatures(deps.branch(), signatures)
                            .unwrap(),
                        ContractError::Auth(AuthorizationError::Unauthorized(
                            macros::loc_string!()
                        ))
                    );
                }
                None => {
                    pending.nexthash = common::keccak256hash(
                        (pending.nexthash.clone() + info.sender.as_ref()).as_bytes(),
                    );
                    ensure!(
                        is_pending_owner(deps.as_ref(), info.sender.to_string())?,
                        ContractError::Update(UpdateError::CallerIsNotPendingNewOwner {})
                    );
                }
            }
            ensure!(
                pending.nexthash != old_hash,
                ContractError::Update(UpdateError::NextHashNotUpdated {})
            );
            PENDING.save(deps.storage, &pending)?;
            ACCOUNT.save(deps.storage, &pending)?;
            Ok(Response::default())
        }
        ExecuteMsg::CancelUpdateOwner {} => {
            let current_account: UserAccount = ACCOUNT.load(deps.storage)?;
            if !is_pending_owner(deps.as_ref(), info.sender.to_string())? {
                auth_ensure(
                    &BasicAuth::OwnerOfLocalUserAccount,
                    deps.as_ref(),
                    &env,
                    &info,
                    macros::loc_string!(),
                    None,
                )?;
            }
            PENDING.save(deps.storage, &current_account)?;
            Ok(Response::default())
        }
        ExecuteMsg::FirstUpdateOwner {
            first_owner,
            evm_contract_address,
            evm_signing_address,
            signers,
        } => {
            deps.api.debug(&format!("Checking auth: {:?}", first_owner));
            auth_ensure(
                &BasicAuth::UserEntry,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let mut account = ACCOUNT.load(deps.storage)?;
            deps.api
                .debug(&format!("Setting first owner to: {:?}", first_owner));
            account.expend_magic_update(deps.api.addr_validate(&first_owner)?)?;
            account.signers = signers;
            // informational purposes only
            account.evm_contract_address = Some(evm_contract_address);
            account.evm_signing_address = Some(evm_signing_address);
            ACCOUNT.save(deps.storage, &account)?;
            deps.api
                .debug(&format!("First owner set to: {:?}", account.legacy_owner));
            Ok(Response::default())
        }
        ExecuteMsg::RmAbstractionRule { ty, rule_id } => {
            auth_ensure(
                &BasicAuth::OwnerOfLocalUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let account = ACCOUNT.load(deps.storage)?;
            // TODO: message-based (not spend-based) rules should be allowed to expire themselves
            execute_attach_submsg_rm_rule(account, ty, rule_id)
        }
        // Only can be successfully called during factory creation
        ExecuteMsg::SetUserStateEntry { new_user_entry } => {
            deps.api.debug(&format!(
                "In user-account, setting user entry in user state to: {}",
                new_user_entry
            ));
            let account: UserAccount = ACCOUNT.load(deps.storage)?;
            Ok(
                Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                    code_hash: account.user_state_code_hash.ok_or(ContractError::Account(
                        AccountError::UserStateCodeHashNotSet(macros::loc_string!()),
                    ))?,
                    contract_addr: account.user_state_contract_addr.ok_or(
                        ContractError::Account(AccountError::UserStateContractAddressNotSet(
                            macros::loc_string!(),
                        )),
                    )?,
                    msg: to_binary(&classes::msg_user_state::ExecuteMsg::SetUserEntry {
                        new_user_entry,
                    })?,
                    funds: vec![],
                })),
            )
        }
        // Updates to a new user account address (called from the old one).
        // Pass-through since user state only knows its account address.
        // Checks of old_owner == new_owner are done in user_state.
        ExecuteMsg::UpdateUserStateAccountAddress {
            new_user_account,
            new_user_account_code_hash,
        } => {
            auth_ensure(
                &BasicAuth::OwnerOfLocalUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let account: UserAccount = ACCOUNT.load(deps.storage)?;

            let update_user_account_inner_msg =
                &classes::msg_user_state::ExecuteMsg::UpdateUserAccount {
                    new_user_account,
                    new_user_account_code_hash,
                };
            let msg_to_attach =
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: account.user_state_contract_addr.ok_or(
                        ContractError::Account(AccountError::UserStateNotAttached {}),
                    )?,
                    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                    code_hash: account.user_state_code_hash.ok_or(ContractError::Account(
                        AccountError::UserStateCodeHashNotSet(macros::loc_string!()),
                    ))?,
                    msg: to_binary(update_user_account_inner_msg)?,
                    funds: vec![],
                });
            Ok(Response::new().add_message(msg_to_attach))
        }
        ExecuteMsg::UpsertAbstractionRule { id, updated_rule } => {
            auth_ensure(
                &BasicAuth::GatekeeperOrOwnerOfLocalUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let account: UserAccount = ACCOUNT.load(deps.storage)?;
            Ok(
                Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                    code_hash: account.user_state_code_hash.unwrap_or_default(),
                    contract_addr: account.user_state_contract_addr.unwrap_or_default(),
                    msg: to_binary(
                        &classes::msg_user_state::ExecuteMsg::UpsertAbstractionRule {
                            id,
                            updated_rule,
                        },
                    )?,
                    funds: vec![],
                })),
            )
        }
    }
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::NextHash {} => {
            let account = ACCOUNT.load(deps.storage)?;
            Ok(to_binary(&NextHashResponse {
                next_hash: account.nexthash,
            })?)
        }
        QueryMsg::LegacyOwner {} => {
            to_binary(&query_legacy_owner(deps)?).map_err(ContractError::Std)
        }
        QueryMsg::PendingOwner {} => {
            to_binary(&query_pending_owner(deps)?).map_err(ContractError::Std)
        }
        QueryMsg::CanExecute {
            address,
            msg,
            funds: _,
        } => to_binary(&can_execute(deps, env, address, msg)?).map_err(ContractError::Std),
        QueryMsg::UpdateDelay {} => {
            to_binary(&query_update_delay(deps)?).map_err(ContractError::Std)
        }
        QueryMsg::GatekeeperContracts {} => {
            to_binary(&query_gatekeeper_contracts(deps)?).map_err(ContractError::Std)
        }
        QueryMsg::Signers {} => to_binary(&query_signers(deps)?).map_err(ContractError::Std),
    }
}
// === Entry Points End ===

// === Main Execute Functions Start === //
fn execute_attach_submsg_add_rule(
    account: UserAccount,
    new_rule: AbstractionRule,
) -> Result<Response, ContractError> {
    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            code_hash: account.user_state_code_hash.unwrap_or_default(),
            contract_addr: account.user_state_contract_addr.unwrap_or_default(),
            msg: to_binary(&classes::msg_user_state::ExecuteMsg::AddAbstractionRule { new_rule })?,
            funds: vec![],
        })),
    )
}

fn execute_attach_submsg_rm_rule(
    account: UserAccount,
    ty: GatekeeperType,
    rule_id: u16,
) -> Result<Response, ContractError> {
    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            code_hash: account.user_state_code_hash.unwrap_or_default(),
            contract_addr: account.user_state_contract_addr.unwrap_or_default(),
            msg: to_binary(&classes::msg_user_state::ExecuteMsg::RmAbstractionRule {
                ty,
                rule_id,
            })?,
            funds: vec![],
        })),
    )
}

fn execute_execute(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: UniversalMsg,
    sender: String,
    signatures: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let valid_sender = deps.api.addr_validate(&sender)?;
    let can_execute_response = match signatures {
        Some(signatures) => {
            deps.api
                .debug(&format!("reached location: {}", macros::loc_string!()));
            let mut account: UserAccount = ACCOUNT.load(deps.storage)?;
            account.can_execute_mut(deps.branch(), signatures)?
        }
        None => can_execute(deps.as_ref(), env, valid_sender.to_string(), msg.clone())?,
    };
    let yes_reason: CanExecuteReason = match can_execute_response.can_execute {
        CanExecute::Yes(reason) => reason,
        CanExecute::No(reason) => {
            deps.api.debug("Cannot execute!");
            return Ok(
                Response::new().add_attribute("cannot_execute_reason", readable(reason as u8))
            );
        }
        CanExecute::Maybe(reason) => {
            deps.api.debug("Cannot execute! Evaluation stuck on maybe");
            return Ok(
                Response::new().add_attribute("pending_execute_reason", readable(reason as u8))
            );
        }
    };

    let mut messages_to_attach: Vec<CosmosMsg> = vec![];
    if let Some(update_spendlimit_message) = can_execute_response.reduce_spendlimit_msg {
        messages_to_attach.push(update_spendlimit_message);
    };

    // If we're acting as owner or sessionkey, we update last activity.
    // Allowance wallets should not be able to update this or they could delay inheritance/recovery dormancy
    // even when the actual owner is incapacitated.
    let update_last_activity_message = CosmosMsg::Wasm(WasmMsg::Execute {
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        code_hash: ACCOUNT.load(deps.storage)?.user_state_code_hash.ok_or(
            ContractError::Account(AccountError::UserStateCodeHashNotSet(macros::loc_string!())),
        )?,
        contract_addr: ACCOUNT.load(deps.storage)?.user_state_contract_addr.ok_or(
            ContractError::Account(AccountError::UserStateContractAddressNotSet(
                macros::loc_string!(),
            )),
        )?,
        msg: to_binary(&classes::msg_user_state::ExecuteMsg::UpdateLastActivity {})?,
        funds: vec![],
    });
    match yes_reason {
        val if val == CanExecuteReason::OwnerNoDelay
            || val == CanExecuteReason::OwnerWithDebtButNoFundsSpent
            || val == CanExecuteReason::OwnerDelayComplete
            || val == CanExecuteReason::NoFundsAndAllowlist
            || val == CanExecuteReason::SessionkeyAsOwner
            || val == CanExecuteReason::SessionkeyAsOwnerWithDebtButNoFundsSpent
            || val == CanExecuteReason::AllowanceWithBlanketAuthorizedToken
            || val == CanExecuteReason::SubrulesPass =>
        {
            messages_to_attach.push(update_last_activity_message)
        }
        _ => {}
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    struct CosmosMsgs {
        msgs: Vec<CosmosMsg>,
    }
    match msg {
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        UniversalMsg::Secret(secret_msg) => {
            deps.api
                .debug(&format!("Replying with secret msg: {:?}", secret_msg));
            Ok(Response::new()
                .add_attribute("execute_msg", "secret_msg")
                .add_attribute("can_execute_reason", readable(yes_reason as u8))
                .add_messages(messages_to_attach)
                .set_data(to_binary(&CosmosMsgs {
                    msgs: vec![secret_msg],
                })?))
        }
        // todo: execute here should be supported just to reduce spend limits etc.
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        UniversalMsg::Legacy(_secret_msg) => Err(ContractError::Std(StdError::generic_err(
            "other-chain cosmos msgs can only be signed, not attached",
        ))),
        #[cfg(feature = "cosmwasm")]
        UniversalMsg::Legacy(cosmos_msg) => {
            let cosmos_msgs = &CosmosMsgs {
                msgs: vec![cosmos_msg.into()],
            };
            let serialized_execute_msg = to_binary(cosmos_msgs)?;
            Ok(Response::new()
                .add_attribute("execute_msg", "legacy_msg")
                .add_attribute("can_execute_reason", readable(yes_reason as u8))
                .add_messages(messages_to_attach)
                .set_data(serialized_execute_msg))
        }
        // UniversalMsg::Osmo(osmo_msg) => {
        //     let converted: CosmosMsg = osmo_msg.into();
        //     Ok(Response::new()
        //         .add_attribute("osmo_msg", "osmo_msg")
        //         .add_message(converted)
        //         .add_messages(messages_to_attach)
        //         .set_data(to_binary(&CosmosMsgs {
        //             msgs: vec![osmo_msg],
        //         })?))
        // }
        UniversalMsg::Eth(_eth_userop) => {
            Err(ContractError::Msg(MessageError::ErrorExecuteEthMessages {}))
        } /*
          These cases only applicable if handling IBC
          *
          #[cfg(feature = "cosmwasm")]
          UniversalMsg::Secret(_secret_msg) => {
              Err(ContractError::Msg(MessageError::ErrorParseSecretMessage {}))
          }
          #[cfg(all(feature = "secretwasm",not(feature = "cosmwasm")))]
          UniversalMsg::Legacy(legacy_msg) => Err(ContractError::NotImplemented {}), // sign or IBC broadcast
          */
    }
}
// === Main Execute Functions End === //

// === Main Query Functions Start === //
pub fn query_update_delay(deps: Deps) -> StdResult<UpdateDelayResponse> {
    let account: UserAccount = ACCOUNT.load(deps.storage)?;
    let update_delay = account
        .owner_updates_delay_secs
        .unwrap_or(DEFAULT_UPDATE_DELAY);
    Ok(UpdateDelayResponse { update_delay })
}

pub fn query_legacy_owner(deps: Deps) -> StdResult<LegacyOwnerResponse> {
    let legacy_owner = ACCOUNT
        .load(deps.storage)?
        .legacy_owner
        .unwrap_or_else(|| "No owner".to_string());
    Ok(LegacyOwnerResponse { legacy_owner })
}

pub fn query_pending_owner(deps: Deps) -> StdResult<PendingOwnerResponse> {
    let pending_owner = PENDING
        .load(deps.storage)?
        .legacy_owner
        .unwrap_or_else(|| "No owner".to_string());
    Ok(PendingOwnerResponse { pending_owner })
}

pub fn query_gatekeeper_contracts(deps: Deps) -> StdResult<GatekeeperContractsResponse> {
    let account: UserAccount = ACCOUNT.load(deps.storage)?;
    Ok(GatekeeperContractsResponse {
        gatekeepers: account.gatekeepers,
        user_state_contract_addr: account.user_state_contract_addr,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        user_state_code_hash: account.user_state_code_hash,
    })
}

pub fn query_signers(deps: Deps) -> StdResult<SignersResponse> {
    let account: UserAccount = ACCOUNT.load(deps.storage)?;
    let signers = account.signers;
    let evm_contract_address = account.evm_contract_address;
    let evm_signing_address = account.evm_signing_address;
    Ok(SignersResponse {
        signers,
        evm_contract_address,
        evm_signing_address,
    })
}
// === Main Query Functions End === //

pub fn is_pending_owner(deps: Deps, address: String) -> Result<bool, ContractError> {
    Ok(address == query_pending_owner(deps)?.pending_owner)
}

pub fn can_execute(
    deps: Deps,
    env: Env,
    address: String,
    msg: UniversalMsg,
) -> Result<CanExecuteResponse, ContractError> {
    let account: UserAccount = ACCOUNT.load(deps.storage)?;
    let can_execute = account.can_execute(deps, env, address, vec![msg])?;
    Ok(can_execute)
}
