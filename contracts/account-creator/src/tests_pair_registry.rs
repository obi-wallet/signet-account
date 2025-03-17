#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;
classes::cosmwasm_imports!(Addr, Uint128);
use classes::pair_contract::PairContract;
use classes::pair_contract::PairMessageType;
use cw_multi_test::App;
use cw_multi_test::Executor;

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

#[cfg_attr(feature = "cosmwasm", cw_serde)]
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
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
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
    serde(rename_all = "snake_case")
)]
struct PairData {
    protocol: String,
    name: String,
    asset_infos: Vec<AssetInfo>,
    contract_addr: String,
    liquidity_token: String,
    dex: String,
    dummyprice: Option<u128>,
}

#[cfg_attr(feature = "cosmwasm", cw_serde)]
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
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
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
    serde(rename_all = "snake_case")
)]
pub struct AssetInfo {
    pub token: Option<Token>,
    pub native_token: Option<NativeToken>,
}

#[cfg_attr(feature = "cosmwasm", cw_serde)]
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
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
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
    serde(rename_all = "snake_case")
)]
pub struct Token {
    pub contract_addr: String,
}

#[cfg_attr(feature = "cosmwasm", cw_serde)]
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
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
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
    serde(rename_all = "snake_case")
)]
pub struct NativeToken {
    pub denom: String,
}

#[cfg_attr(feature = "cosmwasm", cw_serde)]
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
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
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
    serde(rename_all = "snake_case")
)]
pub struct Pair {
    pub token0: String,
    pub token1: String,
    pub routes: Vec<PairContract>,
    pub dummyprice: Option<u128>,
}

fn read_file(file_path: &str) -> String {
    let mut file = File::open(file_path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    contents
}

pub fn process_pair_contracts_json(
    filename: String,
    router: &mut App,
    code_id: u64,
    mock_environment: bool,
    legacy_owner: String,
) -> Vec<Pair> {
    let file_contents = read_file(&format!("{}{}", "./src/", filename));
    // Deserialize the JSON data into a Vec of PairData objects
    let pair_data_vec = deserialize_json(&file_contents);
    assert!(!pair_data_vec.is_empty());
    // Build the pairs from the PairData objects and return them
    build_pairs(
        &pair_data_vec,
        router,
        code_id,
        mock_environment,
        legacy_owner,
    )
}

fn deserialize_json(json_str: &str) -> Vec<PairData> {
    let data: serde_json::Value = serde_json::from_str(json_str).unwrap();

    let mut pairs = Vec::new();

    for (pair_id, pair_data_json) in data.as_object().unwrap() {
        let mut pair_data = PairData {
            protocol: "".to_string(),
            name: "".to_string(),
            asset_infos: vec![],
            contract_addr: "".to_string(),
            liquidity_token: "".to_string(),
            dex: "".to_string(),
            dummyprice: None,
        };
        pair_data.contract_addr = pair_id.clone();

        if let Some(asset_infos_json) = pair_data_json.get("asset_infos") {
            for asset_info_json in asset_infos_json.as_array().unwrap() {
                let mut asset_info = AssetInfo {
                    token: None,
                    native_token: None,
                };

                if let Some(native_token_json) = asset_info_json.get("native_token") {
                    let denom = native_token_json
                        .get("denom")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string();
                    asset_info.native_token = Some(NativeToken { denom });
                } else if let Some(token_json) = asset_info_json.get("token") {
                    let contract_addr = token_json
                        .get("contract_addr")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string();
                    asset_info.token = Some(Token { contract_addr });
                }

                pair_data.asset_infos.push(asset_info);
            }
        }

        if let Some(dummy_price) = pair_data_json.get("dummyprice") {
            pair_data.dummyprice = Some(dummy_price.to_string().parse::<u128>().unwrap());
        }
        pairs.push(pair_data);
    }

    pairs
}

fn build_pairs(
    pair_data_vec: &[PairData],
    router: &mut App,
    code_id: u64,
    mock_environment: bool,
    legacy_owner: String,
) -> Vec<Pair> {
    let mut pairs = Vec::new();
    let mut pair_map: HashMap<(String, String), (Vec<PairContract>, u128)> = HashMap::new();

    for pair_data in pair_data_vec {
        let token0 = match &pair_data.asset_infos[0].native_token {
            Some(val) => val.denom.clone(),
            None => {
                pair_data.asset_infos[0]
                    .token
                    .clone()
                    .unwrap()
                    .contract_addr
            }
        };
        let token1 = match &pair_data.asset_infos[1].native_token {
            Some(val) => val.denom.clone(),
            None => {
                pair_data.asset_infos[1]
                    .token
                    .clone()
                    .unwrap()
                    .contract_addr
            }
        };
        let contract_addr = &pair_data.contract_addr;
        let chain_id = "phoenix-1";

        let matched_contract_addr: String = match mock_environment {
            true => {
                let init_msg = dummy_price_contract::msg::InstantiateMsg {
                    token0: token0.clone(),
                    token1: token1.clone(),
                    price: Uint128::from(pair_data.dummyprice.unwrap()),
                };
                // Instantiate the dummy price contract using its stored code id
                router
                    .instantiate_contract(
                        code_id,
                        Addr::unchecked(&legacy_owner),
                        &init_msg,
                        &[],
                        "dummy_price",
                        None,
                    )
                    .unwrap()
                    .to_string()
            }
            false => contract_addr.clone(),
        };

        let pair_contract = PairContract {
            identifier: matched_contract_addr.clone(),
            token0: token0.clone(),
            token1: token1.clone(),
            chain_id: chain_id.to_string(),
            query_format: PairMessageType::TerraswapType,
        };

        let pair_key = if token0 < token1 {
            (token0.clone(), token1.clone())
        } else {
            (token1.clone(), token0.clone())
        };

        if let Some(pair_contracts) = pair_map.get_mut(&pair_key) {
            pair_contracts.0.push(pair_contract);
        } else {
            let dummyprice = match mock_environment {
                true => pair_data.dummyprice.unwrap(),
                false => 0,
            };
            pair_map.insert(pair_key.clone(), (vec![pair_contract], dummyprice));
        }
    }

    for ((token0, token1), pair_contracts) in pair_map {
        let pair = Pair {
            token0: token0.clone(),
            token1: token1.clone(),
            routes: pair_contracts.0,
            dummyprice: Some(pair_contracts.1),
        };

        pairs.push(pair);
    }

    pairs
}
