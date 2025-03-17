// === Imports Start ===
macros::cosmwasm_imports!(
    coin,
    ensure,
    Api,
    Attribute,
    CosmosMsg,
    Uint128,
    Uint256,
    WasmMsg,
    to_binary,
    Addr,
    Binary,
    Coin,
    Deps,
    DepsMut,
    Env,
    Event,
    MessageInfo,
    Response,
    StdError,
    StdResult,
);
use crate::state::{State, STATE};
use classes::{
    debtkeeper::MigrateMsg,
    gatekeeper_common::{
        get_gatekeeper_info, CheckTxAgainstRuleResponse, GatekeeperInfo, GatekeeperType,
        GATEKEEPER_INFO,
    },
    msg_gatekeeper_spendlimit::{ExecuteMsg, InstantiateMsg, QueryMsg},
    msg_user_state::LastActivityResponse,
    permissioned_address::CoinBalance,
    rule::Rule,
    user_account::GatekeeperContractsResponse,
    user_state::AbstractionRules,
};
use common::{
    authorization::{Authorization, Authorizations},
    coin256::Coin256,
    common_error::{
        AuthorizationError, ContractError, FlowError, PermissionedAddressError, SpendLimitError,
    },
    common_execute_reasons::{
        CanExecute,
        CanExecuteReason::{Allowance, AllowanceWithReset, Beneficiary, BeneficiaryWithReset},
        CannotExecuteReason::{
            self, AllowanceExceeded, BeneficiaryInheritanceNotActive, NoMatchingRule,
        },
    },
};
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;
// === Imports End ===

// === Entry Points Start ===
#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let cfg = State {
        asset_unifier_contract: msg.asset_unifier_contract,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        asset_unifier_code_hash: msg.asset_unifier_code_hash,
    };
    STATE.save(deps.storage, &cfg)?;
    GATEKEEPER_INFO.save(
        deps.storage,
        &GatekeeperInfo {
            gatekeeper_type: GatekeeperType::Spendlimit,
            execution_priority: 30u8,
            authority: 50u8,
            spend_rider: false,
        },
    )?;
    Ok(Response::default())
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // There are no executes. This contract is just for logic checking
    // transactions against spendlimit rules.
    Err(ContractError::Auth(AuthorizationError::Unauthorized(
        macros::loc_string!(),
    )))
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    // Match query that the querier provides.
    match msg {
        QueryMsg::GatekeeperInfo {} => Ok(to_binary(&get_gatekeeper_info(deps)?)?),
        QueryMsg::CheckTxAgainstRule {
            msg: _,
            sender,
            funds,
            user_account,
            user_account_code_hash,
            rule,
            rule_id,
        } => to_binary(
            &can_spend(
                deps,
                env,
                user_account,
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                user_account_code_hash,
                sender,
                funds,
                rule,
                rule_id,
            )?
            .0,
        )
        .map_err(|e| ContractError::SpendLimit(SpendLimitError::QueryCanSendError(e.to_string()))),
    }
}
// === Entry Points End ===

/// Return whether `rule` allows `sender` to spend `funds` on behalf of `account_address`
#[allow(clippy::too_many_arguments)]
pub fn can_spend(
    deps: Deps,
    env: Env,
    account_address: String,
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] account_code_hash: String,
    sender: String,
    funds: Vec<Coin256>,
    rule: Rule,
    rule_id: u16,
) -> Result<(CheckTxAgainstRuleResponse, Option<Vec<CoinBalance>>), ContractError> {
    let state: State = STATE.load(deps.storage)?;
    let valid_address = deps.api.addr_validate(&sender)?;

    // iterate through funds, running get_balance on each and
    // constructing a new Vec of CoinLimits. limit_remaining
    // here is just a placeholder
    let funds = funds
        .into_iter()
        .map(|coin| {
            // get_balance not implemented on ETH
            // let balance = get_balance(
            //     deps,
            //     env.clone(),
            //     coin.denom.clone(),
            //     account_address.clone(),
            // )?;
            Ok(CoinBalance {
                denom: coin.denom,
                amount: coin.amount,
                spent_this_inheritance_period: None,
                limit_remaining: Uint256::from(0u128),
            })
        })
        .collect::<StdResult<Vec<CoinBalance>>>()?;
    let as_beneficiary: bool;
    let params = match rule {
        Rule::Spendlimit(r) => {
            as_beneficiary = false;
            r
        }
        Rule::Inheritance(r) => {
            as_beneficiary = true;
            r
        }
        _ => {
            return Err(ContractError::Flow(FlowError::MismatchedRuleTypes(
                macros::loc_string!(),
            )))
        }
    };
    if params.expiration != 0 && params.expiration < env.block.time.seconds() {
        return Ok((
            CheckTxAgainstRuleResponse {
                can_execute: CanExecute::No(CannotExecuteReason::RuleExpired),
                repay_msg: None,
                authorizations: None,
                spend_rider: false,
                subrules: vec![],
                reduce_spendlimit_msg: None,
            },
            None,
        ));
    }
    // Get the spendlimits for the provided sender + assets
    let res = params.check_spendlimits(
        deps,
        env.clone(),
        state.asset_unifier_contract,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        state.asset_unifier_code_hash,
        env.block.time,
        valid_address.to_string(),
        funds,
        #[cfg(feature = "cosmwasm")]
        get_last_activity(deps, account_address)?,
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        get_last_activity(deps, account_address, account_code_hash)?,
        as_beneficiary,
    );
    match res {
        Ok(inner_res) => Ok((
            CheckTxAgainstRuleResponse {
                can_execute: if inner_res.as_beneficiary {
                    if inner_res.should_reset {
                        CanExecute::Yes(BeneficiaryWithReset)
                    } else {
                        CanExecute::Yes(Beneficiary)
                    }
                } else if inner_res.should_reset {
                    CanExecute::Yes(AllowanceWithReset)
                } else {
                    CanExecute::Yes(Allowance)
                },
                repay_msg: None,
                authorizations: Some(Authorizations {
                    authorizations: vec![(
                        rule_id,
                        Authorization {
                            identifier: None,
                            actor: Some(valid_address),
                            contract: None,
                            message_name: None,
                            wasmaction_name: None,
                            fields: None,
                            expiration: 0,
                        },
                    )],
                }),
                spend_rider: true,
                subrules: vec![],
                reduce_spendlimit_msg: None,
            },
            Some(inner_res.sourced_coins.coins),
        )),
        Err(e) => {
            deps.api.debug(&format!("matching error: {:?}", e));
            Ok(match e {
                ContractError::SpendLimit(_) => (
                    CheckTxAgainstRuleResponse {
                        can_execute: CanExecute::No(AllowanceExceeded),
                        repay_msg: None,
                        authorizations: None,
                        spend_rider: true,
                        subrules: vec![],
                        reduce_spendlimit_msg: None,
                    },
                    None,
                ),
                ContractError::PermAddy(
                    PermissionedAddressError::BeneficiaryCooldownNotExpired {},
                ) => (
                    CheckTxAgainstRuleResponse {
                        can_execute: CanExecute::No(BeneficiaryInheritanceNotActive),
                        repay_msg: None,
                        authorizations: None,
                        spend_rider: true,
                        subrules: vec![],
                        reduce_spendlimit_msg: None,
                    },
                    None,
                ),
                _ => (
                    CheckTxAgainstRuleResponse {
                        can_execute: CanExecute::No(NoMatchingRule),
                        repay_msg: None,
                        authorizations: None,
                        spend_rider: true,
                        subrules: vec![],
                        reduce_spendlimit_msg: None,
                    },
                    None,
                ),
            })
        }
    }
}

pub fn maybe_addr(api: &dyn Api, human: Option<String>) -> StdResult<Option<Addr>> {
    human.map(|x| api.addr_validate(&x)).transpose()
}

pub fn get_last_activity(
    deps: Deps,
    account_address: String,
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] account_code_hash: String,
) -> StdResult<u64> {
    let user_state_res: GatekeeperContractsResponse = deps.querier.query_wasm_smart(
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        account_code_hash,
        account_address,
        &classes::msg_user_account::QueryMsg::GatekeeperContracts {},
    )?;
    let last_activity_res: LastActivityResponse = deps.querier.query_wasm_smart(
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        user_state_res.user_state_code_hash.unwrap(),
        user_state_res.user_state_contract_addr.unwrap(),
        &classes::msg_user_state::QueryMsg::LastActivity {},
    )?;
    Ok(last_activity_res.last_activity)
}

pub fn get_spendlimit_rules(
    deps: Deps,
    account_contract: String,
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))] account_code_hash: String,
    sender: Option<String>,
    mut ty: Vec<GatekeeperType>,
) -> Result<AbstractionRules, ContractError> {
    if ty == vec![] {
        ty = vec![GatekeeperType::Spendlimit, GatekeeperType::Inheritance];
    }
    let gatekeepers: GatekeeperContractsResponse = deps.querier.query_wasm_smart(
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        account_code_hash,
        account_contract,
        &classes::msg_user_account::QueryMsg::GatekeeperContracts {},
    )?;
    let query_msg = match sender {
        Some(addy) => {
            let valid_sender = deps.api.addr_validate(&addy)?;
            classes::msg_user_state::QueryMsg::AbstractionRules {
                actor: Some(valid_sender),
                ty,
            }
        }
        None => classes::msg_user_state::QueryMsg::AbstractionRules { actor: None, ty },
    };
    let res: Result<AbstractionRules, StdError> = deps.querier.query_wasm_smart(
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        gatekeepers.user_state_code_hash.unwrap_or_default(),
        gatekeepers.user_state_contract_addr.unwrap_or_default(),
        &query_msg,
    );
    Ok(res?)
}
