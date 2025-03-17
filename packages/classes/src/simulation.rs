use std::convert::TryFrom;

#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(StdResult, Uint128, Uint256);
#[allow(deprecated)]
#[cfg(feature = "cosmwasm")]
use osmosis_std::types::osmosis::{
    gamm::v1beta1::QuerySwapExactAmountInResponse,
    poolmanager::v1beta1::{
        EstimateSinglePoolSwapExactAmountInRequest, EstimateSinglePoolSwapExactAmountOutRequest,
    },
    twap::v1beta1::ArithmeticTwapToNowResponse,
};

pub trait Tally {
    fn tally(self) -> Uint128;
}

pub trait UnifyPriceResponse {
    fn unify_price_response(self, amount: Option<Uint256>) -> UnifiedPriceResponse;
}

pub trait FormatQueryMsg {
    fn format_query_msg(self, reverse: bool) -> StdResult<DexQueryMsgFormatted>;
}

#[cfg(feature = "cosmwasm")]
#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
pub enum DexQueryMsgType {
    ReverseSimulation,
    Simulation,
    Token1ForToken2Price,
    Token2ForToken1Price,
    #[cfg(feature = "cosmwasm")]
    OsmosisEstimateIn,
    #[cfg(feature = "cosmwasm")]
    OsmosisEstimateOut,
}

#[cfg(feature = "secretwasm")]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
pub enum DexQueryMsgType {
    ReverseSimulation,
    Simulation,
    Token1ForToken2Price,
    Token2ForToken1Price,
}

#[cfg(feature = "cosmwasm")]
#[cfg_attr(all(feature = "cosmwasm", not(feature = "secretwasm")), cw_serde)]
pub enum DexQueryMsgFormatted {
    ReverseSimulation(ReverseSimulationMsg),
    Simulation(SimulationMsg),
    Token1ForToken2Price(Token1ForToken2Msg),
    Token2ForToken1Price(Token2ForToken1Msg),
    OsmosisEstimateIn(EstimateSinglePoolSwapExactAmountInRequest),
    OsmosisEstimateOut(EstimateSinglePoolSwapExactAmountOutRequest),
}

#[cfg(feature = "secretwasm")]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
pub enum DexQueryMsgFormatted {
    ReverseSimulation(ReverseSimulationMsg),
    Simulation(SimulationMsg),
    Token1ForToken2Price(Token1ForToken2Msg),
    Token2ForToken1Price(Token2ForToken1Msg),
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
pub struct UnifiedPriceResponse {
    pub amount: Uint256,
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
pub struct DexQueryMsg {
    pub ty: DexQueryMsgType,
    pub denom: String,
    pub amount: Uint256,
    pub out_denom: Option<String>,
    pub pool: Option<u64>,
}

impl FormatQueryMsg for DexQueryMsg {
    fn format_query_msg(self, reverse: bool) -> StdResult<DexQueryMsgFormatted> {
        let mut ty = self.ty;
        if reverse {
            ty = match ty {
                DexQueryMsgType::ReverseSimulation => DexQueryMsgType::Simulation,
                DexQueryMsgType::Simulation => DexQueryMsgType::ReverseSimulation,
                DexQueryMsgType::Token1ForToken2Price => DexQueryMsgType::Token2ForToken1Price,
                DexQueryMsgType::Token2ForToken1Price => DexQueryMsgType::Token1ForToken2Price,
                #[cfg(feature = "cosmwasm")]
                DexQueryMsgType::OsmosisEstimateIn => DexQueryMsgType::OsmosisEstimateOut,
                #[cfg(feature = "cosmwasm")]
                DexQueryMsgType::OsmosisEstimateOut => DexQueryMsgType::OsmosisEstimateIn,
            }
        }
        let res = match ty {
            DexQueryMsgType::ReverseSimulation => {
                DexQueryMsgFormatted::ReverseSimulation(ReverseSimulationMsg {
                    ask_asset: Asset {
                        info: AssetInfo::NativeToken { denom: self.denom },
                        amount: self.amount,
                    },
                })
            }
            DexQueryMsgType::Simulation => DexQueryMsgFormatted::Simulation(SimulationMsg {
                offer_asset: Asset {
                    info: AssetInfo::NativeToken { denom: self.denom },
                    amount: self.amount,
                },
            }),
            DexQueryMsgType::Token1ForToken2Price => {
                DexQueryMsgFormatted::Token1ForToken2Price(Token1ForToken2Msg {
                    token1_amount: Uint128::try_from(self.amount)?,
                })
            }
            DexQueryMsgType::Token2ForToken1Price => {
                DexQueryMsgFormatted::Token2ForToken1Price(Token2ForToken1Msg {
                    token2_amount: Uint128::try_from(self.amount)?,
                })
            }
            #[cfg(feature = "cosmwasm")]
            DexQueryMsgType::OsmosisEstimateIn => {
                let pool_id = self.pool.unwrap();
                DexQueryMsgFormatted::OsmosisEstimateIn(
                    EstimateSinglePoolSwapExactAmountInRequest {
                        pool_id,
                        token_in: format!("{}{}", self.amount, self.denom),
                        token_out_denom: self.out_denom.unwrap(),
                    },
                )
            }
            #[cfg(feature = "cosmwasm")]
            DexQueryMsgType::OsmosisEstimateOut => {
                let pool_id = self.pool.unwrap();
                DexQueryMsgFormatted::OsmosisEstimateOut(
                    EstimateSinglePoolSwapExactAmountOutRequest {
                        pool_id,
                        token_out: format!("{}{}", self.amount, self.out_denom.unwrap()),
                        token_in_denom: self.denom,
                    },
                )
            }
        };
        Ok(res)
    }
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
pub struct Token1ForToken2Msg {
    pub token1_amount: Uint128,
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
pub struct Token2ForToken1Msg {
    pub token2_amount: Uint128,
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
pub struct Token1ForToken2PriceResponse {
    pub token2_amount: Uint128,
}

impl UnifyPriceResponse for Token1ForToken2PriceResponse {
    fn unify_price_response(self, _amount: Option<Uint256>) -> UnifiedPriceResponse {
        UnifiedPriceResponse {
            amount: Uint256::from(self.token2_amount),
        }
    }
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
pub struct Token2ForToken1PriceResponse {
    pub token1_amount: Uint128,
}

impl UnifyPriceResponse for Token2ForToken1PriceResponse {
    fn unify_price_response(self, _amount: Option<Uint256>) -> UnifiedPriceResponse {
        UnifiedPriceResponse {
            amount: Uint256::from(self.token1_amount),
        }
    }
}

impl Tally for Token1ForToken2PriceResponse {
    fn tally(self) -> Uint128 {
        self.token2_amount
    }
}

impl Tally for Token2ForToken1PriceResponse {
    fn tally(self) -> Uint128 {
        self.token1_amount
    }
}

// raw unwrap, needs handling
#[cfg(feature = "cosmwasm")]
impl UnifyPriceResponse for ArithmeticTwapToNowResponse {
    fn unify_price_response(self, amount: Option<Uint256>) -> UnifiedPriceResponse {
        UnifiedPriceResponse {
            // maybe loses precision in Decimal -> Uint128 -> Uint256 conversion
            amount: amount
                .unwrap()
                .saturating_mul(Uint256::from(self.arithmetic_twap.parse::<u128>().unwrap())),
        }
    }
}

impl Tally for UnifiedPriceResponse {
    fn tally(self) -> Uint128 {
        Uint128::try_from(self.amount).unwrap()
    }
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
pub struct SimulationMsg {
    pub offer_asset: Asset,
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
pub struct Asset {
    pub amount: Uint256,
    pub info: AssetInfo,
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
pub enum AssetInfo {
    NativeToken { denom: String },
    Token { contract_addr: String },
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
pub struct ReverseSimulationMsg {
    pub ask_asset: Asset,
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
pub struct SimulationResponse {
    pub commission_amount: Uint128,
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
}

impl Tally for SimulationResponse {
    fn tally(self) -> Uint128 {
        self.commission_amount + self.return_amount
    }
}

impl UnifyPriceResponse for SimulationResponse {
    fn unify_price_response(self, _amount: Option<Uint256>) -> UnifiedPriceResponse {
        UnifiedPriceResponse {
            amount: Uint256::from(self.return_amount),
        }
    }
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
pub struct ReverseSimulationResponse {
    pub commission_amount: Uint128,
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
}

impl UnifyPriceResponse for ReverseSimulationResponse {
    fn unify_price_response(self, _amount: Option<Uint256>) -> UnifiedPriceResponse {
        UnifiedPriceResponse {
            amount: Uint256::from(self.offer_amount),
        }
    }
}

impl Tally for ReverseSimulationResponse {
    fn tally(self) -> Uint128 {
        self.commission_amount + self.offer_amount
    }
}

// deprecated; can switch to twap as soon as it is working. Support is still spotty.
#[allow(deprecated)]
#[cfg(feature = "cosmwasm")]
impl UnifyPriceResponse for QuerySwapExactAmountInResponse {
    fn unify_price_response(self, _amount: Option<Uint256>) -> UnifiedPriceResponse {
        UnifiedPriceResponse {
            amount: self.token_out_amount.parse::<u128>().unwrap().into(),
        }
    }
}

#[allow(deprecated)]
#[cfg(feature = "cosmwasm")]
impl Tally for QuerySwapExactAmountInResponse {
    fn tally(self) -> Uint128 {
        self.token_out_amount.parse::<u128>().unwrap().into()
    }
}
