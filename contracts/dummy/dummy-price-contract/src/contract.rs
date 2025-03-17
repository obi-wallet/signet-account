macros::cosmwasm_imports!(
    to_binary,
    Binary,
    Deps,
    DepsMut,
    Env,
    MessageInfo,
    Response,
    StdError,
    StdResult,
    Uint128,
);
#[cfg(feature = "cosmwasm")]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::msg::{
    Asset, AssetInfo, ExecuteMsg, InstantiateMsg, QueryMsg, ReverseSimulationResponse,
    SimulationResponse, Token1ForToken2Response, Token2ForToken1Response,
};
use crate::state::{State, STATE};

#[cfg(feature = "cosmwasm")]
use osmosis_std::types::osmosis::poolmanager::v1beta1::EstimateSwapExactAmountInResponse;

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        token0: msg.token0,
        token1: msg.token1,
        price: msg.price,
    };
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {}
}

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
#[cfg_attr(feature = "cosmwasm", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Simulation { offer_asset } => to_binary(&query_simulation(deps, offer_asset)?),
        QueryMsg::ReverseSimulation { ask_asset } => {
            to_binary(&query_reverse_simulation(deps, ask_asset)?)
        }
        QueryMsg::Token1ForToken2Price { token1_amount } => {
            let state = STATE.load(deps.storage)?;
            let res = juno_style_swap1(
                deps,
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: state.token0,
                    },
                    amount: token1_amount,
                },
            );
            to_binary(&res?)
        }
        QueryMsg::Token2ForToken1Price { token2_amount } => {
            let state = STATE.load(deps.storage)?;
            to_binary(&juno_style_swap2(
                deps,
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: state.token1,
                    },
                    amount: token2_amount,
                },
            )?)
        }
        // osmo
        #[cfg(feature = "cosmwasm")]
        QueryMsg::EstimateSinglePoolSwapExactAmountInRequest {
            pool_id: _,
            token_in,
            token_out_denom,
        } => to_binary(&osmo_style_swap(deps, token_in, token_out_denom)?),
    }
}

fn token0_to_token1(deps: Deps, token0_amount: Uint128) -> StdResult<Uint128> {
    let state: State = STATE.load(deps.storage)?;
    let amount = token0_amount.checked_mul(state.price)? / Uint128::from(1_000_000u128);
    Ok(amount)
}

fn token1_to_token0(deps: Deps, token1_amount: Uint128) -> StdResult<Uint128> {
    let state: State = STATE.load(deps.storage)?;
    token1_amount
        .checked_mul(Uint128::from(1_000_000u128))?
        .checked_div(state.price)
        .map_err(|e| StdError::generic_err(e.to_string()))
}

fn query_simulation(deps: Deps, offer_asset: Asset) -> StdResult<SimulationResponse> {
    let state: State = STATE.load(deps.storage)?;
    let denom = match offer_asset.info {
        AssetInfo::Token { contract_addr } => contract_addr,
        AssetInfo::NativeToken { denom } => denom,
    };
    let base_amount = if denom == state.token0 {
        token0_to_token1(deps, offer_asset.amount)?
    } else if denom == state.token1 {
        token1_to_token0(deps, offer_asset.amount)?
    } else {
        return Err(StdError::generic_err("Unrecognized asset"));
    };
    Ok(SimulationResponse {
        commission_amount: base_amount / Uint128::from(100u128),
        return_amount: base_amount.saturating_sub(base_amount / Uint128::from(100u128)),
        spread_amount: Uint128::from(100u128),
    })
}

fn juno_style_swap1(deps: Deps, known_asset: Asset) -> StdResult<Token1ForToken2Response> {
    let res = Token1ForToken2Response {
        token2_amount: token0_to_token1(deps, known_asset.amount)?,
    };
    Ok(res)
}

fn juno_style_swap2(deps: Deps, known_asset: Asset) -> StdResult<Token2ForToken1Response> {
    Ok(Token2ForToken1Response {
        token1_amount: token1_to_token0(deps, known_asset.amount)?,
    })
}

#[cfg(feature = "cosmwasm")]
fn osmo_style_swap(
    deps: Deps,
    token_in: String,
    token_out_denom: String,
) -> StdResult<EstimateSwapExactAmountInResponse> {
    let mut number_string = String::new();

    for c in token_in.chars() {
        if c.is_numeric() {
            number_string.push(c);
        } else {
            break;
        }
    }

    let number = Uint128::from(number_string.parse::<u128>().unwrap());
    let state: State = STATE.load(deps.storage)?;
    match token_out_denom {
        val if val == state.token0 => Ok(EstimateSwapExactAmountInResponse {
            token_out_amount: number
                .saturating_mul(Uint128::from(1_000_000u128))
                .checked_div(state.price)?
                .to_string(),
        }),
        val if val == state.token1 => Ok(EstimateSwapExactAmountInResponse {
            token_out_amount: number
                .saturating_mul(state.price)
                .checked_div(Uint128::from(1_000_000u128))?
                .to_string(),
        }),
        _ => Err(StdError::generic_err("Unrecognized base asset denom")),
    }
}

fn query_reverse_simulation(deps: Deps, ask_asset: Asset) -> StdResult<ReverseSimulationResponse> {
    let state: State = STATE.load(deps.storage)?;
    let denom = match ask_asset.info {
        AssetInfo::Token { contract_addr } => contract_addr,
        AssetInfo::NativeToken { denom } => denom,
    };
    let target_amount = if denom == state.token0 {
        token0_to_token1(deps, ask_asset.amount)?
    } else if denom == state.token1 {
        token1_to_token0(deps, ask_asset.amount)?
    } else {
        return Err(StdError::generic_err("Unrecognized asset"));
    };
    Ok(ReverseSimulationResponse {
        commission_amount: target_amount / Uint128::from(100u128),
        offer_amount: target_amount.saturating_sub(target_amount / Uint128::from(100u128)),
        spread_amount: Uint128::from(100u128),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    macros::cosmwasm_imports!(coins, Api);
    macros::cosmwasm_testing_imports!(mock_dependencies, mock_env, mock_info);
    use common::constants::TERRA_MAINNET_AXLUSDC_IBC;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            token0: "ujunox".to_owned(),
            token1: TERRA_MAINNET_AXLUSDC_IBC.to_string(),
            price: Uint128::from(157u128),
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn mock_simulation() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            token0: "ujunox".to_owned(),
            token1: TERRA_MAINNET_AXLUSDC_IBC.to_string(),
            price: Uint128::from(137_000_000u128),
        };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // query the prices
        let query_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: TERRA_MAINNET_AXLUSDC_IBC.to_owned(),
            },
            amount: Uint128::from(1_000_000u128),
        };
        let query = query_simulation(deps.as_ref(), query_asset).unwrap();
        println!("query gives {:?}", query);
        assert_eq!(query.commission_amount, Uint128::from(72u128));
        assert_eq!(query.return_amount, Uint128::from(7_227u128));
        assert_eq!(query.spread_amount, Uint128::from(100u128));

        // query reverse (juno)
        let query_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: "ujunox".to_owned(),
            },
            amount: Uint128::from(1_000_000u128),
        };
        let query = query_simulation(deps.as_ref(), query_asset).unwrap();
        println!("query reverse gives {:?}", query);
        assert_eq!(query.commission_amount, Uint128::from(1_370_000u128));
        assert_eq!(query.return_amount, Uint128::from(135_630_000u128));
        assert_eq!(query.spread_amount, Uint128::from(100u128));

        // query JunoSwap style (juno->usdc)
        let query_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: "ujunox".to_owned(),
            },
            amount: Uint128::from(2_000_000u128),
        };
        let query = juno_style_swap1(deps.as_ref(), query_asset).unwrap();
        println!("junoswap query gives {:?}", query);
        assert_eq!(query.token2_amount, Uint128::from(274_000_000u128));

        // query JunoSwap style (usdc->juno)
        let query_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                    .to_owned(),
            },
            amount: Uint128::from(20_000_000u128),
        };
        let query = juno_style_swap2(deps.as_ref(), query_asset).unwrap();
        deps.api
            .debug(&format!("junoswap query reverse gives {:?}", query));
        assert_eq!(query.token1_amount, Uint128::from(145_985u128));

        #[cfg(feature = "cosmwasm")]
        {
            // query osmo style (juno->usdc)
            let query = osmo_style_swap(
                deps.as_ref(),
                "2000000ujunox".to_string(),
                "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4".to_string(),
            )
            .unwrap();
            println!("osmoswap query (not reverse) gives {:?}", query);
            assert_eq!(
                query.token_out_amount,
                Uint128::from(274_000_000u128).to_string()
            );
        }

        #[cfg(feature = "cosmwasm")]
        {
            // query osmo style (usdc->juno)
            let query = osmo_style_swap(
                deps.as_ref(),
                "20000000ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                    .to_string(),
                "ujunox".to_string(),
            )
            .unwrap();
            println!("osmoswap query (not reverse) gives {:?}", query);
            assert_eq!(
                query.token_out_amount,
                Uint128::from(145_985u128).to_string()
            );
        }
    }
}
