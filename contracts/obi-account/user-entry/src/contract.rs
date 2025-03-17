// === Imports Start ===
macros::cosmwasm_imports!(
    ensure,
    from_binary,
    Addr,
    BankMsg,
    CosmosMsg,
    ReplyOn,
    SubMsg,
    Uint256,
    WasmMsg,
    to_binary,
    Binary,
    Deps,
    DepsMut,
    Env,
    MessageInfo,
    Reply,
    Response,
    StdError,
    StdResult,
    SubMsgResult
);
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
use classes::storage_items::USER_ACCOUNT_CODE_HASH;
use classes::{
    auth::{auth_ensure, BasicAuth},
    gatekeeper_common::LegacyOwnerResponse,
    msg_user_entry::{
        ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UserAccountAddressResponse,
    },
    user_account::GatekeeperContractsResponse,
};
use classes::{signers::Signers, storage_items::USER_ACCOUNT_ADDRESS};
use common::common_error::{AuthorizationError, ContractError};
#[allow(unused_imports)]
#[cfg(feature = "cosmwasm")]
use common::legacy_cosmosmsg as LegacyMsg;
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;
#[cfg(feature = "cosmwasm")]
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
// === Imports End ===

const FIRST_UPDATE_REPLY_ID: u64 = 0;
const EXECUTE_REPLY_ID: u64 = 42;

// === Entry Points Start ===
#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    // user_entry knows user_account address (updateable)
    // everything else, including owner and signer address, is handled by user_account
    let valid_address = deps.api.addr_validate(&msg.user_account_address)?;
    USER_ACCOUNT_ADDRESS.save(deps.storage, &valid_address.to_string())?;
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
    USER_ACCOUNT_CODE_HASH.save(deps.storage, &msg.user_account_code_hash)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Execute { msg, signatures } => {
            // no gatekeeping or auth checking here; this is a pass-through, and auth
            // checking is complex and is done in `user-account`
            auth_ensure(
                &BasicAuth::Deferred,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            execute_execute(deps, env, info, msg, signatures)
        }
        ExecuteMsg::FirstUpdateOwner {
            first_owner,
            evm_contract_address,
            evm_signing_address,
            signers,
        } => first_update_owner(
            deps,
            env,
            info,
            first_owner,
            evm_contract_address,
            evm_signing_address,
            signers,
        ),
        ExecuteMsg::UpdateUserAccountAddress {
            new_address,
            new_code_hash,
        } => {
            #[cfg(not(test))]
            auth_ensure(
                &BasicAuth::OwnerOfAttachedUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;

            // The new account's owner must be the same – sanity check that we are not
            // migrating to the wrong account or an account with a different owner.
            // Note that code signing/verification is not implemented yet.
            #[cfg(not(test))]
            let new_accountowner_res: LegacyOwnerResponse = deps.querier.query_wasm_smart(
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                new_code_hash.clone().unwrap_or_default(),
                deps.api.addr_validate(&new_address)?,
                &classes::msg_user_account::QueryMsg::LegacyOwner {},
            )?;
            #[cfg(test)]
            let new_accountowner_res = LegacyOwnerResponse {
                legacy_owner: info.sender.to_string(),
            };
            ensure!(
                new_accountowner_res.legacy_owner == info.sender,
                AuthorizationError::Unauthorized(macros::loc_string!())
            );
            // In the future we can new_account() here, or that may be overly complex with
            // reply handling, etc. Right now, account must be created first in separate tx.
            USER_ACCOUNT_ADDRESS.save(deps.storage, &new_address)?;
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            USER_ACCOUNT_CODE_HASH.save(deps.storage, &new_code_hash.unwrap())?;
            Ok(Response::default())
        }
        ExecuteMsg::WrappedMigrate {
            account_code_id,
            account_code_hash,
            entry_code_id,
            entry_code_hash,
            state_code_id,
            state_code_hash,
            migrate_msg_binary,
        } => {
            auth_ensure(
                &BasicAuth::OwnerOfAttachedUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let migrate_msg: MigrateMsg = match migrate_msg_binary {
                Some(msg) => from_binary(&msg)?,
                None => MigrateMsg {},
            };
            let mut res = Response::new();
            if let Some(id) = entry_code_id {
                res = res.add_message(get_migrate_msg(
                    env.contract.address.to_string(),
                    id,
                    #[cfg(not(feature = "cosmwasm"))]
                    entry_code_hash.unwrap_or_default(),
                    migrate_msg.clone(),
                ))
            }
            if let Some(id) = state_code_id {
                // user-entry doesn't know user-state, so we need to ask user-account;
                // maybe better to create a pass-thru
                let account_res: GatekeeperContractsResponse = deps.querier.query_wasm_smart(
                    #[cfg(not(feature = "cosmwasm"))]
                    USER_ACCOUNT_CODE_HASH.load(deps.storage)?,
                    USER_ACCOUNT_ADDRESS.load(deps.storage)?,
                    &classes::msg_user_account::QueryMsg::GatekeeperContracts {},
                )?;
                #[cfg(not(feature = "cosmwasm"))]
                if account_res.user_state_contract_addr.is_some() {
                    ensure!(
                        state_code_hash.is_some(),
                        ContractError::Std(StdError::generic_err(
                            "State code hash must be provided if state code id is provided",
                        ))
                    );
                    res = res.add_message(get_migrate_msg(
                        account_res.user_state_contract_addr.unwrap(),
                        id,
                        state_code_hash.unwrap_or_default(),
                        migrate_msg.clone(),
                    ))
                } else {
                    return Err(ContractError::Std(StdError::generic_err(
                        "User state contract details not found in user account",
                    )));
                }
            }
            if let Some(id) = account_code_id {
                #[cfg(not(feature = "cosmwasm"))]
                ensure!(
                    account_code_hash.is_some(),
                    ContractError::Std(StdError::generic_err(
                        "Account code hash must be provided if account code id is provided",
                    ))
                );
                res = res.add_message(get_migrate_msg(
                    USER_ACCOUNT_ADDRESS.load(deps.storage)?,
                    id,
                    #[cfg(not(feature = "cosmwasm"))]
                    account_code_hash.clone().unwrap_or_default(),
                    migrate_msg,
                ));
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                USER_ACCOUNT_CODE_HASH.save(deps.storage, &account_code_hash.unwrap())?;
            }
            Ok(res)
        }
    }
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::UserAccountAddress {} => to_binary(&UserAccountAddressResponse {
            user_account_address: USER_ACCOUNT_ADDRESS.load(deps.storage)?,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash: USER_ACCOUNT_CODE_HASH.load(deps.storage)?,
        }),
    }
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    // note that secret doesn't preserve anything in data automatically as
    // other cosmwasm chains do
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    struct CosmosMsgs {
        msgs: Vec<CosmosMsg>,
    }
    deps.api
        .debug(&format!("User entry is handling reply: {:?}", msg));
    match msg.id {
        EXECUTE_REPLY_ID => match msg.result {
            SubMsgResult::Ok(res) => match res.data {
                Some(data) => {
                    let mut trimmed_data = data;
                    // Remove newline (0x0A), space (0x20), SOH (0x01) and control (0x9D) characters
                    // the last two in particular are generally included in data manually set
                    trimmed_data.0.retain(|&byte| {
                        byte != 0x0A && byte != 0x20 && byte != 0x9D && byte != 0x01
                    });
                    let msgs: CosmosMsgs = from_binary(&trimmed_data)?;
                    Ok(Response::default().add_messages(msgs.msgs))
                }
                None => Err(ContractError::Std(StdError::generic_err(
                    "No messages approved to execute",
                ))),
            },
            SubMsgResult::Err(e) => Err(ContractError::Std(StdError::generic_err(format!(
                "Submsg error result: {}",
                e
            )))),
        },
        FIRST_UPDATE_REPLY_ID => Ok(Response::default()),
        _ => Err(ContractError::Std(StdError::generic_err(
            "reply id not recognized",
        ))),
    }
}
// === Entry Points End ===

/// Executes a wrapped `UniversalMsg` by forwarding it to the user account.
///
/// This function loads the user account's address and code hash (if on secretwasm)
/// from the contract storage and constructs a `CosmosMsg::Wasm(WasmMsg::Execute)` message
/// to forward the actual execution to the user account. Since this user-entry address
/// is where user assets and identity reside, the execution will come back as a reply
/// if execution is allowed by the user-account and its gatekeepers.
///
/// # Arguments
///
/// * `deps` - `DepsMut`, as usual for `execute`.
/// * `_env` - `Env`, as usual for `execute`.
/// * `info` - `MessageInfo`, as usual for `execute`.
/// * `msg` - The actual `Binary` representation of a `UniversalMsg` to execute.
/// * `signatures` - Optional signatures by > threshold of `signers` on the account, for alternate authentication.
///
fn execute_execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: Binary,
    signatures: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    // no gatekeeping or auth checking here; this is done in the account
    let user_account_address: String = USER_ACCOUNT_ADDRESS.load(deps.storage)?;
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
    let user_account_code_hash = USER_ACCOUNT_CODE_HASH.load(deps.storage)?;
    deps.api.debug("Attaching message...");
    Ok(Response::new().add_submessage(SubMsg {
        id: EXECUTE_REPLY_ID,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: user_account_address,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            code_hash: user_account_code_hash,
            msg: to_binary(&classes::msg_user_account::ExecuteMsg::Execute {
                msg,
                // we need to pass along the sender since info.sender in
                // the user-account contract will be this user-entry address
                sender: info.sender.to_string(),
                signatures,
            })?,
            funds: vec![],
        }),
        gas_limit: None,
        reply_on: ReplyOn::Always,
    }))
}

pub fn get_migrate_msg(
    contract_addr: String,
    code_id: u64,
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] code_hash: String,
    migrate_msg: MigrateMsg,
) -> CosmosMsg {
    CosmosMsg::Wasm(WasmMsg::Migrate {
        contract_addr,
        #[cfg(not(feature = "cosmwasm"))]
        code_hash,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        code_id,
        #[cfg(feature = "cosmwasm")]
        new_code_id: code_id,
        msg: to_binary(&migrate_msg).unwrap(),
    })
}

fn first_update_owner(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    first_owner: String,
    evm_contract_address: Option<String>,
    evm_signing_address: Option<String>,
    signers: classes::signers::SignersUnparsed,
) -> Result<Response, ContractError> {
    // auth check
    auth_ensure(
        &BasicAuth::OwnerOfAttachedUserAccount,
        deps.as_ref(),
        &env,
        &info,
        macros::loc_string!(),
        None,
    )?;
    let user_account_address: String = USER_ACCOUNT_ADDRESS.load(deps.storage)?;
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
    let user_account_code_hash = USER_ACCOUNT_CODE_HASH.load(deps.storage)?;
    Ok(Response::new().add_submessage(SubMsg {
        id: FIRST_UPDATE_REPLY_ID,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: user_account_address,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            code_hash: user_account_code_hash,
            msg: to_binary(&classes::msg_user_account::ExecuteMsg::FirstUpdateOwner {
                first_owner,
                evm_contract_address: evm_contract_address.unwrap_or("".to_string()),
                evm_signing_address: evm_signing_address.unwrap_or("".to_string()),
                signers: Signers::new(signers.signers, signers.threshold)?,
            })?,
            funds: vec![],
        }),
        gas_limit: None,
        reply_on: ReplyOn::Error,
    }))
}
