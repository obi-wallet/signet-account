// === Imports Start ===
macros::cosmwasm_imports!(
    ensure,
    to_binary,
    Binary,
    Deps,
    DepsMut,
    Env,
    MessageInfo,
    QueryRequest,
    Response,
    StdError,
    StdResult,
    WasmQuery
);
use crate::state::{State, STATE};
use classes::{
    asset_unifier::{
        DefaultUnifiedAssetResponse, ExecuteMsg, InstantiateMsg, LegacyOwnerResponse, QueryMsg,
        UnifiedAssetsResponse, UnifyAssetsMsg,
    },
    debtkeeper::MigrateMsg,
    gatekeeper_common::{ensure_authorized, update_legacy_owner, LEGACY_OWNER},
    pair_contract::SwapRouteResponse,
    pair_registry::QueryMsg as PairRegistryQueryMsg,
    permissioned_address::CoinBalance,
    sourced_coin::SourcedCoins,
    sources::Sources,
};
use common::{
    coin256::Coin256,
    common::get_axlusdc_ibc_denom,
    common_error::{ContractError, SpendLimitError, UnifierError},
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
    LEGACY_OWNER.save(deps.storage, &msg.legacy_owner)?;
    #[cfg(feature = "cosmwasm")]
    let cfg = State {
        default_asset_unifier: msg.default_asset_unifier,
        home_network: msg.home_network,
        pair_contract_registry: msg.pair_contract_registry,
    };
    #[cfg(feature = "secretwasm")]
    let cfg = State {
        default_asset_unifier: msg.default_asset_unifier,
        home_network: msg.home_network,
        pair_contract_registry: msg.pair_contract_registry,
        pair_contract_registry_code_hash: msg.pair_contract_registry_code_hash,
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
    match msg {
        ExecuteMsg::UpdateLegacyOwner { new_owner } => {
            let valid_new_owner = deps.api.addr_validate(&new_owner)?;
            update_legacy_owner(deps, env, info, valid_new_owner)
        }
        ExecuteMsg::UpdatePairRegistry { addr } => {
            ensure_authorized(deps.as_ref(), env, info.sender, None, macros::loc_string!())?;
            let mut cfg: State = STATE.load(deps.storage)?;
            cfg.pair_contract_registry = addr;
            STATE.save(deps.storage, &cfg).map_err(ContractError::Std)?;
            Ok(Response::default())
        }
    }
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::LegacyOwner {} => to_binary(&query_legacy_owner(deps)?),
        QueryMsg::UnifyAssets(UnifyAssetsMsg {
            target_asset,
            assets,
            assets_are_target_amount,
        }) => to_binary(
            &unify_assets(
                deps,
                env.clone(),
                target_asset.unwrap_or_else(|| get_axlusdc_ibc_denom(env.block.chain_id)),
                assets,
                assets_are_target_amount,
            )
            .map_err(|e| StdError::generic_err(format!("{:?}", e)))?,
        ),
        QueryMsg::DefaultUnifiedAsset {} => to_binary(&default_asset_unifier(deps)?),
    }
}
// === Entry Points End ===

pub fn default_asset_unifier(deps: Deps) -> StdResult<DefaultUnifiedAssetResponse> {
    let default_asset_unifier = STATE.load(deps.storage)?.default_asset_unifier;
    Ok(DefaultUnifiedAssetResponse {
        default_asset_unifier,
    })
}

pub fn query_legacy_owner(deps: Deps) -> StdResult<LegacyOwnerResponse> {
    let legacy_owner = LEGACY_OWNER.load(deps.storage)?;
    let legacy_owner = match legacy_owner {
        Some(legacy_owner) => legacy_owner,
        None => "No owner".to_string(),
    };
    Ok(LegacyOwnerResponse { legacy_owner })
}

fn is_hex_string(s: String) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn unify_assets(
    deps: Deps,
    env: Env,
    target_asset: String,
    assets: Vec<Coin256>,
    assets_are_target_amount: bool,
) -> Result<UnifiedAssetsResponse, ContractError> {
    let pair_route_registry = STATE.load(deps.storage)?.pair_contract_registry;
    #[cfg(feature = "secretwasm")]
    let pair_route_registry_code_hash = STATE.load(deps.storage)?.pair_contract_registry_code_hash;
    let mut return_coin = SourcedCoins {
        coins: vec![],
        wrapped_sources: Sources { sources: vec![] },
    };

    for asset in assets {
        // ignore case so that ETH address lower/upper/checksum doesn't matter
        // perhaps checksum support later
        if asset.denom.clone().eq_ignore_ascii_case(&target_asset)
            || asset.denom.clone().eq_ignore_ascii_case(&target_asset[2..])
        {
            return_coin.coins.push(CoinBalance {
                denom: asset.denom.clone(),
                amount: asset.amount,
                spent_this_inheritance_period: None,
                limit_remaining: 0u128.into(),
            });
        } else if is_hex_string(asset.denom.clone()) {
            deps.api.debug(&format!(
                "trying to convert {} to {}",
                asset.denom, target_asset
            ));
            return Err(ContractError::Unifier(
                UnifierError::EthPriceConversionError {},
            ));
        } else {
            let query_msg = PairRegistryQueryMsg::SwapRoute {
                token0: asset.denom.clone(),
                token1: target_asset.clone(),
            };
            deps.api.debug("Inter-contract query: \x1b[1;34mAsset Unifer\x1b[0m querying \x1b[1;34mPair Registry\x1b[0m");

            let query_response: Result<SwapRouteResponse, StdError> =
                deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: pair_route_registry.clone(),
                    #[cfg(feature = "secretwasm")]
                    code_hash: pair_route_registry_code_hash.clone(),
                    msg: to_binary(&query_msg)?,
                }));

            let pair_contract = match query_response {
                Ok(res) => res.pair_routes[0].clone(),
                Err(e) => {
                    return Err(ContractError::SpendLimit(
                        SpendLimitError::PriceCheckFailed(
                            format!("Price check loc 1: {:?}", to_binary(&query_msg)?),
                            pair_route_registry,
                            e.to_string(),
                        ),
                    ));
                }
            };

            let reverse: bool;
            let reverse_message_type: bool;

            let price_response = match pair_contract.query_format {
                // osmosis
                #[cfg(feature = "cosmwasm")]
                PairMessageType::OsmoType => {
                    match &asset.denom {
                        val if val == &pair_contract.token0 => {
                            reverse = false;
                            reverse_message_type = false;
                        }
                        val if val == &pair_contract.token1 => {
                            reverse = true;
                            reverse_message_type = false;
                        }
                        _ => {
                            return Err(ContractError::Std(StdError::generic_err(format!(
                                "Asset {} not found in OsmoPool {}",
                                asset.denom, pair_contract.identifier
                            ))));
                        }
                    }

                    pair_contract
                        .query_contract(
                            deps,
                            env.clone(),
                            asset.amount,
                            reverse,
                            assets_are_target_amount,
                            reverse_message_type,
                        )
                        .map_err(|e| {
                            ContractError::SpendLimit(SpendLimitError::PriceCheckFailed(
                                format!("Price check loc 3: {:?}", to_binary(&query_msg).unwrap()),
                                pair_route_registry.clone(),
                                e.to_string(),
                            ))
                        })?
                }
                _ => {
                    if pair_contract.token0 == asset.denom.clone() {
                        reverse = false;
                        reverse_message_type = false;
                    } else {
                        reverse = true;
                        reverse_message_type = true;
                    }

                    pair_contract
                        .query_contract(
                            deps,
                            env.clone(),
                            asset.amount,
                            reverse,
                            assets_are_target_amount,
                            reverse_message_type,
                        )
                        .map_err(|e| {
                            ContractError::SpendLimit(SpendLimitError::PriceCheckFailed(
                                format!("Price check loc 2: {:?}", to_binary(&query_msg).unwrap()),
                                pair_route_registry.clone(),
                                e.to_string(),
                            ))
                        })?
                }
            };

            return_coin.coins.push(price_response.coins[0].clone());
            return_coin.wrapped_sources = price_response.wrapped_sources.clone();
        }
    }

    Ok(UnifiedAssetsResponse {
        asset_unifier: return_coin.coins[0].clone().to_coin(),
        sources: return_coin.wrapped_sources,
    })
}
