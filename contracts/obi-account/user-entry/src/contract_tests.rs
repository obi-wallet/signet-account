mod tests {
    macros::cosmwasm_imports!(from_binary, to_binary, BankMsg, Coin, CosmosMsg, Uint128);
    macros::cosmwasm_testing_imports!(mock_dependencies, mock_env, mock_info);
    use classes::msg_user_entry::{
        ExecuteMsg, InstantiateMsg, QueryMsg, UserAccountAddressResponse,
    };
    #[allow(unused_imports)]
    use common::legacy_cosmosmsg as LegacyMsg;
    use common::universal_msg::UniversalMsg;

    use crate::contract::{execute, instantiate, query};

    #[test]
    fn successful_instantiation() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        let env = mock_env();

        #[cfg(feature = "cosmwasm")]
        let instantiate_msg = InstantiateMsg {
            user_account_address: "account1".to_string(),
        };
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        let instantiate_msg = InstantiateMsg {
            user_account_address: "account1".to_string(),
            user_account_code_hash: "hash1".to_string(),
        };

        let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn execute_universal_msg() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        let env = mock_env();

        #[cfg(feature = "cosmwasm")]
        let instantiate_msg = InstantiateMsg {
            user_account_address: "account1".to_string(),
        };
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        let instantiate_msg = InstantiateMsg {
            user_account_address: "account1".to_string(),
            user_account_code_hash: "hash1".to_string(),
        };

        instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

        // Test ExecuteMsg::Execute (bank send)
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        let execute_msg = ExecuteMsg::Execute {
            msg: to_binary(&UniversalMsg::Secret(CosmosMsg::Bank(BankMsg::Send {
                to_address: "account2".to_string(),
                amount: vec![Coin {
                    denom: "uscrt".to_string(),
                    amount: Uint128::from(100u128),
                }],
            })))
            .unwrap(),
            signatures: None,
        };
        #[cfg(feature = "cosmwasm")]
        let execute_msg = ExecuteMsg::Execute {
            msg: to_binary(&UniversalMsg::Legacy(LegacyMsg::CosmosMsg::Bank(
                LegacyMsg::BankMsg::Send {
                    to_address: "account2".to_string(),
                    amount: vec![Coin {
                        denom: "uscrt".to_string(),
                        amount: Uint128::from(100u128),
                    }],
                },
            )))
            .unwrap(),
            signatures: None,
        };
        let res = execute(deps.as_mut(), env, info, execute_msg).unwrap();
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn update_user_account_address() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        let env = mock_env();

        #[cfg(feature = "cosmwasm")]
        let instantiate_msg = InstantiateMsg {
            user_account_address: "account1".to_string(),
        };
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        let instantiate_msg = InstantiateMsg {
            user_account_address: "account1".to_string(),
            user_account_code_hash: "hash1".to_string(),
        };

        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

        #[cfg(feature = "cosmwasm")]
        let execute_msg = ExecuteMsg::UpdateUserAccountAddress {
            new_address: "account2".to_string(),
            new_code_hash: None,
        };
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        let execute_msg = ExecuteMsg::UpdateUserAccountAddress {
            new_address: "account2".to_string(),
            new_code_hash: Some("hash2".to_string()),
        };
        let res = execute(deps.as_mut(), env.clone(), info, execute_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // query UserAccountAddress to ensure update
        let query_msg = QueryMsg::UserAccountAddress {};
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let new_address: UserAccountAddressResponse = from_binary(&res).unwrap();
        assert_eq!("account2", new_address.user_account_address);
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        assert_eq!("hash2", new_address.user_account_code_hash);
    }

    #[test]
    fn query_user_account_address() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        let env = mock_env();

        #[cfg(feature = "cosmwasm")]
        let instantiate_msg = InstantiateMsg {
            user_account_address: "account1".to_string(),
        };
        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        let instantiate_msg = InstantiateMsg {
            user_account_address: "account1".to_string(),
            user_account_code_hash: "hash1".to_string(),
        };

        instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();

        // Test QueryMsg::UserAccountAddress
        let query_msg = QueryMsg::UserAccountAddress {};
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let address: UserAccountAddressResponse = from_binary(&res).unwrap();
        assert_eq!("account1", address.user_account_address);
    }
}
