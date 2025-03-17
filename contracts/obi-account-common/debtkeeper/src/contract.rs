// === Imports Start ===
macros::cosmwasm_imports!(
    ensure,
    to_binary,
    Addr,
    BankMsg,
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
#[cfg(feature = "secretwasm")]
use crate::state::ASSET_UNIFIER_CODE_HASH;
use crate::state::{ASSET_UNIFIER_CONTRACT, STATE};
use classes::{
    auth::{auth_ensure, BasicAuth},
    debtkeeper::{ExecuteMsg, InstantiateMsg, OutstandingDebtResponse, QueryMsg},
    gatekeeper_common::LEGACY_OWNER,
    sources::Sources,
    storage_items::USER_ACCOUNT_ADDRESS,
};
use common::common_error::{AuthorizationError, ContractError};
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;
// === Imports End ===

pub struct SourcedRepayMsg {
    pub repay_msg: Option<BankMsg>,
    pub wrapped_sources: Sources,
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    ASSET_UNIFIER_CONTRACT.save(deps.storage, &msg.asset_unifier_contract)?;
    #[cfg(feature = "secretwasm")]
    ASSET_UNIFIER_CODE_HASH.save(deps.storage, &msg.asset_unifier_code_hash)?;
    USER_ACCOUNT_ADDRESS.save(deps.storage, &msg.user_account)?;
    Ok(Response::new().add_event(
        Event::new("factory_instantiate")
            .add_attribute("contract_address", env.contract.address.to_string()),
    ))
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
            auth_ensure(
                &BasicAuth::OwnerOfAttachedUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let mut cfg = STATE.load(deps.storage)?;
            cfg.legacy_owner = Some(deps.api.addr_validate(&new_legacy_owner)?.to_string());
            STATE.save(deps.storage, &cfg)?;
            Ok(Response::default())
        }
        ExecuteMsg::IncurDebt { additional_debt } => {
            auth_ensure(
                &BasicAuth::OwnerOfAttachedUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let mut cfg = STATE.load(deps.storage)?;
            cfg.adjust_debt(cfg.convert_and_validate_adjustment(
                additional_debt,
                false,
                info.sender,
            )?)?;
            STATE.save(deps.storage, &cfg)?;
            Ok(Response::default())
        }
        ExecuteMsg::ClearDebt { debt_to_clear } => {
            auth_ensure(
                &BasicAuth::OwnerOfAttachedUserAccount,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let mut cfg = STATE.load(deps.storage)?;
            cfg.adjust_debt(cfg.convert_and_validate_adjustment(
                debt_to_clear,
                true,
                info.sender,
            )?)?;
            STATE.save(deps.storage, &cfg)?;
            Ok(Response::default())
        }
        ExecuteMsg::UpdateUserAccount { user_account } => {
            // One-time operation during setup, allowed only if `can_update_user_account`
            auth_ensure(
                &BasicAuth::Open,
                deps.as_ref(),
                &env,
                &info,
                macros::loc_string!(),
                None,
            )?;
            let mut cfg = STATE.load(deps.storage)?;
            ensure!(
                cfg.can_update_user_account,
                ContractError::Auth(AuthorizationError::Unauthorized(macros::loc_string!()))
            );
            cfg.immutable_user_account_contract = user_account;
            cfg.can_update_user_account = false;
            STATE.save(deps.storage, &cfg)?;
            Ok(Response::default())
        }
    }
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::OutstandingDebt {} => {
            let cfg = STATE.load(deps.storage)?;

            // println!("Encoded response: {:#?}", encoded);
            to_binary(&OutstandingDebtResponse {
                amount: cfg.fee_debt.amount.into(),
                denom: cfg.fee_debt.denom,
            })
        }
        QueryMsg::LegacyOwner {} => to_binary(&LEGACY_OWNER.load(deps.storage)?),
    }
}
