#[cfg(test)]
mod tests {
    macros::cosmwasm_imports!(from_binary, Addr, Env);
    macros::cosmwasm_testing_imports!(
        mock_dependencies,
        mock_dependencies_with_balance,
        mock_env,
        mock_info
    );

    use crate::{
        contract::{execute, instantiate, query},
        msg::{ExecuteMsg, FeeDetailsResponse, InstantiateMsg, QueryMsg},
    };

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&[]);

        let msg = InstantiateMsg {
            fee_divisors: ("chain1".to_string(), 1000u64),
            fee_pay_addresses: ("chain1".to_string(), "fee_repay_address".to_string()),
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn update_fee_pay_address() {
        let mut deps = mock_dependencies_with_balance(&[]);

        let msg = InstantiateMsg {
            fee_divisors: ("chain1".to_string(), 1000u64),
            fee_pay_addresses: ("chain1".to_string(), "fee_repay_address".to_string()),
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // query fee pay addresses
        let query_msg = crate::msg::QueryMsg::FeeDetails {
            chain_id: "chain1".to_string(),
        };
        let res = query(deps.as_ref(), mock_env(), query_msg);
        let unwrapped_res: FeeDetailsResponse = from_binary(&res.unwrap()).unwrap();
        assert_eq!(
            unwrapped_res.fee_pay_address,
            "fee_repay_address".to_string()
        );

        let update_msg = crate::msg::ExecuteMsg::SetFeeAddress {
            chain_id: "chain1".to_string(),
            new_fee_address: "fee_repay_address2".to_string(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, update_msg).unwrap();

        let query_msg = crate::msg::QueryMsg::FeeDetails {
            chain_id: "chain1".to_string(),
        };
        let res = query(deps.as_ref(), mock_env(), query_msg);
        let unwrapped_res: FeeDetailsResponse = from_binary(&res.unwrap()).unwrap();
        assert_eq!(
            unwrapped_res.fee_pay_address,
            "fee_repay_address2".to_string()
        );
    }

    #[test]
    fn non_creator_cannot_update_fee_divisor() {
        let mut deps = mock_dependencies_with_balance(&[]);

        let msg = InstantiateMsg {
            fee_divisors: ("chain1".to_string(), 1000u64),
            fee_pay_addresses: ("chain1".to_string(), "fee_repay_address".to_string()),
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let bad_info = mock_info("badactor", &[]);
        let update_fee_msg = ExecuteMsg::SetFee {
            chain_id: "chain1".to_string(),
            new_fee_divisor: 1001u64,
        };
        let _res = execute(deps.as_mut(), mock_env(), bad_info, update_fee_msg).unwrap_err();
    }

    #[test]
    fn creator_cannot_update_more_than_1_bip() {
        let mut deps = mock_dependencies_with_balance(&[]);

        let msg: InstantiateMsg = InstantiateMsg {
            fee_divisors: ("chain1".to_string(), 1000u64),
            fee_pay_addresses: ("chain1".to_string(), "fee_repay_address".to_string()),
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let update_fee_msg = ExecuteMsg::SetFee {
            chain_id: "chain1".to_string(),
            new_fee_divisor: 499u64,
        };
        let _res = execute(deps.as_mut(), mock_env(), info, update_fee_msg).unwrap_err();
    }

    #[test]
    fn creator_can_update_1_bip() {
        let mut deps = mock_dependencies_with_balance(&[]);

        let msg = InstantiateMsg {
            fee_divisors: ("chain1".to_string(), 1000u64),
            fee_pay_addresses: ("chain1".to_string(), "fee_repay_address".to_string()),
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // after instantiate cannot update for 24 hours
        let mut future_env = mock_env();
        future_env.block.time = future_env.block.time.plus_seconds(86400);

        let update_fee_msg = ExecuteMsg::SetFee {
            chain_id: "chain1".to_string(),
            new_fee_divisor: 500u64,
        };
        let _res = execute(deps.as_mut(), future_env.clone(), info, update_fee_msg).unwrap();

        let query_msg = QueryMsg::FeeDetails {
            chain_id: "chain1".to_string(),
        };
        let res = query(deps.as_ref(), future_env, query_msg).unwrap();
        let res_de: FeeDetailsResponse = from_binary(&res).unwrap();
        assert!(res_de.fee_divisor == 500u64);
    }

    #[test]
    fn only_one_update_per_24_hours() {
        let mut deps = mock_dependencies_with_balance(&[]);

        let msg = InstantiateMsg {
            fee_divisors: ("chain1".to_string(), 1000u64),
            fee_pay_addresses: ("chain1".to_string(), "fee_repay_address".to_string()),
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // instantiate counts! update in 24 hours
        let mut future_env = mock_env();
        future_env.block.time = future_env.block.time.plus_seconds(86400);

        let update_fee_msg = ExecuteMsg::SetFee {
            chain_id: "chain1".to_string(),
            new_fee_divisor: 500u64,
        };
        let _res = execute(
            deps.as_mut(),
            future_env.clone(),
            info.clone(),
            update_fee_msg,
        )
        .unwrap();

        let update_fee_msg = ExecuteMsg::SetFee {
            chain_id: "chain1".to_string(),
            new_fee_divisor: 400u64,
        };
        let _res = execute(
            deps.as_mut(),
            future_env.clone(),
            info.clone(),
            update_fee_msg,
        )
        .unwrap_err();

        // doable in 24 hours
        future_env.block.time = future_env.block.time.plus_seconds(86400);
        let update_fee_msg = ExecuteMsg::SetFee {
            chain_id: "chain_id".to_string(),
            new_fee_divisor: 400u64,
        };
        let _res = execute(deps.as_mut(), future_env, info, update_fee_msg).unwrap();
    }
}
