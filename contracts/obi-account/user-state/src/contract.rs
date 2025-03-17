// === Imports Start ===
macros::cosmwasm_imports!(ensure, Addr, CosmosMsg, ReplyOn, SubMsg, WasmMsg);
#[cfg(not(feature = "library"))]
macros::cosmwasm_imports!(
    to_binary,
    Binary,
    Deps,
    DepsMut,
    Env,
    Event,
    MessageInfo,
    Response,
    StdError,
    StdResult
);
use crate::state::{ABSTRACTION_RULES, COUNTER, LAST_ACTIVITY, USER_ENTRY};
use classes::msg_user_state::{
    ExecuteMsg, InstantiateMsg, LastActivityResponse, MigrateMsg, QueryMsg, UserEntryResponse,
};
use classes::storage_items::USER_ACCOUNT_ADDRESS;
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
use classes::storage_items::USER_ACCOUNT_CODE_HASH;
use classes::{
    auth::{auth_ensure, BasicAuth},
    gatekeeper_common::{GatekeeperType, LegacyOwnerResponse},
    user_state::{AbstractionRule, AbstractionRules},
};
use common::common_error::{AuthorizationError, ContractError};
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;
// === Imports End ===

// === Entry Points Start ===
#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    COUNTER.save(deps.storage, &0u16)?;

    USER_ACCOUNT_ADDRESS.save(deps.storage, &msg.user_account_address)?;
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
    USER_ACCOUNT_CODE_HASH.save(deps.storage, &msg.user_account_code_hash)?;

    LAST_ACTIVITY.save(deps.storage, &env.block.time.seconds())?;
    Ok(Response::default().add_event(
        Event::new("factory_instantiate")
            .add_attribute("contract_address", env.contract.address.to_string()),
    ))
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
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
        ExecuteMsg::AddAbstractionRule { new_rule } => {
            auth_ensure(
                &BasicAuth::AttachedUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let mut ctr = COUNTER.load(deps.storage)?;
            let mut rule_with_id = new_rule;
            rule_with_id.id = Some(ctr);
            deps.api.debug(&format!(
                "Saving rule: {}",
                String::from_utf8(to_binary(&rule_with_id).unwrap().0).unwrap()
            ));
            #[cfg(feature = "cosmwasm")]
            ABSTRACTION_RULES.save(deps.storage, ctr, &rule_with_id)?;
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            #[allow(clippy::borrow_interior_mutable_const)]
            ABSTRACTION_RULES.insert(deps.storage, &ctr, &rule_with_id)?;
            ctr = ctr.wrapping_add(1u16);
            COUNTER.save(deps.storage, &ctr)?;
            execute_update_last_activity(deps, env)?;
            Ok(Response::default())
        }
        ExecuteMsg::RmAbstractionRule { ty: _, rule_id } => {
            #[cfg(feature = "cosmwasm")]
            let rule = ABSTRACTION_RULES.load(deps.storage, rule_id)?;
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            #[allow(clippy::borrow_interior_mutable_const)]
            let rule = ABSTRACTION_RULES.get(deps.storage, &rule_id).unwrap();
            auth_ensure(
                &BasicAuth::AttachedUserAccountOrRuleActor,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                Some(rule.actor.to_string()),
            )?;
            // remove is a result in secretwasm Keymap, but not in cosmwasm
            #[allow(clippy::declare_interior_mutable_const)]
            #[allow(clippy::borrow_interior_mutable_const)]
            let _res = ABSTRACTION_RULES.remove(
                deps.storage,
                #[cfg(feature = "cosmwasm")]
                rule_id,
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                &rule_id,
            );
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            _res?;
            execute_update_last_activity(deps, env)?;
            Ok(Response::default())
        }
        // Can only be called during factory instantiation
        ExecuteMsg::SetUserEntry { new_user_entry } => {
            deps.api.debug(&format!(
                "In user-state, setting user entry to: {}",
                new_user_entry
            ));
            if USER_ENTRY.may_load(deps.storage)?.is_some() {
                return Err(ContractError::Auth(AuthorizationError::Unauthorized(
                    macros::loc_string!(),
                )));
            }
            USER_ENTRY.save(
                deps.storage,
                &deps.api.addr_validate(&new_user_entry)?.to_string(),
            )?;
            Ok(Response::default())
        }
        ExecuteMsg::UpsertAbstractionRule { id, updated_rule } => {
            auth_ensure(
                &BasicAuth::AttachedUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            #[cfg(feature = "cosmwasm")]
            let res = ABSTRACTION_RULES.save(deps.storage, id, &updated_rule);
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            #[allow(clippy::borrow_interior_mutable_const)]
            #[allow(clippy::borrow_interior_mutable_const)]
            let res = ABSTRACTION_RULES.insert(deps.storage, &id, &updated_rule);
            deps.api.debug(&format!("res: {:?}", res));
            res?;
            Ok(Response::default())
        }
        ExecuteMsg::UpdateLastActivity {} => {
            auth_ensure(
                &BasicAuth::AttachedUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            execute_update_last_activity(deps, env)?;
            Ok(Response::default())
        }
        ExecuteMsg::UpdateUserAccount {
            new_user_account,
            new_user_account_code_hash,
        } => {
            auth_ensure(
                &BasicAuth::AttachedUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let valid_new_account = deps.api.addr_validate(&new_user_account)?;

            // ensure owners of both accounts are same
            let old_owner: LegacyOwnerResponse = deps.querier.query_wasm_smart(
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                USER_ACCOUNT_CODE_HASH.load(deps.storage)?,
                USER_ACCOUNT_ADDRESS.load(deps.storage)?,
                &classes::msg_user_account::QueryMsg::LegacyOwner {},
            )?;

            let new_owner: LegacyOwnerResponse = deps.querier.query_wasm_smart(
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                new_user_account_code_hash.clone().unwrap_or_default(),
                valid_new_account.clone(),
                &classes::msg_user_account::QueryMsg::LegacyOwner {},
            )?;

            ensure!(
                old_owner == new_owner,
                ContractError::Auth(AuthorizationError::Unauthorized(macros::loc_string!()))
            );
            USER_ACCOUNT_ADDRESS.save(deps.storage, &valid_new_account.to_string())?;
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            USER_ACCOUNT_CODE_HASH.save(deps.storage, &new_user_account_code_hash.unwrap())?;
            Ok(Response::default())
        }
    }
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AbstractionRules { actor, ty } => {
            deps.api.debug(&format!(
                "Querying user state for abstraction rules for actor {} and types {:?}...",
                actor
                    .as_ref()
                    .map(|a| a.to_string())
                    .unwrap_or("None".to_string()),
                ty,
            ));
            let formatted_res = AbstractionRules {
                rules: query_abstraction_rules(deps, actor, ty)?,
            };
            let bin = to_binary(&formatted_res)?;
            Ok(bin)
        }
        QueryMsg::LastActivity {} => to_binary(&query_last_activity(deps)),
        QueryMsg::UserEntry {} => {
            let user_entry = USER_ENTRY.load(deps.storage)?;
            Ok(to_binary(&UserEntryResponse { user_entry })?)
        }
    }
}
// === Entry Points End ===

fn execute_update_last_activity(deps: DepsMut, env: Env) -> StdResult<()> {
    let _last_activity = LAST_ACTIVITY.load(deps.storage)?;
    let new_last_activity = env.block.time.seconds();
    LAST_ACTIVITY.save(deps.storage, &new_last_activity)
}

pub fn query_last_activity(deps: Deps) -> LastActivityResponse {
    LastActivityResponse {
        last_activity: LAST_ACTIVITY.load(deps.storage).unwrap(),
    }
}

pub fn query_abstraction_rules(
    deps: Deps,
    actor: Option<Addr>,
    ty: Vec<GatekeeperType>,
) -> StdResult<Vec<AbstractionRule>> {
    let mut abstraction_rules: Vec<AbstractionRule> = vec![];
    if ty.is_empty() {
        abstraction_rules = query_abstraction_rules_by_ty(deps, None)?;
    } else {
        for t in ty {
            deps.api
                .debug(&format!("Processing abstraction rules type {:?}", t));
            let mut new_rules = query_abstraction_rules_by_ty(deps, Some(t))?;
            deps.api
                .debug(&format!("Got {:?} new rules", new_rules.len()));
            for rule in new_rules.drain(..) {
                deps.api.debug(&format!("Appending rule: {:?}", rule));
                abstraction_rules.push(rule);
                deps.api.debug("Rule appended");
            }
        }
    }

    match actor {
        None => Ok(abstraction_rules),
        Some(actor) => Ok(abstraction_rules
            .into_iter()
            .filter(|r| actor == r.actor)
            .collect()),
    }
}

pub fn query_abstraction_rules_by_ty(
    deps: Deps,
    ty: Option<GatekeeperType>,
) -> StdResult<Vec<AbstractionRule>> {
    let binding = ABSTRACTION_RULES;
    let limit = 30;
    #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
    let rules = binding.iter(deps.storage)?;
    #[cfg(feature = "cosmwasm")]
    let rules = binding;

    // Loop over the keys
    // TODO: There is some kind of iterator bug or issue where rules thinks it has an
    // incorrect length. This makes methods like filter/map/filter_map throw
    // index out of bounds errors. It may be a secret_toolkit issue, whether
    // a bug or a consequence of how Keymap is implemented to work with
    // encrypted values.
    #[cfg(feature = "cosmwasm")]
    let rules = rules.range(deps.storage, None, None, cosmwasm_std::Order::Ascending);
    let filtered_rules = rules
        .filter_map(|item| {
            let item_ref = item.as_ref();
            if let Ok((_rule_id, rule)) = item_ref {
                match &ty {
                    Some(t) => {
                        if rule.ty == t.clone() {
                            Some(rule.clone())
                        } else {
                            None
                        }
                    }
                    None => Some(rule.clone()),
                }
            } else {
                None
            }
        })
        .take(limit);
    let collected_rules = filtered_rules.collect::<Vec<AbstractionRule>>();
    deps.api.debug(&format!(
        "Collected filtered rule length: {:?}",
        collected_rules.len()
    ));
    if !collected_rules.is_empty() {
        deps.api
            .debug(&format!("rule idx 0: {:?}", collected_rules[0]));
    }
    Ok(collected_rules)
}
