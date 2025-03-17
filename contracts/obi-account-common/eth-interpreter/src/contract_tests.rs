mod test {
    use common::eth::EthUserOp;

    use crate::{
        contract::{self, query},
        msg::{InstantiateMsg, ParseUserOpResponse, QueryMsg},
    };
    macros::cosmwasm_imports!(from_binary, Uint256);
    macros::cosmwasm_testing_imports!(mock_dependencies, mock_env, mock_info);

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {};
        let res = contract::instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn query_parse_user_op() {
        let deps = mock_dependencies();
        let env = mock_env();
        let msg = QueryMsg::ParseUserOp {
            user_op: EthUserOp::dummy(true, true),
        };
        let res: ParseUserOpResponse =
            from_binary(&query(deps.as_ref(), env, msg).unwrap()).unwrap();
        // TODO: fields logic not implemented yet
        assert!(res.fields.is_none());
        // TODO: function enum with function signatures for ERC standards
        assert!(res.function_signatures == vec!["18dfb3c7".to_string()]);
        assert!(
            res.contract_address == Some("5cf29823ccfc73008fa53630d54a424ab82de6f2".to_string())
        );
        assert!(res.fee_recipient == Some("c1d4f3dcc31d86a66de90c3c5986f0666cc52ce4".to_string()));
        assert!(res.fee[0].amount == Uint256::from(1000000000000000u64));
        assert!(res.spend[0].amount == Uint256::from(1000000000000000000u64));
    }
}
