#[cfg(test)]
mod tests {
    macros::cosmwasm_imports!(Uint256);
    use common::constants::{
        JUNO_MAINNET_DENOM, JUNO_MAINNET_DEX_DENOM, TERRA_MAINNET_AXLUSDC_IBC,
    };

    use classes::pair_contract::{PairContract, PairMessageType};
    use classes::simulation::{DexQueryMsg, DexQueryMsgType, FormatQueryMsg};

    #[test]
    fn pair_contract_get_denoms() {
        let test_pair_contract = PairContract {
            identifier: String::from(
                "juno1ctsmp54v79x7ea970zejlyws50cj9pkrmw49x46085fn80znjmpqz2n642",
            ),
            token0: String::from(JUNO_MAINNET_DENOM),
            token1: String::from(TERRA_MAINNET_AXLUSDC_IBC),
            query_format: PairMessageType::JunoType,
            chain_id: "local".to_string(),
            #[cfg(feature = "secretwasm")]
            code_hash: "dummy_hash".to_string(),
        };

        assert_eq!(
            test_pair_contract.get_denoms().unwrap(),
            (
                JUNO_MAINNET_DENOM.to_string(),
                TERRA_MAINNET_AXLUSDC_IBC.to_string()
            )
        );
    }

    #[test]
    fn pair_contract_create_loop_query_msg() {
        make_query_msg(
            false,
            ("testtokens".to_string(), JUNO_MAINNET_DEX_DENOM.to_string()),
            "testtokens".to_string(),
            PairMessageType::TerraswapType,
            Uint256::from(1_000_000u128),
        );
    }

    fn make_query_msg(
        flip_assets: bool,
        denoms: (String, String),
        expected_query_asset: String,
        ty: PairMessageType,
        amount: Uint256,
    ) {
        let test_pair_contract = PairContract {
            identifier: String::from(
                "juno1ctsmp54v79x7ea970zejlyws50cj9pkrmw49x46085fn80znjmpqz2n642",
            ),
            token0: denoms.0,
            token1: denoms.1,
            query_format: ty,
            chain_id: "local".to_string(),
            #[cfg(feature = "secretwasm")]
            code_hash: "dummy_hash".to_string(),
        };

        let query_msg = test_pair_contract
            .create_query_msg(amount, flip_assets)
            .unwrap();
        let dex_query_msg = DexQueryMsg {
            denom: expected_query_asset,
            amount,
            ty: DexQueryMsgType::Simulation,
            pool: None,
            out_denom: None,
        };
        assert_eq!(query_msg.0, dex_query_msg.format_query_msg(false).unwrap());
    }

    #[test]
    fn pair_contract_create_loop_reverse_query_msg() {
        let flip_assets = true; // going from dex tokens to test tokens
        let test_pair_contract = PairContract {
            identifier: String::from(
                "juno1ctsmp54v79x7ea970zejlyws50cj9pkrmw49x46085fn80znjmpqz2n642",
            ),
            token0: "testtokens".to_string(),
            token1: String::from(JUNO_MAINNET_DEX_DENOM),
            query_format: PairMessageType::TerraswapType,
            chain_id: "local".to_string(),
            #[cfg(feature = "secretwasm")]
            code_hash: "dummy_hash".to_string(),
        };

        let amount = Uint256::from(1_000_000u128);

        let query_msg = test_pair_contract
            .create_query_msg(amount, flip_assets)
            .unwrap();
        let test_msg = DexQueryMsg {
            ty: DexQueryMsgType::ReverseSimulation,
            denom: "testtokens".to_string(),
            amount,
            pool: None,
            out_denom: None,
        };
        assert_eq!(query_msg.0, test_msg.format_query_msg(false).unwrap());
    }

    #[test]
    fn pair_contract_create_juno_query_msg() {
        let flip_assets = false; // going from test tokens to usdc
        let test_pair_contract = PairContract {
            identifier: String::from(
                "juno1ctsmp54v79x7ea970zejlyws50cj9pkrmw49x46085fn80znjmpqz2n642",
            ),
            token0: "testtokens".to_string(),
            token1: String::from(TERRA_MAINNET_AXLUSDC_IBC),
            query_format: PairMessageType::JunoType,
            chain_id: "local".to_string(),
            #[cfg(feature = "secretwasm")]
            code_hash: "dummy_hash".to_string(),
        };

        let amount = Uint256::from(1_000_000u128);

        let query_msg = test_pair_contract
            .create_query_msg(amount, flip_assets)
            .unwrap();
        let dex_query_msg = DexQueryMsg {
            denom: "testtokens".to_string(),
            amount,
            ty: DexQueryMsgType::Token1ForToken2Price,
            pool: None,
            out_denom: None,
        };
        assert_eq!(query_msg.0, dex_query_msg.format_query_msg(false).unwrap());
    }

    #[test]
    fn pair_contract_create_juno_reverse_query_msg() {
        let flip_assets = true; // going from test tokens to usdc
        let test_pair_contract = PairContract {
            identifier: String::from(
                "juno1ctsmp54v79x7ea970zejlyws50cj9pkrmw49x46085fn80znjmpqz2n642",
            ),
            token0: "testtokens".to_string(),
            token1: String::from(TERRA_MAINNET_AXLUSDC_IBC),
            query_format: PairMessageType::JunoType,
            chain_id: "local".to_string(),
            #[cfg(feature = "secretwasm")]
            code_hash: "dummy_hash".to_string(),
        };

        let amount = Uint256::from(1_000_000u128);

        let query_msg = test_pair_contract
            .create_query_msg(amount, flip_assets)
            .unwrap();
        let dex_query_msg = DexQueryMsg {
            denom: "testtokens".to_string(),
            amount,
            ty: DexQueryMsgType::Token2ForToken1Price,
            pool: None,
            out_denom: None,
        };
        assert_eq!(query_msg.0, dex_query_msg.format_query_msg(false).unwrap());
    }
}
