macros::cosmwasm_imports!(
    ensure,
    to_binary,
    BankMsg,
    Binary,
    Deps,
    DepsMut,
    Env,
    MessageInfo,
    Response,
    StdError,
    StdResult,
);
use crate::state::{State, STATE};
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;

use classes::{
    asset_unifier::LegacyOwnerResponse,
    debtkeeper::MigrateMsg,
    gatekeeper_common::{update_legacy_owner, LEGACY_OWNER},
    pair_contract::{PairContract, PairContracts, SwapRouteResponse},
    pair_registry::{ExecuteMsg, InstantiateMsg, QueryMsg},
    sources::Sources,
};

use common::common_error::{AuthorizationError, ContractError};

pub struct SourcedRepayMsg {
    pub repay_msg: Option<BankMsg>,
    pub wrapped_sources: Sources,
}

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
    LEGACY_OWNER.save(deps.storage, &msg.legacy_owner)?;
    let cfg = State {
        pair_contracts: PairContracts {
            swap_routes: vec![],
        },
    };
    STATE.save(deps.storage, &cfg)?;
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
    // temporarily mapping these errors over
    match msg {
        ExecuteMsg::UpsertPair {
            token0,
            token1,
            routes,
        } => upsert_pair_addrs(deps, env, info, token0, token1, routes).map_err(|e| {
            ContractError::Std(StdError::GenericErr {
                msg: format!("pair-registry/contract.rs:100 {}", e),
            })
        }),
        ExecuteMsg::UpdateLegacyOwner { new_owner } => {
            let valid_new_owner = deps.api.addr_validate(&new_owner)?;
            update_legacy_owner(deps, env, info, valid_new_owner)
        }
    }
}

pub fn upsert_pair_addrs(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token0: String,
    token1: String,
    routes: Vec<PairContract>,
) -> Result<Response, ContractError> {
    let mut cfg = STATE.load(deps.storage)?;
    ensure!(
        Some(info.sender.to_string()) == LEGACY_OWNER.load(deps.storage)?,
        ContractError::Auth(AuthorizationError::UnauthorizedInfo(
            LEGACY_OWNER.load(deps.storage)?.unwrap(),
            Some(info.sender.to_string()),
            macros::loc_string!()
        ))
    );
    cfg.pair_contracts
        .upsert_pair_contracts(token0, token1, routes)?;
    STATE.save(deps.storage, &cfg)?;
    Ok(Response::new().add_attribute("action", "upsert_pair_contract"))
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::SwapRoute { token0, token1 } => to_binary(
            &query_swap_route(deps, token0, token1)
                .map_err(|e| StdError::generic_err(format!("{:?}", e)))?,
        ),
        QueryMsg::LegacyOwner {} => to_binary(&query_legacy_owner(deps)?),
    }
}

pub fn query_legacy_owner(deps: Deps) -> StdResult<LegacyOwnerResponse> {
    let legacy_owner = LEGACY_OWNER.load(deps.storage)?;
    let legacy_owner = match legacy_owner {
        Some(legacy_owner) => legacy_owner,
        None => "No owner".to_string(),
    };
    Ok(LegacyOwnerResponse { legacy_owner })
}

pub fn query_swap_route(
    deps: Deps,
    token0: String,
    token1: String,
) -> Result<SwapRouteResponse, ContractError> {
    let cfg = STATE.load(deps.storage)?;
    let (swap_route, _reverse) = cfg.pair_contracts.get_swap_route((token0, token1))?;
    Ok(SwapRouteResponse {
        pair_routes: swap_route.routes,
    })
}
