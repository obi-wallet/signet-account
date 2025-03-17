use common::common_error::ContractError;

#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
macros::cosmwasm_imports!(
    to_binary,
    Attribute,
    Deps,
    QueryRequest,
    StdError,
    StdResult,
    Uint256,
    WasmQuery,
);

use crate::{
    asset_unifier::{self, DefaultUnifiedAssetResponse, UnifiedAssetsResponse, UnifyAssetsMsg},
    permissioned_address::CoinBalance,
    sources::{Source, Sources},
};

use common::coin256::Coin256;

/// `SourcedCoins` represents the coins and wrapped sources
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
pub struct SourcedCoins {
    pub coins: Vec<CoinBalance>,
    pub wrapped_sources: Sources,
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
pub enum UnifyAssetsQueryMsg {
    UnifyAssets(UnifyAssetsMsg),
}

/// Return admin sourced coins
pub fn get_admin_sourced_coin() -> SourcedCoins {
    // TODO: Replace these placeholder values with actual values in the future if needed?
    SourcedCoins {
        coins: vec![CoinBalance {
            denom: String::from("unlimited"),
            amount: Uint256::from(0u128),
            spent_this_inheritance_period: None,
            limit_remaining: Uint256::from(u128::MAX),
        }],
        wrapped_sources: Sources {
            sources: [Source {
                contract_addr: String::from("no spend limit check"),
                query_msg: String::from("caller is admin"),
            }]
            .to_vec(),
        },
    }
}

impl SourcedCoins {
    pub fn get_asset_unifier_default(
        &self,
        deps: Deps,
        asset_unifier_contract_address: String,
        #[cfg(feature = "secretwasm")] asset_unifier_code_hash: String,
    ) -> Result<String, ContractError> {
        // query the asset unifier to get its default unified asset
        let unified_denom: Result<DefaultUnifiedAssetResponse, StdError> =
            deps.querier.query_wasm_smart(
                #[cfg(feature = "secretwasm")]
                asset_unifier_code_hash,
                asset_unifier_contract_address,
                &asset_unifier::QueryMsg::DefaultUnifiedAsset {},
            );
        if let Err(e) = unified_denom {
            return Err(ContractError::Std(StdError::generic_err(format!(
                "{}, {}",
                macros::loc_string!(),
                e
            ))));
        }
        Ok(unified_denom?.default_asset_unifier)
    }

    pub fn convert_to_base_or_target_asset(
        &self,
        deps: Deps,
        asset_unifier_contract_address: String,
        #[cfg(feature = "secretwasm")] asset_unifier_code_hash: String,
        amount_is_target: bool,
        _chain_id: String,
        target_asset: Option<String>,
    ) -> Result<UnifiedAssetsResponse, ContractError> {
        let denom = match target_asset {
            Some(asset) => asset,
            None => self
                .get_asset_unifier_default(
                    deps,
                    asset_unifier_contract_address.clone(),
                    #[cfg(feature = "secretwasm")]
                    asset_unifier_code_hash.clone(),
                )
                .unwrap_or(String::from("uscrt")),
        };
        if self.coins.len() == 1 && self.coins[0].denom == *denom {
            return Ok(UnifiedAssetsResponse {
                asset_unifier: Coin256::new(self.coins[0].amount, denom),
                sources: Sources { sources: vec![] },
            });
        }

        // local single contract test uses test assets worth 100 USDC each
        if asset_unifier_contract_address == *"LOCAL_TEST" {
            let multiplier: Uint256;
            let divisor: Uint256;
            if self.coins[0].denom == *denom {
                divisor = Uint256::from(1u128);
                multiplier = Uint256::from(1u128);
            } else if amount_is_target {
                divisor = Uint256::from(100u128);
                multiplier = Uint256::from(1u128);
            } else {
                divisor = Uint256::from(1u128);
                multiplier = Uint256::from(100u128);
            }
            let converted_res = Ok(UnifiedAssetsResponse {
                asset_unifier: Coin256 {
                    denom: if !amount_is_target {
                        denom
                    } else {
                        self.coins[0].denom.clone()
                    },
                    amount: self.coins[0]
                        .amount
                        .checked_mul(multiplier)?
                        .checked_div(divisor)
                        .map_err(|_| StdError::generic_err("failed to convert to usdc"))?,
                },
                sources: Sources { sources: vec![] },
            });
            return converted_res;
        }

        let query_msg: UnifyAssetsQueryMsg = UnifyAssetsQueryMsg::UnifyAssets(UnifyAssetsMsg {
            target_asset: Some(denom),
            assets: self
                .coins
                .clone()
                .into_iter()
                .map(|c| c.to_coin())
                .collect(),
            assets_are_target_amount: amount_is_target,
        });

        deps.api.debug("Inter-contract query: \x1b[1;34mUser Account or Spendlimit Gatekeeper\x1b[0m querying \x1b[1;34mAsset Unifier\x1b[0m");
        // deps.api.debug("encoded message: {:?}", to_binary(&query_msg)?);
        let query_response: StdResult<UnifiedAssetsResponse> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: asset_unifier_contract_address,
                #[cfg(feature = "secretwasm")]
                code_hash: asset_unifier_code_hash,
                msg: to_binary(&query_msg)?,
            }));
        if let Err(e) = query_response {
            return Err(ContractError::Std(StdError::generic_err(format!(
                "{}, {}",
                macros::loc_string!(),
                e
            ))));
        }
        Ok(query_response?)
    }

    pub fn sources_as_attributes(&self) -> Vec<Attribute> {
        let mut attributes: Vec<Attribute> = vec![];
        for n in 0..self.wrapped_sources.sources.len() {
            attributes.push(Attribute {
                key: format!(
                    "query to contract {}",
                    self.wrapped_sources.sources[n].contract_addr.clone()
                ),
                value: self.wrapped_sources.sources[n].query_msg.clone(),
                #[cfg(feature = "secretwasm")]
                encrypted: false,
            })
        }
        attributes
    }
}
