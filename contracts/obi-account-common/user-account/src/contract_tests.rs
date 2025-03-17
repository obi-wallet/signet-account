#[cfg(test)]
mod tests {
    macros::cosmwasm_imports!(
        coins,
        from_binary,
        to_binary,
        Addr,
        Api,
        BankMsg,
        Binary,
        Coin,
        CosmosMsg,
        Env,
        Uint128,
        Uint256,
        WasmMsg
    );
    macros::cosmwasm_testing_imports!(mock_dependencies, mock_env, mock_info);

    use classes::{
        gatekeeper_common::{GatekeeperType, LegacyOwnerResponse},
        msg_user_account::{ExecuteMsg, InstantiateMsg, QueryMsg},
        rule::Rule,
        signers::{Signer, Signers, SignersUnparsed},
        user_account::{
            CanExecuteResponse, GatekeeperContractsResponse, PendingOwnerResponse, SignersResponse,
            UpdateDelayResponse, UserAccount,
        },
        user_state::AbstractionRule,
    };

    use common::{
        authorization::Authorization,
        common_execute_reasons::{CanExecute, CanExecuteReason},
        universal_msg::UniversalMsg,
    };

    use crate::contract::{execute, instantiate, query};

    // helper
    fn test_account_instantiate_msg(owner: String, attach_user_state: bool) -> InstantiateMsg {
        InstantiateMsg {
            account: UserAccount {
                signers: Signers::new(
                    vec![Signer {
                        address: Addr::unchecked("signer"),
                        ty: "ty".to_string(),
                        pubkey_base_64: "pubkey".to_string(),
                    }],
                    None,
                )
                .unwrap(),
                legacy_owner: Some(owner),
                evm_contract_address: None,
                evm_signing_address: None,
                asset_unifier_contract_addr: Some("asset_unifier_contract_addr".to_string()),
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                asset_unifier_code_hash: Some("asset_unifier_code_hash".to_string()),
                owner_updates_delay_secs: None,
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                gatekeepers: vec![
                    (
                        "message_gatekeeper_contract_addr".to_string(),
                        "message_gatekeeper_code_hash".to_string(),
                    ),
                    (
                        "spendlimit_gatekeeper_contract_addr".to_string(),
                        "spendlimit_gatekeeper_code_hash".to_string(),
                    ),
                ],
                #[cfg(feature = "cosmwasm")]
                gatekeepers: vec![
                    "message_gatekeeper_contract_addr".to_string(),
                    "spendlimit_gatekeeper_contract_addr".to_string(),
                ],
                debtkeeper_contract_addr: Some("debtkeeper_contract_addr".to_string()),
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                debtkeeper_code_hash: None,
                fee_pay_wallet: None,
                // none so that a new one can be attached by factory
                user_state_contract_addr: if attach_user_state {
                    Some("user_state_contract_addr".to_string())
                } else {
                    None
                },
                #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
                user_state_code_hash: Some("user_state_code_hash".to_string()),
                magic_update: false,
                nexthash: "7521287949df0b010a268aafdf2872cbca03eac58db6e2cb6d9c7009aa710d0a"
                    .to_string(),
            },
        }
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let owner = "owner".to_string();
        let msg: InstantiateMsg = test_account_instantiate_msg(owner, true);
        let info = mock_info("anyone", &coins(2, "token"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let query_msg = QueryMsg::GatekeeperContracts {};
        let query_res: GatekeeperContractsResponse =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        #[cfg(feature = "cosmwasm")]
        assert_eq!(
            query_res.gatekeepers,
            vec![
                "message_gatekeeper_contract_addr".to_string(),
                "spendlimit_gatekeeper_contract_addr".to_string(),
            ]
        );

        #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
        assert_eq!(
            query_res.gatekeepers,
            vec![
                (
                    "message_gatekeeper_contract_addr".to_string(),
                    "message_gatekeeper_code_hash".to_string()
                ),
                (
                    "spendlimit_gatekeeper_contract_addr".to_string(),
                    "spendlimit_gatekeeper_code_hash".to_string()
                ),
            ]
        );

        let query_msg = QueryMsg::LegacyOwner {};
        let query_res: LegacyOwnerResponse =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        assert_eq!(query_res.legacy_owner, "owner".to_string());

        let query_msg = QueryMsg::Signers {};
        let query_res: SignersResponse =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        assert_eq!(1, query_res.signers.signers.len());
        assert_eq!("signer", query_res.signers.signers[0].address.to_string());
        assert_eq!("ty", query_res.signers.signers[0].ty);
    }

    #[test]
    fn add_abstraction_rule() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        let actor = "actor";
        let msg = test_account_instantiate_msg(owner.to_string(), true);
        let info = mock_info(owner, &[]);
        let env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // session keys are now message keys with expiration
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

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        assert_eq!(1, res.messages.len());

        if let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = res.messages[0].msg.clone() {
            if let classes::msg_user_state::ExecuteMsg::AddAbstractionRule { new_rule } =
                from_binary(&msg).unwrap()
            {
                assert_eq!(new_rule.actor, deps.api.addr_validate(actor).unwrap());
                assert_eq!(new_rule.ty, GatekeeperType::Allowlist);
                assert_eq!(
                    new_rule.main_rule,
                    Rule::Allow(Authorization {
                        identifier: None,
                        actor: Some(deps.api.addr_validate(actor).unwrap()),
                        contract: None,
                        message_name: None,
                        wasmaction_name: None,
                        fields: None,
                        expiration: env.block.time.plus_seconds(3600).seconds(),
                    })
                );
            } else {
                panic!("Unexpected abstraction message type")
            }
        } else {
            panic!("Unexpected message type")
        }
    }

    #[test]
    fn attach_user_state() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        // user_state can only be attached if none is attached
        let msg = test_account_instantiate_msg(owner.to_string(), false);
        let info = mock_info(owner, &[]);
        let env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::AttachUserState {
            user_state_addr: Some("addr".to_string()),
            #[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
            user_state_code_hash: Some("hash".to_string()),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        assert_eq!(0, res.messages.len());

        let res = execute(deps.as_mut(), env, info, msg);

        assert!(res.is_err());
    }

    #[test]
    fn change_owner_updates_delay() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        let actor = "actor";
        let msg: InstantiateMsg = test_account_instantiate_msg(owner.to_string(), true);
        let info = mock_info(owner, &[]);
        let env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::ChangeOwnerUpdatesDelay { new_delay: 0 };

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(actor, &[]),
            msg.clone(),
        );
        // legacy owner check
        assert!(res.is_err());

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        assert_eq!(0, res.messages.len());

        let query_msg = QueryMsg::UpdateDelay {};
        let query_res: UpdateDelayResponse =
            from_binary(&query(deps.as_ref(), env, query_msg).unwrap()).unwrap();

        assert_eq!(0, query_res.update_delay);
    }

    #[test]
    fn execute_execute() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        let msg = test_account_instantiate_msg(owner.to_string(), true);
        let info = mock_info(owner, &[]);
        let env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        #[cfg(not(feature = "cosmwasm"))]
        {
            let msg = ExecuteMsg::Execute {
                msg: to_binary(&UniversalMsg::Secret(CosmosMsg::Bank(BankMsg::Burn {
                    amount: vec![],
                })))
                .unwrap(),
                sender: owner.to_string(),
                signatures: None,
            };

            let query_msg = QueryMsg::CanExecute {
                address: owner.to_string(),
                #[cfg(not(feature = "cosmwasm"))]
                msg: UniversalMsg::Secret(CosmosMsg::Bank(BankMsg::Burn { amount: vec![] })),
                funds: vec![],
            };
            let query_res: CanExecuteResponse =
                from_binary(&query(deps.as_ref(), env.clone(), query_msg).unwrap()).unwrap();

            assert_eq!(
                query_res.can_execute,
                CanExecute::Yes(CanExecuteReason::OwnerNoDelay)
            );

            let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
        }

        // in secret, legacy should also be supported
        // so both secretwasm and cosmwasm flags can proceed here
        let msg = ExecuteMsg::Execute {
            msg: to_binary(&UniversalMsg::Legacy(
                common::legacy_cosmosmsg::CosmosMsg::Bank(
                    common::legacy_cosmosmsg::BankMsg::Burn { amount: vec![] },
                ),
            ))
            .unwrap(),
            sender: owner.to_string(),
            signatures: None,
        };
        let query_msg = QueryMsg::CanExecute {
            address: owner.to_string(),
            msg: UniversalMsg::Legacy(common::legacy_cosmosmsg::CosmosMsg::Bank(
                common::legacy_cosmosmsg::BankMsg::Burn { amount: vec![] },
            )),
            funds: vec![],
        };
        let query_res: CanExecuteResponse =
            from_binary(&query(deps.as_ref(), env.clone(), query_msg).unwrap()).unwrap();

        assert_eq!(
            query_res.can_execute,
            CanExecute::Yes(CanExecuteReason::OwnerNoDelay)
        );

        let _res = execute(deps.as_mut(), env, info, msg);
    }

    #[test]
    fn propose_update_owner() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        let actor = "actor";
        let msg = test_account_instantiate_msg(owner.to_string(), true);
        let info = mock_info(owner, &[]);
        let env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // note that Signers::new MUST have >0 signers
        let msg = ExecuteMsg::ProposeUpdateOwner {
            new_owner: "new_owner".to_string(),
            signers: SignersUnparsed {
                signers: vec![Signer {
                    address: Addr::unchecked("signer"),
                    ty: "ty".to_string(),
                    pubkey_base_64: "pubkey".to_string(),
                }],
                threshold: None,
            },
            signatures: None,
        };

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(actor, &[]),
            msg.clone(),
        );
        // legacy owner check
        assert!(res.is_err());

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(0, res.messages.len());

        let query_msg = QueryMsg::PendingOwner {};
        let query_res: PendingOwnerResponse =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        assert_eq!("new_owner", query_res.pending_owner);
    }

    #[test]
    fn confirm_update_owner() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        let msg = test_account_instantiate_msg(owner.to_string(), true);
        let info = mock_info(owner, &[]);
        let env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::ProposeUpdateOwner {
            new_owner: "new_owner".to_string(),
            signers: SignersUnparsed {
                signers: vec![Signer {
                    address: Addr::unchecked("signer"),
                    ty: "ty".to_string(),
                    pubkey_base_64: "pubkey".to_string(),
                }],
                threshold: None,
            },
            signatures: None,
        };

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::ConfirmUpdateOwner { signatures: None };

        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());

        assert!(res.is_err());

        let res = execute(deps.as_mut(), env, mock_info("new_owner", &[]), msg).unwrap();

        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn cancel_update_owner() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        let msg = test_account_instantiate_msg(owner.to_string(), true);
        let info = mock_info(owner, &[]);
        let env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::ProposeUpdateOwner {
            new_owner: "new_owner".to_string(),
            signers: SignersUnparsed {
                signers: vec![Signer {
                    address: Addr::unchecked("signer"),
                    ty: "ty".to_string(),
                    pubkey_base_64: "pubkey".to_string(),
                }],
                threshold: None,
            },
            signatures: None,
        };

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CancelUpdateOwner {};

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("test", &[]),
            msg.clone(),
        );

        assert!(res.is_err());

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(0, res.messages.len());

        // cancelling pending owner resets pending to == current owner
        let query_msg = QueryMsg::PendingOwner {};
        let query_res: PendingOwnerResponse =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        assert_eq!("owner", query_res.pending_owner);
    }

    #[test]
    fn remove_abstraction_rule() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        let actor = "actor";
        let msg = test_account_instantiate_msg(owner.to_string(), true);
        let info = mock_info(owner, &[]);
        let env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

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

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::RmAbstractionRule {
            ty: GatekeeperType::Allowlist,
            rule_id: 0u16,
        };

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(actor, &[]),
            msg.clone(),
        );
        // legacy owner check
        assert!(res.is_err());

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(1, res.messages.len());

        if let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = res.messages[0].msg.clone() {
            if let Ok(classes::msg_user_state::ExecuteMsg::RmAbstractionRule { ty, rule_id }) =
                from_binary(&msg)
            {
                assert_eq!(ty, GatekeeperType::Allowlist);
                assert_eq!(rule_id, 0u16);
            } else {
                panic!("Unexpected message type")
            }
        } else {
            panic!("Unexpected message type")
        }
    }

    #[test]
    fn upsert_abstraction_rule() {
        let mut deps = mock_dependencies();
        let owner = "owner";
        let actor = "actor";
        let msg = test_account_instantiate_msg(owner.to_string(), true);
        let info = mock_info(owner, &[]);
        let env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let updated_rule = AbstractionRule {
            id: Some(0),
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

        let msg = ExecuteMsg::UpsertAbstractionRule {
            id: 0,
            updated_rule,
        };

        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info(actor, &[]),
            msg.clone(),
        );
        // legacy owner check
        assert!(res.is_err());

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(1, res.messages.len());

        if let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = res.messages[0].msg.clone() {
            if let classes::msg_user_state::ExecuteMsg::UpsertAbstractionRule { id, updated_rule } =
                from_binary(&msg).unwrap()
            {
                assert_eq!(id, 0u16);
                assert_eq!(updated_rule.actor, deps.api.addr_validate(actor).unwrap());
            } else {
                panic!("Unexpected message type")
            }
        } else {
            panic!("Unexpected message type")
        }
    }
}
