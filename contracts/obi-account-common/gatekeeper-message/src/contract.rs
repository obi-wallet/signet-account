// === Imports Start ===
macros::cosmwasm_imports!(
    WasmMsg,
    ensure,
    from_binary,
    to_binary,
    Addr,
    Api,
    Binary,
    Deps,
    DepsMut,
    Env,
    Event,
    MessageInfo,
    Response,
    StdError,
    StdResult,
);
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
use classes::storage_items::USER_ACCOUNT_CODE_HASH;
use classes::{
    debtkeeper::MigrateMsg,
    gatekeeper_common::{
        get_gatekeeper_info, CheckTxAgainstRuleResponse, GatekeeperInfo, GatekeeperType,
        InstantiateMsg, GATEKEEPER_INFO,
    },
    msg_gatekeeper_message::{ExecuteMsg, QueryMsg},
    rule::Rule,
    storage_items::USER_ACCOUNT_ADDRESS,
    user_account::GatekeeperContractsResponse,
    user_state::AbstractionRules,
};
#[cfg(feature = "cosmwasm")]
use common::legacy_cosmosmsg as LegacyMsg;
use common::{
    authorization::Authorizations,
    common_error::{AuthorizationError, ContractError, EthError, MessageError},
    common_execute_reasons::{CanExecute, CannotExecuteReason, PendingReason},
    eth::EthUserOp,
    universal_msg::UniversalMsg,
};
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;
use eth_interpreter::msg::ParseUserOpResponse;
use serde_json_value_wasm::Value;

use crate::state::{ETH_INTERPRETER_ADDRESS, ETH_INTERPRETER_CODE_HASH};
// === Imports End ===

#[allow(dead_code)]
const DEFAULT_LIMIT: u32 = 10;
#[allow(dead_code)]
const MAX_LIMIT: u32 = 30;

// === Entry Points Start ===
#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    ETH_INTERPRETER_ADDRESS.save(deps.storage, &msg.eth_interpreter_address)?;
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
    ETH_INTERPRETER_CODE_HASH.save(deps.storage, &msg.eth_interpreter_code_hash)?;
    GATEKEEPER_INFO.save(
        deps.storage,
        &GatekeeperInfo {
            gatekeeper_type: GatekeeperType::Allowlist,
            execution_priority: 20u8, // most complicated message type, and may have a rider
            authority: 1u8,
            spend_rider: false,
        },
    )?;
    Ok(Response::new().add_event(
        Event::new("factory_instantiate")
            .add_attribute("contract_address", env.contract.address.to_string()),
    ))
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Err(ContractError::Auth(AuthorizationError::Unauthorized(
        macros::loc_string!(),
    )))
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GatekeeperInfo {} => Ok(to_binary(&get_gatekeeper_info(deps)?)?),
        #[allow(unused_variables)]
        QueryMsg::CheckTxAgainstRule {
            msg,
            sender,
            rule,
            rule_id,
            funds: _,
            user_account,
            user_account_code_hash,
        } => to_binary(&check_msg(
            deps,
            env,
            user_account,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash.unwrap_or_default(),
            deps.api.addr_validate(&sender)?,
            msg,
            rule,
            rule_id,
            None,
            None,
        )?)
        .map_err(ContractError::Std),
    }
}
// === Entry Points End ===

/// `authorization` may have:
///
/// `identifier` - A u16 pinpointing a specific authorization. If 0, ignore.
/// `actor` - an Option<Addr>. If Some, find only authorizations which authorize
/// this actor.
///
/// `contract` - an Option<Addr>. If Some, find only authorizations which allow
/// actor(s) to take actions on this contract.
///
/// `message_name` - Option<String>. If Some, find only authorizations matching
/// this message name (for example, "MsgExecuteContract"). Note that a universal
/// authorization by contract or wasmaction_name will result in `true` even if
/// a specific `message_name` is not included.
///
/// `wasmaction_name` – Option<String>. Applicable only to MsgExecuteContract,
/// but works if `message_name` is None. If Some, find only authorizations that
/// are wasm execute messages with this action name (for example, "transfer").
/// Note that a universal authorization by contract or message_name will result in
/// `true` even if a specific `wasmaction_name` is not included.
///
/// `fields` – Option<Vec<(String, String)>. Finds only authorizations which
/// allow messages with certain parameters. For example, if checking whether
/// messages with `token_id` set to `15` are allowed, `fields` should be:
/// vec![("token_id", "15")]. Note that not finding such an authorization does not
/// mean a related transaction will not succeed: the sender may have, for example,
/// a universal authorization on the particular contract or message_name.
/// `authorization` may have:
///
/// `identifier` - A u16 pinpointing a specific authorization. If 0, ignore.
/// `actor` - an Option<Addr>. If Some, find only authorizations which authorize
/// this actor.
///
/// `contract` - an Option<Addr>. If Some, find only authorizations which allow
/// actor(s) to take actions on this contract.
///
/// `message_name` - Option<String>. If Some, find only authorizations matching
/// this message name (for example, "MsgExecuteContract"). Note that a universal
/// authorization by contract or wasmaction_name will result in `true` even if
/// a specific `message_name` is not included.
///
/// `wasmaction_name` – Option<String>. Applicable only to MsgExecuteContract,
/// but works if `message_name` is None. If Some, find only authorizations that
/// are wasm execute messages with this action name (for example, "transfer").
/// Note that a universal authorization by contract or message_name will result in
/// `true` even if a specific `wasmaction_name` is not included.
///
/// `fields` – Option<Vec<(String, String)>. Finds only authorizations which
/// allow messages with certain parameters. For example, if checking whether
/// messages with `token_id` set to `15` are allowed, `fields` should be:
/// vec![("token_id", "15")]. Note that not finding such an authorization does not
/// mean a related transaction will not succeed: the sender may have, for example,
/// a universal authorization on the particular contract or message_name.
pub fn get_authorizations_with_idx(
    deps: Deps,
    msg: Option<Binary>,
    rule: Rule,
    rule_id: u16,
) -> Result<Authorizations, ContractError> {
    deps.api.debug("In get_authorizations_with_idx");
    let (rule_id, authorization) = match rule {
        Rule::Allow(a) => (rule_id, a),
        Rule::Block(a) => (rule_id, a),
        _ => {
            return Err(ContractError::Flow(
                common::common_error::FlowError::MismatchedRuleTypes(macros::loc_string!()),
            ));
        }
    };
    let actor = authorization.actor.as_ref().map(|a| a.to_string());
    // if identifier specified, our task is easy
    if let Some(id) = authorization.identifier {
        let rules = get_message_rules(deps, actor, GatekeeperType::Allowlist)?;
        let working_auths = Authorizations {
            authorizations: rules
                .flatten_main_rules()
                .into_iter()
                .filter_map(|(id, rule)| match rule {
                    Rule::Allow(auth) => Some((id, auth)),
                    _ => None, // handle this later
                })
                .collect(),
        };
        match working_auths.find_by_id(id) {
            Some(auth) => {
                return Ok(Authorizations {
                    authorizations: vec![(id, auth.clone())],
                })
            }
            None => {
                return Ok(Authorizations {
                    authorizations: vec![],
                });
            }
        }
    }

    let matched_authorizations = Authorizations {
        authorizations: vec![(rule_id, authorization)],
    }
    .filter_by_msg(&msg.unwrap());
    Ok(matched_authorizations)
}

fn build_auths_response(
    auths: Authorizations,
) -> Result<CheckTxAgainstRuleResponse, ContractError> {
    let can_execute: CanExecute = if !auths.authorizations.is_empty() {
        CanExecute::Maybe(PendingReason::AllowlistPendingSpendCheck {})
    } else {
        CanExecute::No(CannotExecuteReason::NoMatchingRule {})
    };
    Ok(CheckTxAgainstRuleResponse {
        can_execute,
        repay_msg: None,
        authorizations: Some(auths),
        spend_rider: false,
        subrules: vec![],
        reduce_spendlimit_msg: None,
    })
}

#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn check_msg(
    deps: Deps,
    env: Env,
    user_account: String,
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] user_account_code_hash: String,
    sender: Addr,
    msg: UniversalMsg,
    rule: Rule,
    rule_id: u16,
    interpreter: Option<String>,
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] interpreter_code_hash: Option<
        String,
    >,
) -> Result<CheckTxAgainstRuleResponse, ContractError> {
    let base_no_response = CheckTxAgainstRuleResponse {
        can_execute: CanExecute::No(CannotExecuteReason::NoMatchingRule {}),
        repay_msg: None,
        authorizations: None,
        spend_rider: false,
        subrules: vec![],
        reduce_spendlimit_msg: None,
    };
    // expired rules don't take effect
    // TODO: cleanup expired rules if context is execute
    match &rule {
        Rule::Allow(a) | Rule::Block(a) => {
            if a.expiration > 0 && a.expiration <= env.block.time.seconds() {
                return Ok(CheckTxAgainstRuleResponse {
                    can_execute: CanExecute::No(CannotExecuteReason::RuleExpired {}),
                    repay_msg: None,
                    authorizations: None,
                    spend_rider: false,
                    subrules: vec![],
                    reduce_spendlimit_msg: None,
                });
            }
        }
        _ => return Ok(base_no_response),
    };
    let todo_auths = Authorizations {
        authorizations: vec![],
    };
    match msg {
        #[cfg(feature = "cosmwasm")]
        UniversalMsg::Legacy(msg) => match msg {
            LegacyMsg::CosmosMsg::Bank(_) => Ok(base_no_response),
            LegacyMsg::CosmosMsg::Custom(_) => Ok(base_no_response),
            #[cfg(feature = "staking")]
            LegacyMsg::CosmosMsg::Staking(_) => Ok(base_no_response),
            #[cfg(feature = "staking")]
            LegacyMsg::CosmosMsg::Distribution(_) => Ok(base_no_response),
            LegacyMsg::CosmosMsg::Wasm(msg) => match msg {
                LegacyMsg::WasmMsg::Execute {
                    contract_addr,
                    msg,
                    funds: _,
                } => build_auths_response(check_wasm_msg(
                    deps,
                    Some(deps.api.addr_validate(&contract_addr)?),
                    Binary(msg.0),
                    "MsgExecuteContract".to_string(),
                    rule,
                    rule_id,
                )?),
                LegacyMsg::WasmMsg::Instantiate {
                    admin: _,
                    code_id: _,
                    msg,
                    funds: _,
                    label: _,
                } => build_auths_response(check_wasm_msg(
                    deps,
                    None,
                    Binary(msg.0),
                    "MsgInstantiateContract".to_string(),
                    rule,
                    rule_id,
                )?),
                LegacyMsg::WasmMsg::Migrate {
                    contract_addr,
                    new_code_id,
                    msg,
                } => Ok(base_no_response),
                LegacyMsg::WasmMsg::UpdateAdmin {
                    contract_addr,
                    admin,
                } => Ok(base_no_response),
                LegacyMsg::WasmMsg::ClearAdmin { contract_addr } => Ok(base_no_response),
                _ => Ok(base_no_response),
            },
            _ => Ok(base_no_response),
        },
        UniversalMsg::Eth(userop) => build_auths_response(check_eth_msg(
            deps,
            user_account,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash,
            sender,
            userop,
            rule,
            rule_id,
            interpreter.ok_or(ContractError::Eth(EthError::NoEthInterpreter {}))?,
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            interpreter_code_hash
                .ok_or(ContractError::Eth(EthError::NoEthInterpreterCodeHash {}))?,
        )?),
        _ => Ok(base_no_response),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn check_eth_msg(
    deps: Deps,
    _user_account: String,
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] _user_account_code_hash: String,
    sender: Addr,
    userop: EthUserOp,
    rule: Rule,
    _rule_id: u16,
    eth_interpreter: String,
    #[cfg(feature = "secretwasm")] eth_interpreter_code_hash: String,
) -> Result<Authorizations, ContractError> {
    deps.api.debug(&format!("Checking rule: {:?}", rule));
    let parse_user_op_query_msg = eth_interpreter::msg::QueryMsg::ParseUserOp { user_op: userop };
    let parse_user_query_res: ParseUserOpResponse = from_binary(&deps.querier.query_wasm_smart(
        #[cfg(feature = "secretwasm")]
        eth_interpreter_code_hash,
        eth_interpreter,
        &parse_user_op_query_msg,
    )?)?;
    match rule {
        Rule::Allow(auth) => {
            if auth.fields.is_some()
                || auth.message_name.is_some()
                || auth.wasmaction_name.is_some()
            {
                Err(ContractError::Eth(EthError::NotAnEthContractAllowlist {}))
            } else if let Some(cts) = &auth.contract {
                if cts.contains(
                    parse_user_query_res
                        .contract_address
                        .as_ref()
                        .unwrap_or(&"".to_string()),
                ) && auth.actor == Some(sender)
                {
                    return Ok(Authorizations {
                        authorizations: vec![(auth.identifier.unwrap(), auth)],
                    });
                }
                // no valid contract auth found for this sender
                Ok(Authorizations {
                    authorizations: vec![],
                })
            } else {
                Err(ContractError::Eth(EthError::NotAnEthContractAllowlist {}))
            }
        }
        _ => Err(ContractError::Eth(EthError::NotAnEthContractAllowlist {})),
    }
}

/// Checks a WasmMsg::Execute (MsgExecuteContract) against authorizations table.
/// Returns any matching authorizations.
pub fn check_wasm_msg(
    deps: Deps,
    _target_contract: Option<Addr>,
    msg: Binary,
    _message_name: String,
    rule: Rule,
    rule_id: u16,
) -> Result<Authorizations, ContractError> {
    deps.api.debug("Running check_wasm_msg...");
    let msg_value: Value = serde_json_wasm::from_slice(&msg)?;
    let msg_obj: &serde_json_value_wasm::Map<String, Value> = match msg_value.as_object() {
        Some(obj) => obj,
        None => {
            return Err(ContractError::Auth(AuthorizationError::Unauthorized(
                macros::loc_string!(),
            )))
        }
    };
    let _wasmaction_name = Some(match msg_obj.keys().next() {
        Some(key) => key.to_string(),
        None => {
            return Err(ContractError::Msg(MessageError::NoExecuteMessage {}));
        }
    });
    // note that rules are already filtered by sender when grabbed
    let auths = get_authorizations_with_idx(deps, Some(msg), rule, rule_id)?;
    Ok(auths)
}

pub fn maybe_addr(api: &dyn Api, human: Option<String>) -> StdResult<Option<Addr>> {
    human.map(|x| api.addr_validate(&x)).transpose()
}

pub fn get_message_rules(
    deps: Deps,
    sender: Option<String>,
    ty: GatekeeperType, // here Allowlist or Blocklist is intended
) -> Result<AbstractionRules, ContractError> {
    let account_contract = USER_ACCOUNT_ADDRESS.load(deps.storage)?;
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
    let account_code_hash = USER_ACCOUNT_CODE_HASH.load(deps.storage)?;
    let gatekeepers: GatekeeperContractsResponse = deps.querier.query_wasm_smart(
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        account_code_hash,
        account_contract,
        &classes::msg_user_account::QueryMsg::GatekeeperContracts {},
    )?;
    let query_msg = match sender {
        None => classes::msg_user_state::QueryMsg::AbstractionRules {
            actor: None,
            ty: vec![ty],
        },
        Some(sender) => {
            let valid_sender = deps.api.addr_validate(&sender)?;
            classes::msg_user_state::QueryMsg::AbstractionRules {
                actor: Some(valid_sender),
                ty: vec![ty],
            }
        }
    };
    let res: Result<AbstractionRules, StdError> = deps.querier.query_wasm_smart(
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        gatekeepers.user_state_code_hash.unwrap_or_default(),
        gatekeepers.user_state_contract_addr.unwrap_or_default(),
        &query_msg,
    );
    Ok(res?)
}
