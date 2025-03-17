#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(
    to_binary,
    Deps,
    Env,
    QueryRequest,
    StdError,
    Uint256,
    WasmQuery
);
#[allow(deprecated)]
#[cfg(feature = "cosmwasm")]
use osmosis_std::types::osmosis::gamm::v1beta1::QuerySwapExactAmountInResponse;
// migrate to twap as soon as it's working
#[allow(unused_imports)]
#[cfg(feature = "cosmwasm")]
use osmosis_std::types::osmosis::twap::v1beta1::ArithmeticTwapToNowResponse;
use serde::Deserialize;

use crate::permissioned_address::CoinBalance;
use crate::simulation::{
    ReverseSimulationResponse, SimulationResponse, UnifiedPriceResponse, UnifyPriceResponse,
};
use crate::sourced_coin::SourcedCoins;
#[cfg(test)]
use crate::tests_constants::get_test_sourced_coin;
use crate::{
    simulation::{DexQueryMsg, Token1ForToken2PriceResponse, Token2ForToken1PriceResponse},
    simulation::{DexQueryMsgFormatted, DexQueryMsgType, FormatQueryMsg, Tally},
};
use common::common_error::{ContractError, SpendLimitError, SwapError};

use crate::sources::{Source, Sources};

#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
#[cfg_attr(
    feature = "secretwasm",
    derive(
        serde::Serialize,
        serde::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        schemars::JsonSchema
    )
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
pub struct ArithmeticTwapToNow {
    id: u64,
    quote_asset_denom: String,
    base_asset_denom: String,
    start_time: i64,
}

#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
#[cfg_attr(
    feature = "secretwasm",
    derive(
        serde::Serialize,
        serde::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        schemars::JsonSchema
    )
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
pub enum PairMessageType {
    TerraswapType, //also used by Phoenix, Terraswap, Loop
    JunoType,
    #[cfg(feature = "cosmwasm")]
    OsmoType,
}

#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
#[cfg_attr(
    feature = "secretwasm",
    derive(
        serde::Serialize,
        serde::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        schemars::JsonSchema
    )
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
pub enum PairContractType {
    Wasm,
    Osmo,
}

#[cfg(feature = "cosmwasm")]
#[cfg_attr(not(feature = "secretwasm"), cw_serde)]
pub struct PairContract {
    pub identifier: String,
    pub token0: String,
    pub token1: String,
    pub chain_id: String,
    pub query_format: PairMessageType,
}

#[cfg(feature = "secretwasm")]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
pub struct PairContract {
    pub identifier: String,
    pub token0: String,
    pub token1: String,
    pub chain_id: String,
    pub query_format: PairMessageType,
    pub code_hash: String,
}

#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
#[cfg_attr(
    feature = "secretwasm",
    derive(
        serde::Serialize,
        serde::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        schemars::JsonSchema
    )
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
pub struct OsmoPool {
    pub pool_id: u64,
    pub token0: String,
    pub token1: String,
    pub chain_id: String,
    pub query_format: PairMessageType,
}

#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
#[cfg_attr(
    feature = "secretwasm",
    derive(
        serde::Serialize,
        serde::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        schemars::JsonSchema
    )
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
pub struct SwapRoute {
    pub token0: String,
    pub token1: String,
    pub routes: Vec<PairContract>,
}

#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
#[cfg_attr(
    feature = "secretwasm",
    derive(
        serde::Serialize,
        serde::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        schemars::JsonSchema
    )
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
pub struct SwapRouteResponse {
    pub pair_routes: Vec<PairContract>,
}

#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
#[cfg_attr(
    feature = "secretwasm",
    derive(
        serde::Serialize,
        serde::Deserialize,
        Clone,
        Debug,
        Eq,
        PartialEq,
        schemars::JsonSchema
    )
)]
#[cfg_attr(feature = "secretwasm", serde(rename_all = "snake_case"))]
pub struct PairContracts {
    pub swap_routes: Vec<SwapRoute>,
}

impl PairContracts {
    pub fn get_swap_route(
        &self,
        tokens: (String, String),
    ) -> Result<(SwapRoute, bool), ContractError> {
        for n in 0..self.swap_routes.len() {
            if self.swap_routes[n].token0 == tokens.0 && self.swap_routes[n].token1 == tokens.1 {
                return Ok((self.swap_routes[n].clone(), false));
            } else if self.swap_routes[n].token1 == tokens.0
                && self.swap_routes[n].token0 == tokens.1
            {
                return Ok((self.swap_routes[n].clone(), true));
            }
        }
        Err(ContractError::Swap(SwapError::PairContractNotFound(
            tokens.0, tokens.1,
        )))
    }

    pub fn upsert_pair_contracts(
        &mut self,
        token0: String,
        token1: String,
        routes: Vec<PairContract>,
    ) -> Result<bool, StdError> {
        for n in 0..self.swap_routes.len() {
            if (self.swap_routes[n].token0 == token0 && self.swap_routes[n].token1 == token1)
                || (self.swap_routes[n].token0 == token1 && self.swap_routes[n].token1 == token0)
            {
                self.swap_routes[n].routes = routes;
                return Ok(true);
            }
        }
        self.swap_routes.push(SwapRoute {
            token0,
            token1,
            routes,
        });
        Ok(true)
    }
}

impl PairContract {
    /// returns this pair contract's tokens as (String, String)
    pub fn get_tokens(&self) -> Result<(String, String), ContractError> {
        Ok((self.token0.clone(), self.token1.clone()))
    }

    pub fn get_denoms(&self) -> Result<(String, String), ContractError> {
        Ok((self.token0.clone(), self.token1.clone()))
    }

    #[allow(unreachable_code)]
    #[allow(unused_variables)]
    pub fn query_contract(
        self,
        deps: Deps,
        env: Env,
        amount: Uint256,
        reverse: bool,
        amount_is_target: bool,
        reverse_message_type: bool,
    ) -> Result<SourcedCoins, ContractError> {
        let mut flip_assets: bool = amount_is_target;

        if reverse_message_type || reverse {
            flip_assets = !flip_assets;
        }

        // Todo: figure out what this does
        let query_msg = self.clone().create_query_msg(amount, flip_assets)?;

        // TODO: Move this out of here!
        #[cfg(test)]
        {
            let test_denom1 = match flip_assets {
                true => self.token1,
                false => self.token0,
            };
            return get_test_sourced_coin((test_denom1, query_msg.1), amount, reverse)
                .map_err(ContractError::Std);
        }

        let query_result: Result<SourcedCoins, ContractError>;

        match flip_assets {
            false => match self.query_format {
                PairMessageType::TerraswapType => {
                    self.process_query::<SimulationResponse>(deps, &query_msg.0, query_msg.1, None)
                }
                PairMessageType::JunoType => self.process_query::<Token1ForToken2PriceResponse>(
                    deps,
                    &query_msg.0,
                    query_msg.1,
                    None,
                ),
                #[allow(deprecated)]
                #[cfg(feature = "cosmwasm")]
                PairMessageType::OsmoType => self.process_query::<QuerySwapExactAmountInResponse>(
                    deps,
                    &query_msg.0,
                    query_msg.1,
                    Some(amount),
                ),
            },
            true => match self.query_format {
                PairMessageType::TerraswapType => self.process_query::<ReverseSimulationResponse>(
                    deps,
                    &query_msg.0,
                    query_msg.1,
                    None,
                ),
                PairMessageType::JunoType => self.process_query::<Token2ForToken1PriceResponse>(
                    deps,
                    &query_msg.0,
                    query_msg.1,
                    None,
                ),
                #[allow(deprecated)]
                #[cfg(feature = "cosmwasm")]
                PairMessageType::OsmoType => self.process_query::<QuerySwapExactAmountInResponse>(
                    deps,
                    &query_msg.0,
                    query_msg.1,
                    Some(amount),
                ),
            },
        }
    }

    pub fn create_query_msg(
        self,
        amount: Uint256,
        flip_assets: bool,
    ) -> Result<(DexQueryMsgFormatted, String), ContractError> {
        let response_asset: String;

        Ok(match self.query_format {
            PairMessageType::TerraswapType => {
                let dex_query_msg = DexQueryMsg {
                    ty: DexQueryMsgType::Simulation,
                    denom: self.token0.clone(),
                    amount,
                    pool: None,
                    out_denom: None,
                };

                response_asset = self.token1;

                (dex_query_msg.format_query_msg(flip_assets)?, response_asset)
            }

            PairMessageType::JunoType => {
                let dex_query_msg = DexQueryMsg {
                    ty: DexQueryMsgType::Token1ForToken2Price,
                    denom: self.token0.clone(), // unused by juno type
                    amount,
                    pool: None,
                    out_denom: None,
                };

                let response_asset = match flip_assets {
                    false => self.token1,
                    true => self.token0,
                    // no cw20 support yet (except for the base asset)
                };

                (dex_query_msg.format_query_msg(flip_assets)?, response_asset)
            }
            #[cfg(feature = "cosmwasm")]
            PairMessageType::OsmoType => {
                let dex_query_msg = DexQueryMsg {
                    ty: if flip_assets {
                        DexQueryMsgType::OsmosisEstimateOut
                    } else {
                        DexQueryMsgType::OsmosisEstimateIn
                    },
                    denom: self.token0.clone(),
                    amount,
                    pool: Some(self.identifier.parse::<u64>().unwrap()),
                    out_denom: Some(if flip_assets {
                        self.token1.clone()
                    } else {
                        self.token0.clone()
                    }),
                };

                let response_asset = match flip_assets {
                    false => self.token1,
                    true => self.token0,
                    // no cw20 support yet (except for the base asset)
                };

                (dex_query_msg.format_query_msg(flip_assets)?, response_asset)
            }
        })
    }

    // useful for TWAP later
    #[allow(dead_code)]
    fn slice_out_denom(&self, s: String) -> String {
        let pos = s
            .char_indices()
            .find_map(|(i, c)| if c.is_numeric() { None } else { Some(i) })
            .unwrap_or(s.len());

        s[pos..].to_string()
    }

    fn process_query<T>(
        &self,
        deps: Deps,
        query_msg: &DexQueryMsgFormatted,
        response_asset: String,
        wrap_amount: Option<Uint256>,
    ) -> Result<SourcedCoins, ContractError>
    where
        T: for<'de> Deserialize<'de>,
        T: UnifyPriceResponse + Tally,
    {
        deps.api.debug("Inter-contract query: \x1b[1;34mAsset Unifer\x1b[0m querying \x1b[1;34mDex Contract\x1b[0m");
        // deps.api.debug("encoded message: {:?}", to_binary(&query_msg)?);
        let query_response: Result<T, StdError> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.identifier.clone(),
                msg: to_binary(query_msg)?,
                #[cfg(feature = "secretwasm")]
                code_hash: self.code_hash.clone(),
            }));
        let maybe_wrapped_response: Result<UnifiedPriceResponse, StdError> =
            query_response.map(|r| r.unify_price_response(wrap_amount));

        match maybe_wrapped_response {
            Ok(res) => Ok(SourcedCoins {
                coins: vec![CoinBalance {
                    denom: response_asset,
                    amount: Uint256::from(res.tally()),
                    spent_this_inheritance_period: None,
                    limit_remaining: Uint256::from(0u128),
                }],
                wrapped_sources: Sources {
                    sources: vec![Source {
                        contract_addr: self.identifier.clone(),
                        query_msg: format!("{:?}", to_binary(&query_msg)?),
                    }],
                },
            }),
            Err(e) => Err(ContractError::SpendLimit(
                SpendLimitError::PriceCheckFailed(
                    format!("Price check loc 4: {:?}", to_binary(&query_msg)?),
                    self.identifier.clone(),
                    e.to_string(),
                ),
            )),
        }
    }
}
