pub const LEGACY_OWNER_STR: &str = "alice";

#[cfg(test)]
mod tests {

    use super::*;
    use crate::contract::{execute, instantiate, query};
    use classes::asset_unifier::LegacyOwnerResponse;
    use classes::gatekeeper_common::is_legacy_owner;
    use classes::pair_contract::{PairContract, PairMessageType, SwapRouteResponse};
    use classes::pair_registry::{ExecuteMsg, InstantiateMsg, QueryMsg};

    macros::cosmwasm_testing_imports!(mock_dependencies, mock_env, mock_info);
    macros::cosmwasm_imports!(from_binary, Api);

    const ANYONE: &str = "anyone";

    #[test]
    fn instantiate_and_modify_owner() {
        let mut deps = mock_dependencies();
        let current_env = mock_env();
        let _res = instantiate(
            deps.as_mut(),
            current_env,
            mock_info(LEGACY_OWNER_STR, &[]),
            InstantiateMsg {
                legacy_owner: Some(LEGACY_OWNER_STR.to_string()),
            },
        )
        .unwrap();

        // ensure expected config
        let expected = LegacyOwnerResponse {
            legacy_owner: LEGACY_OWNER_STR.to_string(),
        };
        assert!(is_legacy_owner(
            deps.as_ref(),
            deps.api.addr_validate(&expected.legacy_owner).unwrap()
        )
        .unwrap());
    }

    #[test]
    fn upsert_pair() {
        let mut deps = mock_dependencies();
        let current_env = mock_env();
        let _res = instantiate(
            deps.as_mut(),
            current_env,
            mock_info(LEGACY_OWNER_STR, &[]),
            InstantiateMsg {
                legacy_owner: Some(LEGACY_OWNER_STR.to_string()),
            },
        )
        .unwrap();

        let pair1 = PairContract {
            identifier: "noble1fd68ah02gr2y8ze7tm9te7m70zlmc7vjyyhs6xlhsdmqqcjud4dql4wpxr"
                .to_string(),
            token0: "uluna".to_string(),
            token1: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                .to_string(),
            chain_id: "4".to_string(),
            query_format: PairMessageType::JunoType,
            #[cfg(feature = "secretwasm")]
            code_hash: "dummy_hash".to_string(),
        };
        let pair2 = PairContract {
            identifier: "noble1ll68rj627k4g88v9q45pp64zwv8gg6x2v8ev68c570tzhq253gcsu74qrl"
                .to_string(),
            token0: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                .to_string(),
            token1: "ibc/BC8A77AFBD872FDC32A348D3FB10CC09277C266CFE52081DE341C7EC6752E674"
                .to_string(),
            chain_id: "4".to_string(),
            query_format: PairMessageType::JunoType,
            #[cfg(feature = "secretwasm")]
            code_hash: "dummy_hash".to_string(),
        };
        let routes = vec![pair1, pair2];

        let query_msg = QueryMsg::SwapRoute {
            token0: "uluna".to_string(),
            token1: "ibc/BC8A77AFBD872FDC32A348D3FB10CC09277C266CFE52081DE341C7EC6752E674"
                .to_string(),
        };
        let wrong_query_msg = QueryMsg::SwapRoute {
            token0: "ujuno".to_string(),
            token1: "ibc/BC8A77AFBD872FDC32A348D3FB10CC09277C266CFE52081DE341C7EC6752E674"
                .to_string(),
        };

        // non-operator cannot add pair
        let info = mock_info(ANYONE, &[]);
        let msg = ExecuteMsg::UpsertPair {
            token0: "uluna".to_string(),
            token1: "ibc/BC8A77AFBD872FDC32A348D3FB10CC09277C266CFE52081DE341C7EC6752E674"
                .to_string(),
            routes: routes.clone(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        let raw_res = query(deps.as_ref(), mock_env(), query_msg.clone());
        let _res = raw_res.unwrap_err();

        let info = mock_info(LEGACY_OWNER_STR, &[]);
        let msg = ExecuteMsg::UpsertPair {
            token0: "uluna".to_string(),
            token1: "ibc/BC8A77AFBD872FDC32A348D3FB10CC09277C266CFE52081DE341C7EC6752E674"
                .to_string(),
            routes,
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let raw_res = query(deps.as_ref(), mock_env(), query_msg);
        let _res: SwapRouteResponse = from_binary(&raw_res.unwrap()).unwrap();
        assert_eq!(_res.pair_routes.len(), 2);

        let raw_res = query(deps.as_ref(), mock_env(), wrong_query_msg);
        let _res = raw_res.unwrap_err();
    }
}
