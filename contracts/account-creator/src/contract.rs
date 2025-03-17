macros::cosmwasm_imports!(
    coin,
    ensure,
    Addr,
    CosmosMsg,
    Coin,
    Reply,
    ReplyOn,
    SubMsg,
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
    SubMsgResult,
    Uint128,
);
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
use crate::parse_reply_instantiate::parse_reply_instantiate_data;
use crate::{/*sudo,*/ Addresses, ADDRESSES, CONFIG};
use classes::{
    account_creator::{
        /*AccountSudoMsg,*/ Config, ConfigUpdate, ExecuteMsg, InstantiateMsg, MigrateMsg,
        QueryMsg,
    },
    debtkeeper::InstantiateMsg as DebtInstantiateMsg,
    gatekeeper_common::{update_legacy_owner, LegacyOwnerResponse, LEGACY_OWNER},
    msg_user_account::{
        ExecuteMsg as UserAccountExecuteMsg, InstantiateMsg as UserAccountInstantiateMsg,
    },
    signers::Signers,
    user_account::UserAccount,
};
use common::common_error::{AuthorizationError, ContractError, ReplyError};
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;
#[cfg(feature = "cosmwasm")]
use cw0::parse_reply_instantiate_data;

#[derive(Debug, PartialEq)]
#[repr(u64)]
pub enum FactoryReplies {
    NoReplyHandling = 0,
    InitUserAccount = 1,
    InitUserEntry = 2,
    InitUserState = 3,
    UpdateCodeAdminToSelf = 4,
    InitDebt = 5,
    Unknown = 99,
}

impl From<u64> for FactoryReplies {
    fn from(num: u64) -> Self {
        match num {
            0 => FactoryReplies::NoReplyHandling,
            1 => FactoryReplies::InitUserAccount,
            2 => FactoryReplies::InitUserEntry,
            3 => FactoryReplies::InitUserState,
            4 => FactoryReplies::UpdateCodeAdminToSelf,
            5 => FactoryReplies::InitDebt,
            _ => FactoryReplies::Unknown,
        }
    }
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    LEGACY_OWNER.save(deps.storage, &Some(msg.owner))?;
    CONFIG.save(deps.storage, &msg.config)?;
    ADDRESSES.save(deps.storage, &Addresses::default())?;
    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        0u64 => match msg.result {
            SubMsgResult::Ok(_) => Ok(Response::default()),
            SubMsgResult::Err(res) => Err(ContractError::Std(StdError::generic_err(res))),
        },
        _ => {
            let mut addresses: Addresses = ADDRESSES.load(deps.storage)?;
            let config: Config = CONFIG.load(deps.storage)?;
            #[cfg(feature = "cosmwasm")]
            let reply_msg = msg.clone();
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            let reply_msg = msg.clone();
            let parsed_data = parse_reply_instantiate_data(reply_msg).map_err(|e| {
                ContractError::Reply(ReplyError::CannotParseReplyMsg(e.to_string()))
            })?;
            deps.api
                .debug(&format!("EVENT EXTRACTION: {:#?}", parsed_data));
            let child_contract = deps.api.addr_validate(&parsed_data.contract_address)?;
            let parsed_msg_id = FactoryReplies::from(msg.id);
            let msg_to_attach: Option<Vec<CosmosMsg>> = match parsed_msg_id {
                FactoryReplies::InitUserEntry => {
                    addresses.user_entry = Some(child_contract.to_string());
                    // now we know user entry, so we can update code admins
                    #[cfg(not(test))]
                    {
                        let admins_to_update = [
                            (
                                addresses.user_account.clone(),
                                #[cfg(not(feature = "cosmwasm"))]
                                config.user_account_code_hash.clone(),
                            ),
                            (
                                addresses.user_entry.clone(),
                                #[cfg(not(feature = "cosmwasm"))]
                                config.user_entry_code_hash,
                            ),
                        ];
                        let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
                        // iterate through the contracts to update
                        // since code_hash will be feature gated, use index iterating
                        // instead of tuple destructuring
                        for contract_details in admins_to_update {
                            if let Some(addr) = contract_details.0 {
                                let update_code_admin_msg = WasmMsg::UpdateAdmin {
                                    admin: addresses.user_entry.clone().unwrap(),
                                    contract_addr: addr.to_string(),
                                };
                                cosmos_msgs.push(CosmosMsg::Wasm(update_code_admin_msg));
                            }
                        }
                        Some(cosmos_msgs)
                    }
                    #[cfg(test)]
                    None
                }
                FactoryReplies::InitUserAccount => {
                    addresses.user_account = Some(child_contract.to_string());
                    Some(vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
                        // init owner to factory so can auto-update to user-entry later
                        #[cfg(not(test))]
                        admin: Some(env.contract.address.to_string()),
                        #[cfg(test)]
                        admin: Some("contract14".to_string()), //user entry in factory integration test
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: config.user_entry_code_hash.to_string(),
                        code_id: config.user_entry_code_id,
                        msg: to_binary(&classes::msg_user_entry::InstantiateMsg {
                            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                            user_account_code_hash: config.user_account_code_hash,
                            user_account_address: child_contract.to_string(),
                        })?,
                        funds: vec![],
                        label: format!(
                            "User Entry {}-{}",
                            &env.contract.address.to_string()
                                [(&env.contract.address.to_string().len() - 4)..],
                            env.block.height
                        ),
                    })])
                }
                FactoryReplies::UpdateCodeAdminToSelf => {
                    match msg.result {
                        SubMsgResult::Ok(_) => {}
                        SubMsgResult::Err(res) => {
                            // res.to_string(),
                            //     env.contract.address.to_string(),
                            //     addresses.user_account,
                            return Err(ContractError::Reply(ReplyError::CannotUpdateCodeAdmin(
                                res,
                                env.contract.address.to_string(),
                                addresses.user_account,
                            )));
                        }
                    }
                    None
                }
                FactoryReplies::InitDebt => {
                    addresses.debt = Some(child_contract.to_string());
                    let attach_debtkeeper_msg =
                        &classes::msg_user_account::ExecuteMsg::AttachDebtkeeper {
                            debtkeeper_addr: addresses.debt.clone().unwrap(),
                        };
                    Some(vec![CosmosMsg::Wasm(WasmMsg::Execute {
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        code_hash: config.user_account_code_hash,
                        contract_addr: addresses.user_account.clone().unwrap(),
                        msg: to_binary(&attach_debtkeeper_msg).unwrap(),
                        funds: vec![],
                    })])
                }
                FactoryReplies::InitUserState => {
                    deps.api.debug(&format!(
                        "New user_state in reply: {:#?}",
                        child_contract.to_string()
                    ));
                    addresses.user_state = Some(child_contract.to_string());
                    // we need to store the user state address with the user account
                    let attach_user_state_msg = UserAccountExecuteMsg::AttachUserState {
                        user_state_addr: addresses.user_state.clone(),
                        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                        user_state_code_hash: Some(config.user_state_code_hash),
                    };
                    // in order to let user_account FirstUpdateOwner check it is called by
                    // authorized user_entry, user_state must be aware of user_entry
                    deps.api.debug(&format!(
                        "Saving user_entry to user_state: {:#?}",
                        addresses.user_entry.clone().unwrap()
                    ));
                    Some(vec![
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                            code_hash: config.user_account_code_hash.clone(),
                            contract_addr: addresses.user_account.clone().unwrap(),
                            msg: to_binary(&attach_user_state_msg).unwrap(),
                            funds: vec![],
                        }),
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                            code_hash: config.user_account_code_hash,
                            contract_addr: addresses.user_account.clone().unwrap(),
                            msg: to_binary(&UserAccountExecuteMsg::SetUserStateEntry {
                                new_user_entry: addresses.user_entry.clone().unwrap(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                    ])
                }
                FactoryReplies::Unknown => {
                    return Err(ContractError::Reply(
                        ReplyError::InvalidInstantiateReplyId {},
                    ));
                }
                FactoryReplies::NoReplyHandling => None,
            };
            ADDRESSES.save(deps.storage, &addresses).map_err(|_| {
                ContractError::Reply(ReplyError::FailedToHandleReply(format!(
                    "{:?}",
                    parsed_msg_id
                )))
            })?;
            deps.api.debug(&format!(
                "Instantiated contract from factory: {:?} to contract address: {}",
                parsed_msg_id, child_contract
            ));
            match msg_to_attach {
                None => Ok(Response::default()),
                Some(m) => {
                    println!("Attaching message in reply(): {:#?}", m);
                    let mut res = Response::default();
                    if parsed_msg_id == FactoryReplies::InitUserAccount {
                        res = res.add_submessage(make_submsg(
                            m[0].clone(),
                            ReplyOn::Always,
                            FactoryReplies::InitUserEntry as u64,
                        ));
                    } else if parsed_msg_id == FactoryReplies::InitUserState {
                        for msg in m {
                            res = res.add_submessage(make_submsg(
                                msg,
                                ReplyOn::Never,
                                FactoryReplies::NoReplyHandling as u64,
                            ));
                        }
                    } else if parsed_msg_id == FactoryReplies::InitUserEntry {
                        if !env.block.chain_id.starts_with("secretdev") {
                            for msg in m {
                                res = res.add_message(msg);
                            }
                        }
                    } else {
                        for msg in m {
                            res = res.add_message(msg);
                        }
                    }
                    Ok(res)
                }
            }
        }
    }
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
        ExecuteMsg::UpdateLegacyOwner { new_legacy_owner } => {
            let valid_owner = deps.api.addr_validate(&new_legacy_owner)?;
            update_legacy_owner(deps, env, info, valid_owner)
        }
        ExecuteMsg::UpdateConfig { new_config } => {
            execute_update_config(deps, env, info, new_config)
        }
        ExecuteMsg::NewAccount {
            owner,
            signers,
            update_delay,
            fee_debt,
            user_state,
            user_state_code_hash,
            next_hash_seed,
        } => execute_new_account(
            deps,
            env,
            info,
            owner,
            Signers::new(signers.signers, None)?,
            fee_debt,
            update_delay,
            user_state,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_state_code_hash,
            next_hash_seed,
        ),
        ExecuteMsg::InitDebt { owner, fee_debt } => {
            ensure_caller_is_self(&info.sender, &env, macros::loc_string!())?;
            init_debt(deps.as_ref(), env, owner, fee_debt)
        }
        ExecuteMsg::SetupUserAccount {
            owner,
            signers,
            update_delay,
            user_state,
            user_state_code_hash,
            next_hash_seed,
        } => {
            ensure_caller_is_self(&info.sender, &env, macros::loc_string!())?;
            init_user_account(
                deps,
                env,
                info,
                owner,
                Signers::new(signers.signers, None)?,
                update_delay,
                user_state,
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                user_state_code_hash,
                next_hash_seed,
            )
        }
        ExecuteMsg::SetupUserState { owner } => {
            // caller is self
            ensure_caller_is_self(&info.sender, &env, macros::loc_string!())?;
            #[allow(clippy::redundant_clone)]
            let gatekeeper_init_msgs = get_user_state_init_msg(deps.as_ref(), env, owner.clone())?;
            let res = Response::new().add_submessage(make_submsg(
                gatekeeper_init_msgs,
                ReplyOn::Always,
                FactoryReplies::InitUserState as u64,
            ));
            Ok(res)
        }
    }
}

fn ensure_caller_is_self(
    sender: &Addr,
    env: &Env,
    call_location: String,
) -> Result<(), ContractError> {
    ensure!(
        sender == &env.contract.address,
        ContractError::Auth(AuthorizationError::UnauthorizedInfo(
            env.contract.address.to_string(),
            Some(sender.to_string()),
            call_location
        ))
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn execute_new_account(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    owner: String,
    signers: Signers,
    fee_debt: u64,
    update_delay: u64,
    user_state: Option<String>,
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] user_state_code_hash: Option<
        String,
    >,
    next_hash_seed: String,
) -> Result<Response, ContractError> {
    let mut addresses = Addresses {
        owner: owner.clone(),
        ..Default::default()
    };
    if let Some(addr) = &user_state {
        addresses.user_state = Some(addr.to_string());
    }
    ADDRESSES.save(deps.storage, &addresses)?;
    let _config = CONFIG.load(deps.storage)?;
    let self_execute_setup_user_account_submsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::SetupUserAccount {
            owner: owner.clone(),
            signers,
            update_delay,
            user_state: user_state.clone(),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_state_code_hash,
            #[cfg(feature = "cosmwasm")]
            user_state_code_hash: None,
            next_hash_seed,
        })?,
        funds: vec![],
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        code_hash: env.contract.code_hash.to_string(),
    });

    let self_execute_init_debt_submsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        code_hash: env.contract.code_hash.to_string(),
        msg: to_binary(&ExecuteMsg::InitDebt {
            owner: owner.clone(),
            fee_debt,
        })?,
        funds: vec![],
    });

    let res = Response::new()
        .add_submessage(make_submsg(
            self_execute_setup_user_account_submsg,
            ReplyOn::Error,
            FactoryReplies::NoReplyHandling as u64,
        ))
        .add_submessage(make_submsg(
            self_execute_init_debt_submsg,
            ReplyOn::Error,
            FactoryReplies::InitDebt as u64,
        ));

    // If we're not attaching an already-existing user state,
    // we need to create a new one
    add_setup_submsg_for_user_state_if_none_exists(env, res, user_state, owner)
}

fn add_setup_submsg_for_user_state_if_none_exists(
    env: Env,
    res: Response,
    user_state: Option<String>,
    owner: String,
) -> Result<Response, ContractError> {
    if user_state.is_none() {
        let self_execute_setup_user_state_submsg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::SetupUserState { owner })?,
            funds: vec![],
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            code_hash: env.contract.code_hash,
        });
        Ok(res.add_submessage(make_submsg(
            self_execute_setup_user_state_submsg,
            ReplyOn::Error,
            FactoryReplies::NoReplyHandling as u64,
        )))
    } else {
        Ok(res)
    }
}

pub fn make_submsg(msg: CosmosMsg, reply: ReplyOn, id: u64) -> SubMsg {
    SubMsg {
        id,
        msg,
        gas_limit: None,
        reply_on: reply,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn init_user_account(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    owner: String,
    signers: Signers,
    update_delay: u64,
    user_state: Option<String>,
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] user_state_code_hash: Option<
        String,
    >,
    next_hash_seed: String,
) -> Result<Response, ContractError> {
    deps.api
        .debug("Self-call: Instantiating user account contract");
    let addresses: Addresses = ADDRESSES.load(deps.storage)?;
    deps.api.debug(&format!("Addresses: {:#?}", addresses));
    let config: Config = CONFIG.load(deps.storage)?;
    deps.api.debug(&format!(
        "user_state in init_user_account: {:#?}",
        user_state
    ));
    deps.api.debug(&format!(
        "addresses.user_state in init_user_account: {:#?}",
        addresses.user_state
    ));
    let user_account_init_msg = CosmosMsg::Wasm(WasmMsg::Instantiate {
        code_id: config.user_account_code_id,
        msg: to_binary(&UserAccountInstantiateMsg {
            account: UserAccount {
                legacy_owner: Some(owner.clone()),
                evm_contract_address: None,
                evm_signing_address: None,
                owner_updates_delay_secs: Some(update_delay),
                asset_unifier_contract_addr: Some(config.asset_unifier_address),
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                asset_unifier_code_hash: Some(config.asset_unifier_code_hash),
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                gatekeepers: config
                    .default_gatekeepers
                    .into_iter()
                    .map(|g| (g.1, g.2))
                    .collect(),
                #[cfg(feature = "cosmwasm")]
                gatekeepers: config
                    .default_gatekeepers
                    .into_iter()
                    .map(|g| g.1)
                    .collect(),
                debtkeeper_contract_addr: None,
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                debtkeeper_code_hash: Some(config.debtkeeper_code_hash),
                fee_pay_wallet: Some(config.fee_pay_address),
                signers,
                user_state_contract_addr: if user_state.is_some() {
                    deps.api.debug(&format!(
                        "user_state_contract_addr in init_user_account: {:#?}",
                        user_state
                    ));
                    user_state
                } else {
                    addresses.user_state
                },
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                user_state_code_hash: if let Some(ch) = user_state_code_hash {
                    Some(ch)
                } else {
                    // we don't return code hash in reply data, but storing it in config is fine
                    Some(config.user_state_code_hash)
                },
                magic_update: true,
                // combine a common string with the inputted next_hash_seed and hash it.
                // on-chain randomness can be used in future versions, maybe, but still has
                // some weird dependency issue
                nexthash: common::keccak256hash(
                    format!(
                        "{}{}",
                        next_hash_seed, "7803p7v087p8wdasrlh1b3fjd87tvtvA+RFT2tlatrasbtenb"
                    )
                    .as_bytes(),
                ),
            },
        })?,
        funds: vec![],

        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        code_hash: config.user_account_code_hash,
        #[cfg(not(test))]
        admin: Some(env.contract.address.to_string()),
        #[cfg(test)]
        admin: Some("contract14".to_string()), //user entry in factory integration test
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        label: format!("{}-{}-{}", "obi_account", &owner[6..12], env.block.height),
        #[cfg(feature = "cosmwasm")]
        label: format!("{}-{}", "obi_account", env.block.height),
    });

    // replies are processed in between
    Ok(Response::new().add_submessage(make_submsg(
        user_account_init_msg,
        ReplyOn::Always,
        FactoryReplies::InitUserAccount as u64,
    )))
}

pub fn init_debt(
    deps: Deps,
    env: Env,
    _owner: String,
    // todo: use
    _fee_debt: u64,
) -> Result<Response, ContractError> {
    println!("Self-call: Instantiating debt gatekeeper contract");
    // caller is self

    let config: Config = CONFIG.load(deps.storage)?;
    // instantiate debt gatekeeper
    let user_acct_addy = match ADDRESSES.load(deps.storage)?.user_account {
        None => {
            return Err(ContractError::Std(StdError::generic_err(
                "user account address not set",
            )))
        }
        Some(addy) => addy,
    };
    let debt_init: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Instantiate {
        code_id: config.debtkeeper_code_id,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        code_hash: config.debtkeeper_code_hash,
        msg: to_binary(&DebtInstantiateMsg {
            asset_unifier_contract: config.asset_unifier_address,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            asset_unifier_code_hash: config.asset_unifier_code_hash,
            user_account: user_acct_addy,
        })?,
        label: format!("{}-{}", "Debtkeeper", env.block.height),
        funds: vec![],
        admin: None,
    });
    Ok(Response::new().add_submessage(make_submsg(debt_init, ReplyOn::Always, 5u64)))
}

pub fn get_user_state_init_msg(
    deps: Deps,
    env: Env,
    owner: String,
) -> Result<CosmosMsg, ContractError> {
    let addresses = ADDRESSES.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    let user_state_init = CosmosMsg::Wasm(WasmMsg::Instantiate {
        code_id: config.user_state_code_id,
        #[cfg(not(test))]
        admin: Some(addresses.user_entry.clone().unwrap()),
        #[cfg(test)]
        admin: Some("contract14".to_string()), //user entry in factory integration test
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        code_hash: config.user_state_code_hash,
        msg: to_binary(&classes::msg_user_state::InstantiateMsg {
            user_account_address: addresses.user_account.unwrap_or_default(),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash: config.user_account_code_hash,
        })?,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        label: format!("{}-{}-{}", "user_state", &owner[6..12], env.block.height),
        #[cfg(feature = "cosmwasm")]
        label: format!("{}-{}", "user_state", env.block.height),
        funds: vec![],
    });

    Ok(user_state_init)
}

pub fn execute_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_config: ConfigUpdate,
) -> Result<Response, ContractError> {
    ensure!(
        Some(info.sender.to_string()) == LEGACY_OWNER.load(deps.storage)?,
        ContractError::Auth(AuthorizationError::UnauthorizedInfo(
            env.contract.address.to_string(),
            Some(info.sender.to_string()),
            macros::loc_string!(),
        ))
    );
    let mut config = CONFIG.load(deps.storage)?;
    config.update(new_config);
    // probably should have some validations here
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

// // NEW_ACCOUNT can be called by anyone, without verification of sender
// #[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
// #[cfg_attr(feature = "cosmwasm", entry_point)]
// pub fn sudo(deps: DepsMut, _env: Env, msg: AccountSudoMsg) -> Result<Response, ContractError> {
//     match msg {
//         AccountSudoMsg::BeforeTx {
//             msgs,
//             tx_bytes,
//             cred_bytes,
//             simulate,
//             ..
//         } => sudo::before_tx(
//             deps.as_ref(),
//             &msgs,
//             &tx_bytes,
//             cred_bytes.as_ref(),
//             simulate,
//         ),
//         AccountSudoMsg::AfterTx { .. } => sudo::after_tx(),
//     }
// }

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::LegacyOwner {} => {
            to_binary(&query_legacy_owner(deps)?).map_err(ContractError::Std)
        }
        QueryMsg::Config {} => to_binary(&query_config(deps)?).map_err(ContractError::Std),
    }
}

pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

pub fn query_legacy_owner(deps: Deps) -> StdResult<LegacyOwnerResponse> {
    let legacy_owner = LEGACY_OWNER.load(deps.storage)?;
    let legacy_owner = match legacy_owner {
        Some(legacy_owner) => legacy_owner,
        None => "No owner".to_string(),
    };
    Ok(LegacyOwnerResponse { legacy_owner })
}
