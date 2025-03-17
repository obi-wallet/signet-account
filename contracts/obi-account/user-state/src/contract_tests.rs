#[cfg(test)]
mod tests {
    macros::cosmwasm_imports!(coins, from_binary, Addr, Api, Coin, Env, Uint128, Uint256);
    macros::cosmwasm_testing_imports!(mock_dependencies, mock_env, mock_info);
    use classes::gatekeeper_common::GatekeeperType;
    use classes::msg_user_state::{ExecuteMsg, InstantiateMsg, LastActivityResponse, QueryMsg};
    use classes::permissioned_address::{CoinBalance, PeriodType, PermissionedAddressParams};
    use classes::rule::Rule;
    use classes::user_state::{AbstractionRule, AbstractionRules};
    use common::authorization::Authorization;

    use crate::contract::{execute, instantiate, query, query_last_activity};

    // in these cases "owner" will be user_account in full deployment
    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let owner = "owner".to_string();
        let msg = InstantiateMsg {
            user_account_address: deps.api.addr_validate(&owner).unwrap().to_string(),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash: "hash".to_string(),
        };
        let info = mock_info("anyone", &coins(2, "token"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn add_abstraction_rule() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        let actor = "actor";
        let msg = InstantiateMsg {
            user_account_address: deps.api.addr_validate(owner).unwrap().to_string(),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash: "hash".to_string(),
        };
        let info = mock_info(owner, &[]);
        let mut env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let old_last_activity = query_last_activity(deps.as_ref()).last_activity;
        env.block.time = env.block.time.plus_seconds(6);

        let new_rule = AbstractionRule {
            id: None,
            actor: deps.api.addr_validate(actor).unwrap(),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(deps.api.addr_validate(actor).unwrap()),
                contract: None,
                message_name: None,
                wasmaction_name: None,
                fields: None,
                expiration: env.block.time.plus_seconds(3600).seconds(),
            }),
        };

        let msg = ExecuteMsg::AddAbstractionRule { new_rule };

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(actor, &[]),
            msg.clone(),
        );
        // legacy owner check
        assert!(res.is_err());

        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // now one rule
        let query_msg = QueryMsg::AbstractionRules {
            actor: Some(deps.api.addr_validate(actor).unwrap()),
            ty: vec![GatekeeperType::Allowlist],
        };
        let res: AbstractionRules =
            from_binary(&query(deps.as_ref(), env, query_msg).unwrap()).unwrap();
        assert_eq!(res.rules.len(), 1);

        let new_last_activity = query_last_activity(deps.as_ref()).last_activity;
        assert!(new_last_activity > old_last_activity);
    }

    #[test]
    fn remove_abstraction_rule() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        let actor = "actor";
        let msg = InstantiateMsg {
            user_account_address: deps.api.addr_validate(owner).unwrap().to_string(),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash: "hash".to_string(),
        };
        let info = mock_info(owner, &coins(2, "token"));
        let mut env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let old_last_activity = query_last_activity(deps.as_ref()).last_activity;
        env.block.time = env.block.time.plus_seconds(6);

        let new_rule = AbstractionRule {
            id: None,
            actor: deps.api.addr_validate(actor).unwrap(),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(deps.api.addr_validate(actor).unwrap()),
                contract: None,
                message_name: None,
                wasmaction_name: None,
                fields: None,
                expiration: env.block.time.plus_seconds(3600).seconds(),
            }),
        };

        let add_rule_msg = ExecuteMsg::AddAbstractionRule { new_rule: new_rule };

        let _res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            add_rule_msg.clone(),
        )
        .unwrap();

        let rm_rule_msg = ExecuteMsg::RmAbstractionRule {
            ty: GatekeeperType::Allowlist,
            rule_id: 0u16,
        };

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("random", &[]),
            rm_rule_msg.clone(),
        );
        assert!(res.is_err());

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(actor, &[]),
            rm_rule_msg,
        );
        // actor can remove their own rule!
        assert!(res.is_ok());

        // check to verify no rule
        let query_msg = QueryMsg::AbstractionRules {
            actor: Some(deps.api.addr_validate(actor).unwrap()),
            ty: vec![GatekeeperType::Allowlist],
        };
        let res: AbstractionRules =
            from_binary(&query(deps.as_ref(), env.clone(), query_msg).unwrap()).unwrap();
        assert_eq!(res.rules.len(), 0);

        // owner can also remove rule...
        // (note id still increments, as cannot be reused, but we check for 0 rules)
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), add_rule_msg).unwrap();
        let rm_rule_msg = ExecuteMsg::RmAbstractionRule {
            ty: GatekeeperType::Allowlist,
            rule_id: 1u16,
        };
        let _res = execute(deps.as_mut(), env.clone(), info, rm_rule_msg).unwrap();

        // check to verify no rules
        let query_msg = QueryMsg::AbstractionRules {
            actor: Some(deps.api.addr_validate(actor).unwrap()),
            ty: vec![GatekeeperType::Allowlist],
        };
        let res: AbstractionRules =
            from_binary(&query(deps.as_ref(), env, query_msg).unwrap()).unwrap();
        assert_eq!(res.rules.len(), 0);

        let new_last_activity = query_last_activity(deps.as_ref()).last_activity;
        assert!(new_last_activity > old_last_activity);
    }

    #[test]
    fn update_abstraction_rule() {
        let mut deps = mock_dependencies();
        let owner = "alice";
        let info = mock_info(owner, &coins(1000, "earth"));
        let msg = InstantiateMsg {
            user_account_address: deps.api.addr_validate(owner).unwrap().to_string(),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash: "hash".to_string(),
        };
        let env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Add a rule
        let add_rule_msg = ExecuteMsg::AddAbstractionRule {
            new_rule: AbstractionRule {
                id: None,
                actor: Addr::unchecked("bob"),
                ty: GatekeeperType::Allowlist,
                main_rule: Rule::Allow(Authorization {
                    identifier: None,
                    actor: Some(Addr::unchecked("bob")),
                    contract: None,
                    message_name: None,
                    wasmaction_name: None,
                    fields: None,
                    expiration: 1571797479,
                }),
            },
        };

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), add_rule_msg).unwrap();

        // get the id for the new rule
        let query_msg = QueryMsg::AbstractionRules {
            actor: Some(Addr::unchecked("bob")),
            ty: vec![GatekeeperType::Allowlist],
        };
        let query_res: AbstractionRules =
            from_binary(&query(deps.as_ref(), env, query_msg).unwrap()).unwrap();

        // Update the rule
        let updated_rule = AbstractionRule {
            id: query_res.rules[0].id,
            actor: Addr::unchecked("bob"),
            ty: GatekeeperType::Allowlist,
            main_rule: Rule::Allow(Authorization {
                identifier: None,
                actor: Some(Addr::unchecked("bob")),
                contract: None,
                message_name: None,
                wasmaction_name: None,
                fields: None,
                expiration: 9999999999,
            }),
        };

        let update_rule_msg = ExecuteMsg::UpsertAbstractionRule {
            id: query_res.rules[0].id.unwrap(),
            updated_rule: updated_rule.clone(),
        };

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("bob", &[]),
            update_rule_msg.clone(),
        );
        // legacy owner check
        assert!(res.is_err());

        let _res = execute(deps.as_mut(), mock_env(), info, update_rule_msg).unwrap();

        // Query for rules
        let query_msg = QueryMsg::AbstractionRules {
            actor: Some(Addr::unchecked("bob")),
            ty: vec![GatekeeperType::Allowlist],
        };

        let query_res: AbstractionRules =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        // Check if the rule is updated
        assert_eq!(query_res.rules.len(), 1);
        assert_eq!(query_res.rules[0], updated_rule);
    }

    #[test]
    fn update_last_activity() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        let msg = InstantiateMsg {
            user_account_address: deps.api.addr_validate(owner).unwrap().to_string(),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash: "hash".to_string(),
        };
        let info = mock_info(owner, &[]);
        let mut env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let old_last_activity = query_last_activity(deps.as_ref()).last_activity;
        env.block.time = env.block.time.plus_seconds(6);

        let msg = ExecuteMsg::UpdateLastActivity {};

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("user", &[]),
            msg.clone(),
        );
        // legacy owner check
        assert!(res.is_err());

        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let query_msg = QueryMsg::LastActivity {};
        let res: LastActivityResponse =
            from_binary(&query(deps.as_ref(), env, query_msg).unwrap()).unwrap();
        assert!(res.last_activity > old_last_activity);
    }

    #[test]
    fn query_abstraction_rules() {
        let env = mock_env();
        let mut deps = mock_dependencies();
        let owner = "alice";
        let info = mock_info(owner, &coins(1000, "earth"));
        let msg = InstantiateMsg {
            user_account_address: deps.api.addr_validate(owner).unwrap().to_string(),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_account_code_hash: "hash".to_string(),
        };

        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Adding some abstraction rules
        let mut execute_msg = ExecuteMsg::AddAbstractionRule {
            new_rule: AbstractionRule {
                id: None,
                actor: deps.api.addr_validate("actor1").unwrap(),
                ty: GatekeeperType::Spendlimit,
                main_rule: Rule::Spendlimit(PermissionedAddressParams {
                    address: "address1".to_string(),
                    cooldown: 100,
                    period_type: PeriodType::Days,
                    period_multiple: 2,
                    spend_limits: vec![CoinBalance {
                        denom: "c".to_string(),
                        amount: Uint256::from(100u128),
                        spent_this_inheritance_period: None,
                        limit_remaining: Uint256::from(0u128),
                    }],
                    default: None,
                    denom: Some("test".to_string()),
                    offset: 0,
                    inheritance_records: vec![],
                    expiration: 0,
                }),
            },
        };
        execute(deps.as_mut(), env.clone(), info.clone(), execute_msg).unwrap();

        execute_msg = ExecuteMsg::AddAbstractionRule {
            new_rule: AbstractionRule {
                id: None,
                actor: deps.api.addr_validate("actor2").unwrap(),
                ty: GatekeeperType::Spendlimit,
                main_rule: Rule::Spendlimit(PermissionedAddressParams {
                    address: "address2".to_string(),
                    cooldown: 200,
                    period_type: PeriodType::Months,
                    period_multiple: 3,
                    spend_limits: vec![CoinBalance {
                        denom: "test2".to_string(),
                        amount: Uint256::from(100u128),
                        spent_this_inheritance_period: None,
                        limit_remaining: Uint256::from(100u128),
                    }],
                    default: None,
                    denom: Some("test2".to_string()),
                    offset: 0,
                    inheritance_records: vec![],
                    expiration: 0,
                }),
            },
        };
        execute(deps.as_mut(), env.clone(), info.clone(), execute_msg).unwrap();

        execute_msg = ExecuteMsg::AddAbstractionRule {
            new_rule: AbstractionRule {
                id: None,
                actor: deps.api.addr_validate("actor3").unwrap(),
                ty: GatekeeperType::Allowlist,
                main_rule: Rule::Allow(Authorization {
                    identifier: None,
                    actor: Some(deps.api.addr_validate("actor3").unwrap()),
                    contract: None,
                    message_name: None,
                    wasmaction_name: None,
                    fields: None,
                    expiration: env.block.time.plus_seconds(300).seconds(),
                }),
            },
        };
        execute(deps.as_mut(), env.clone(), info, execute_msg).unwrap();

        // Querying abstraction rules by actor
        let query_msg = QueryMsg::AbstractionRules {
            actor: Some(deps.api.addr_validate("actor1").unwrap()),
            ty: vec![],
        };
        let res: AbstractionRules =
            from_binary(&query(deps.as_ref(), env.clone(), query_msg).unwrap()).unwrap();
        assert_eq!(res.rules.len(), 1);
        assert_eq!(
            res.rules[0].actor,
            deps.api.addr_validate("actor1").unwrap()
        );
        assert_eq!(res.rules[0].ty, GatekeeperType::Spendlimit);

        // Querying abstraction rules by type
        let query_msg = QueryMsg::AbstractionRules {
            actor: None,
            ty: vec![GatekeeperType::Spendlimit],
        };
        let res: AbstractionRules =
            from_binary(&query(deps.as_ref(), env.clone(), query_msg).unwrap()).unwrap();
        assert_eq!(res.rules.len(), 2);
        assert_eq!(
            res.rules[0].actor,
            deps.api.addr_validate("actor1").unwrap()
        );
        assert_eq!(res.rules[0].ty, GatekeeperType::Spendlimit);
        assert_eq!(
            res.rules[1].actor,
            deps.api.addr_validate("actor2").unwrap()
        );
        assert_eq!(res.rules[1].ty, GatekeeperType::Spendlimit);

        // Querying abstraction rules by both actor and type
        let query_msg = QueryMsg::AbstractionRules {
            actor: Some(deps.api.addr_validate("actor2").unwrap()),
            ty: vec![GatekeeperType::Spendlimit],
        };
        let res: AbstractionRules =
            from_binary(&query(deps.as_ref(), env, query_msg).unwrap()).unwrap();
        assert_eq!(res.rules.len(), 1);
        assert_eq!(
            res.rules[0].actor,
            deps.api.addr_validate("actor2").unwrap()
        );
        assert_eq!(res.rules[0].ty, GatekeeperType::Spendlimit);
    }
}
