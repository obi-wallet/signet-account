#[cfg(test)]
mod tests {
    use std::str::from_utf8;
    use std::vec::Vec;

    use curv::arithmetic::Converter;
    use curv::elliptic::curves::Scalar;
    use curv::BigInt;
    use digest::Digest;
    use hex::FromHex;
    use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::{
        verify, Parameters, SignatureRecid,
    };
    use secret_cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use secret_cosmwasm_std::{
        coins, from_binary, DepsMut, MessageInfo, StdError, StdResult, Uint256,
    };

    use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::sign::{
        PartialSignature, SignManual,
    };
    use sha2::Sha256;

    use scrt_sss::{ECScalar, Secp256k1Scalar};

    use crate::contract_execute::execute;
    use crate::contract_instantiate::instantiate;
    use crate::contract_query::query;
    use crate::errors::SecretShareSignerError;
    use crate::msg::{self, CompletedOfflineStageParts, QueryMsg};
    use crate::msg::{ExecuteMsg, InstantiateMsg, ParticipantsToCompletedOfflineStageParts};
    use crate::test::simulation::tests::{simulate_keygen, simulate_offline_stage};
    use common::eth::EthUserOp;

    fn test_user_op() -> EthUserOp {
        EthUserOp {
            sender: "12a2Fd1adA63FBCA7Cd9ec550098D48600D6dDc7".to_string(),
            nonce: Uint256::one(),
            init_code: vec![],
            call_data: Vec::from_hex("b61d27f60000000000000000000000005cf29823ccfc73008fa53630d54a424ab82de6f2000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000044a9059cbb0000000000000000000000005e73c6729a0a0d6ddd2f9c7504cb146d2dcd1d6000000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: Uint256::from(u64::from_str_radix("1352e", 16).unwrap()),
            verification_gas_limit: Uint256::from(u64::from_str_radix("1984a", 16).unwrap()),
            pre_verification_gas: Uint256::from(u64::from_str_radix("d0df", 16).unwrap()),
            max_fee_per_gas: Uint256::from(u64::from_str_radix("6ef1edcce", 16).unwrap()),
            max_priority_fee_per_gas: Uint256::from(u64::from_str_radix("3b06", 16).unwrap()),
            paymaster_and_data: Vec::from_hex("e93eca6595fe94091dc1af46aac2a8b5d79907700000000000000000000000000000000000000000000000000000000064b564460000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003495a86706e134c9c162d4a020097db67f95673294f1d7a42608633046b13ab7130f5918760717b4a74c347c595d38fc2bdeaa7dc17429a9f6c1ac3f344266e91b").unwrap(),
            signature: vec![],
        }
    }
    impl From<SecretShareSignerError> for StdError {
        fn from(value: SecretShareSignerError) -> Self {
            StdError::generic_err(value.to_string())
        }
    }

    fn instantiate_contract(deps: DepsMut) -> MessageInfo {
        let msg = InstantiateMsg {
            fee_manager_address: "test_fee_manager_address".to_string(),
            fee_manager_code_hash: "test_fee_manager_code_hash".to_string(),
        };
        let info = mock_info("creator", &coins(2, "token"));
        instantiate(deps, mock_env(), info.clone(), msg).unwrap();
        info
    }

    #[test]
    fn execute_multiparty_ecdsa_test() -> StdResult<()> {
        let params = Parameters {
            threshold: 2,
            share_count: 5,
        };
        // First of all, parties need to carry out distributed key generation protocol.
        // After DKG is successfully completed, it outputs [LocalKey] — a party local secret share.
        // With the private keys that each party will use to sign with
        let local_keys = simulate_keygen(&params);
        println!(
            "Parties Share Keys (The private keys they use to sign with): {:?}",
            local_keys
                .iter()
                .map(|p| hex::encode(p.keys_linear.x_i.to_bytes().as_ref()))
                .collect::<Vec<_>>()
        );
        // Then you fix a set of parties who will participate in threshold signing, and they run
        // [OfflineStage] protocol. `OfflineStage` implements [StateMachine] and can be executed in the same
        // way as [Keygen]. `OfflineStage` outputs a [CompletedOfflineStage].
        let participating_parties = vec![1, 2, 3, 4];
        let completed_offline_stages =
            simulate_offline_stage(&local_keys, participating_parties.as_slice());
        // [SignManual] takes a `CompletedOfflineStage` and allows you to perform one-round signing.
        // It doesn't implement `StateMachine`, but rather provides methods to construct messages
        // and final signature manually (refer to [SignManual] documentation to see how to use it).
        // Sign a message with each of the shares
        // Generate 32 random bytes
        let msg = b"Hello";
        let msg_hash = Sha256::digest(msg).to_vec();
        let msg_to_sign = BigInt::from_bytes(msg_hash.as_slice());
        println!("Message: {}", from_utf8(msg).unwrap());

        // Sign a message locally
        let (sign, _sig) =
            SignManual::new(msg_to_sign.clone(), completed_offline_stages[0].clone()).unwrap();
        // Collect partial signatures from other parties
        let sigs: Vec<PartialSignature> = completed_offline_stages[1..]
            .iter()
            .map(|c| {
                let (_sign, sig) = SignManual::new(msg_to_sign.clone(), c.clone()).unwrap();
                sig
            })
            .collect();
        // Complete signing
        let signature = sign.complete(&sigs).unwrap();
        // Verify that signature matches joint public key
        assert!(verify(
            &signature,
            completed_offline_stages[0].public_key(),
            &msg_to_sign
        )
        .is_ok());
        Ok(())
    }

    #[test]
    fn execute_contract_sign_with_aggregated_partial_sigs() -> StdResult<()> {
        let mut deps = mock_dependencies();
        let info = instantiate_contract(deps.as_mut());
        let params = Parameters {
            threshold: 2,
            share_count: 5,
        };
        let local_keys = simulate_keygen(&params);
        let participating_parties = vec![1, 2, 3];
        let completed_offline_stages =
            simulate_offline_stage(&local_keys, participating_parties.as_slice());
        let completed_offline_stage_for_contract = completed_offline_stages[0].clone();
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::SetShares {
                participants_to_completed_offline_stages: vec![
                    ParticipantsToCompletedOfflineStageParts {
                        participants: participating_parties.iter().map(|p| *p as u8).collect(),
                        completed_offline_stage: Some(CompletedOfflineStageParts::from_value_hash(
                            completed_offline_stage_for_contract,
                            "test_user_entry_code_hash".to_string(),
                        )),
                    },
                ],
                user_entry_address: "test_user_entry".to_string(),
            },
        )?;
        let msg_hash: Vec<u8> =
            test_user_op().get_user_op_hash("5FF137D4b0FDCD49DcA30c7CF57E578a026d2789", 5);
        println!("Message (user_op hash): {}", hex::encode(&msg_hash));
        let msg_to_sign = BigInt::from_bytes(msg_hash.as_slice());

        let (first_sign_manual, _sig) =
            SignManual::new(msg_to_sign.clone(), completed_offline_stages[1].clone()).unwrap();

        // Collect partial signatures from other parties
        let partial_sigs: Vec<PartialSignature> = completed_offline_stages[2..]
            .iter()
            .map(|c| {
                let (_sign, sig) = SignManual::new(msg_to_sign.clone(), c.clone()).unwrap();
                sig
            })
            .collect();
        let (needs_one_more_sig_sign_manual, all_but_one_sig) =
            first_sign_manual.add(&partial_sigs).unwrap();
        assert!(needs_one_more_sig_sign_manual.complete(&[]).is_err());
        let sign_msg = ExecuteMsg::Sign {
            participants: participating_parties.iter().map(|p| *p as u8).collect(),
            user_entry_address: "test_user_entry".to_string(),
            user_entry_code_hash: "test_user_entry_code_hash".to_string(),
            entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string(),
            chain_id: "5".to_string(),
            user_operation: Box::from(test_user_op()),
            other_partial_sigs: vec![msg::PartialSignature(
                Secp256k1Scalar::from_slice(all_but_one_sig.0.to_bytes().as_ref()).unwrap(),
            )],
        };

        // Have the contract complete signing
        let res = execute(deps.as_mut(), mock_env(), info, sign_msg)?;
        let signature: crate::multi_party_ecdsa::local_signature::SignatureRecid =
            from_binary(&res.data.unwrap())?;
        // The contract will verify the signature. But, verify again that signature matches joint
        // public key a different w
        assert!(verify(
            &SignatureRecid {
                r: Scalar::from_bytes(&signature.r.to_raw()).unwrap(),
                s: Scalar::from_bytes(&signature.s.to_raw()).unwrap(),
                recid: signature.recid,
            },
            completed_offline_stages[0].public_key(),
            &msg_to_sign
        )
        .is_ok());

        Ok(())
    }

    #[test]
    fn execute_contract_sign_with_unaggregated_partial_sigs() -> StdResult<()> {
        let mut deps = mock_dependencies();
        let info = instantiate_contract(deps.as_mut());
        let params = Parameters {
            threshold: 2,
            share_count: 5,
        };
        let local_keys = simulate_keygen(&params);

        let participating_parties = vec![1, 2, 4];
        let completed_offline_stages =
            simulate_offline_stage(&local_keys, participating_parties.as_slice());
        let completed_offline_stage_for_contract = completed_offline_stages[0].clone();
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::SetShares {
                participants_to_completed_offline_stages: vec![
                    ParticipantsToCompletedOfflineStageParts {
                        participants: participating_parties.iter().map(|p| *p as u8).collect(),
                        completed_offline_stage: Some(CompletedOfflineStageParts::from_value_hash(
                            completed_offline_stage_for_contract,
                            "test_user_entry_code_hash".to_string(),
                        )),
                    },
                ],
                user_entry_address: "test_user_entry".to_string(),
            },
        )?;

        let msg_hash: Vec<u8> =
            test_user_op().get_user_op_hash("5FF137D4b0FDCD49DcA30c7CF57E578a026d2789", 5);
        let msg_to_sign = BigInt::from_bytes(msg_hash.as_slice());

        // Collect partial signatures from other parties
        let _partial_sig = completed_offline_stages.len();
        let partial_sigs: Vec<msg::PartialSignature> = completed_offline_stages[1..]
            .iter()
            .map(|c| {
                let (_sign, sig) = SignManual::new(msg_to_sign.clone(), c.clone()).unwrap();
                msg::PartialSignature(
                    Secp256k1Scalar::from_slice(sig.0.to_bytes().as_ref()).unwrap(),
                )
            })
            .collect();

        let sign_msg = ExecuteMsg::Sign {
            participants: participating_parties.iter().map(|p| *p as u8).collect(),
            user_entry_address: "test_user_entry".to_string(),
            user_entry_code_hash: "test_user_entry_code_hash".to_string(),
            entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string(),
            chain_id: "5".to_string(),
            user_operation: Box::from(test_user_op()),
            other_partial_sigs: partial_sigs,
        };

        // Have the contract complete signing
        let res = execute(deps.as_mut(), mock_env(), info, sign_msg)?;
        let signature: crate::multi_party_ecdsa::local_signature::SignatureRecid =
            from_binary(&res.data.unwrap())?;
        // The contract will verify the signature. But, verify again that signature matches joint
        // public key a different way
        assert!(verify(
            &SignatureRecid {
                r: Scalar::from_bytes(&signature.r.to_raw()).unwrap(),
                s: Scalar::from_bytes(&signature.s.to_raw()).unwrap(),
                recid: signature.recid,
            },
            completed_offline_stages[0].public_key(),
            &msg_to_sign
        )
        .is_ok());

        Ok(())
    }

    #[test]
    fn contract_fails_to_sign_w_partial_sigs_from_wrong_participant_group() -> StdResult<()> {
        let mut deps = mock_dependencies();
        let info = instantiate_contract(deps.as_mut());
        let params = Parameters {
            threshold: 2,
            share_count: 5,
        };
        let local_keys = simulate_keygen(&params);

        let participating_parties_0 = vec![1, 4, 5];
        let completed_offline_stages_0 =
            simulate_offline_stage(&local_keys, participating_parties_0.as_slice());
        let participating_parties_1 = vec![1, 3, 4];
        let completed_offline_stages_1 =
            simulate_offline_stage(&local_keys, participating_parties_1.as_slice());
        let completed_offline_stage_for_contract = completed_offline_stages_0[0].clone();
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::SetShares {
                participants_to_completed_offline_stages: vec![
                    ParticipantsToCompletedOfflineStageParts {
                        participants: participating_parties_0.iter().map(|p| *p as u8).collect(),
                        completed_offline_stage: Some(CompletedOfflineStageParts::from_value_hash(
                            completed_offline_stage_for_contract,
                            "test_user_entry_code_hash".to_string(),
                        )),
                    },
                ],
                user_entry_address: "test_user_entry".to_string(),
            },
        )?;

        let msg_hash: Vec<u8> =
            test_user_op().get_user_op_hash("5FF137D4b0FDCD49DcA30c7CF57E578a026d2789", 5);
        let msg_to_sign = BigInt::from_bytes(msg_hash.as_slice());

        // Collect partial signatures from other parties
        let partial_sigs_from_wrong_participant_group: Vec<msg::PartialSignature> =
            completed_offline_stages_1[1..]
                .iter()
                .map(|c| {
                    let (_sign, sig) = SignManual::new(msg_to_sign.clone(), c.clone()).unwrap();
                    msg::PartialSignature(
                        Secp256k1Scalar::from_slice(sig.0.to_bytes().as_ref()).unwrap(),
                    )
                })
                .collect();

        let sign_msg = ExecuteMsg::Sign {
            participants: participating_parties_0.iter().map(|p| *p as u8).collect(),
            user_entry_address: "test_user_entry".to_string(),
            user_entry_code_hash: "test_user_entry_code_hash".to_string(),
            entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string(),
            chain_id: "5".to_string(),
            user_operation: Box::from(test_user_op()),
            other_partial_sigs: partial_sigs_from_wrong_participant_group,
        };

        // Have the contract complete signing
        assert!(
            execute(deps.as_mut(), mock_env(), info, sign_msg).is_err(),
            "Should fail to sign with partial sigs from wrong participant group"
        );

        Ok(())
    }

    #[test]
    fn execute_contract_sign_fails_w_lt_threshold_count_of_unaggregated_partial_sigs(
    ) -> StdResult<()> {
        let mut deps = mock_dependencies();
        let info = instantiate_contract(deps.as_mut());
        let params = Parameters {
            threshold: 2,
            share_count: 5,
        };
        let local_keys = simulate_keygen(&params);

        let participating_parties = vec![2, 3, 4];
        let completed_offline_stages =
            simulate_offline_stage(&local_keys, participating_parties.as_slice());
        let completed_offline_stage_for_contract = completed_offline_stages[0].clone();
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::SetShares {
                participants_to_completed_offline_stages: vec![
                    ParticipantsToCompletedOfflineStageParts {
                        participants: participating_parties.iter().map(|p| *p as u8).collect(),
                        completed_offline_stage: Some(CompletedOfflineStageParts::from_value_hash(
                            completed_offline_stage_for_contract,
                            "test_user_entry_code_hash".to_string(),
                        )),
                    },
                ],
                user_entry_address: "test_user_entry".to_string(),
            },
        )?;

        let msg_hash: Vec<u8> =
            test_user_op().get_user_op_hash("5FF137D4b0FDCD49DcA30c7CF57E578a026d2789", 5);
        let msg_to_sign = BigInt::from_bytes(msg_hash.as_slice());

        // Collect partial signatures from other parties
        let partial_sigs_missing_last_one: Vec<msg::PartialSignature> = completed_offline_stages
            [1..completed_offline_stages.len() - 1]
            .iter()
            .map(|c| {
                let (_sign, sig) = SignManual::new(msg_to_sign.clone(), c.clone()).unwrap();
                msg::PartialSignature(
                    Secp256k1Scalar::from_slice(sig.0.to_bytes().as_ref()).unwrap(),
                )
            })
            .collect();

        let sign_msg = ExecuteMsg::Sign {
            participants: participating_parties.iter().map(|p| *p as u8).collect(),
            user_entry_address: "test_user_entry".to_string(),
            user_entry_code_hash: "test_user_entry_code_hash".to_string(),
            entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string(),
            chain_id: "5".to_string(),
            user_operation: Box::from(test_user_op()),
            other_partial_sigs: partial_sigs_missing_last_one,
        };

        // Have the contract complete signing
        assert!(
            execute(deps.as_mut(), mock_env(), info, sign_msg).is_err(),
            "Should fail to sign with partial sigs from wrong participant group"
        );

        Ok(())
    }

    #[test]
    fn execute_contract_sign_fails_w_lt_threshold_count_of_aggregated_partial_sigs() -> StdResult<()>
    {
        let mut deps = mock_dependencies();
        let info = instantiate_contract(deps.as_mut());
        let params = Parameters {
            threshold: 2,
            share_count: 5,
        };
        let local_keys = simulate_keygen(&params);

        let participating_parties = vec![3, 4, 5];
        let completed_offline_stages =
            simulate_offline_stage(&local_keys, participating_parties.as_slice());
        let completed_offline_stage_for_contract = completed_offline_stages[0].clone();
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::SetShares {
                participants_to_completed_offline_stages: vec![
                    ParticipantsToCompletedOfflineStageParts {
                        participants: participating_parties.iter().map(|p| *p as u8).collect(),
                        completed_offline_stage: Some(CompletedOfflineStageParts::from_value_hash(
                            completed_offline_stage_for_contract,
                            "test_user_entry_code_hash".to_string(),
                        )),
                    },
                ],
                user_entry_address: "test_user_entry".to_string(),
            },
        )?;

        let msg_hash: Vec<u8> =
            test_user_op().get_user_op_hash("5FF137D4b0FDCD49DcA30c7CF57E578a026d2789", 5);
        let msg_to_sign = BigInt::from_bytes(msg_hash.as_slice());

        // Collect partial signatures from other parties
        let (first_sign_manual, _sig) =
            SignManual::new(msg_to_sign.clone(), completed_offline_stages[1].clone()).unwrap();

        // Collect partial signatures from other parties
        let partial_sigs_missing_last_one: Vec<PartialSignature> = completed_offline_stages
            [2..completed_offline_stages.len() - 1]
            .iter()
            .map(|c| {
                let (_sign, sig) = SignManual::new(msg_to_sign.clone(), c.clone()).unwrap();
                sig
            })
            .collect();
        let (_needs_one_more_sig_sign_manual, all_but_one_sig) = first_sign_manual
            .add(&partial_sigs_missing_last_one)
            .unwrap();

        let sign_msg = ExecuteMsg::Sign {
            participants: participating_parties.iter().map(|p| *p as u8).collect(),
            user_entry_address: "test_user_entry".to_string(),
            user_entry_code_hash: "test_user_entry_code_hash".to_string(),
            entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string(),
            chain_id: "5".to_string(),
            user_operation: Box::from(test_user_op()),
            other_partial_sigs: vec![msg::PartialSignature(
                Secp256k1Scalar::from_slice(all_but_one_sig.0.to_bytes().as_ref()).unwrap(),
            )],
        };

        // Have the contract complete signing
        assert!(
            execute(deps.as_mut(), mock_env(), info, sign_msg).is_err(),
            "Should fail to sign with partial sigs from wrong participant group"
        );

        Ok(())
    }

    #[test]
    fn execute_contract_sign_succeeds_for_all_participant_combos() -> StdResult<()> {
        let mut deps = mock_dependencies();
        let info = instantiate_contract(deps.as_mut());
        let params = Parameters {
            threshold: 2,
            share_count: 5,
        };
        let local_keys = simulate_keygen(&params);

        let all_participating_parties_combos = vec![
            vec![1, 2, 3],
            vec![1, 2, 4],
            vec![1, 2, 5],
            vec![1, 3, 4],
            vec![1, 3, 5],
            vec![1, 4, 5],
            vec![2, 3, 4],
            vec![2, 3, 5],
            vec![2, 4, 5],
            vec![3, 4, 5],
            vec![1, 2, 3, 4],
            vec![2, 3, 4, 5],
            vec![3, 4, 5, 1],
            vec![3, 4, 5, 1],
            vec![4, 5, 1, 2],
            vec![5, 1, 2, 3],
            vec![1, 2, 3, 4, 5],
        ];
        for participating_parties in all_participating_parties_combos {
            let completed_offline_stages =
                simulate_offline_stage(&local_keys, participating_parties.as_slice());
            let completed_offline_stage_for_contract = completed_offline_stages[0].clone();
            execute(
                deps.as_mut(),
                mock_env(),
                info.clone(),
                ExecuteMsg::SetShares {
                    participants_to_completed_offline_stages: vec![
                        ParticipantsToCompletedOfflineStageParts {
                            participants: participating_parties.iter().map(|p| *p as u8).collect(),
                            completed_offline_stage: Some(
                                CompletedOfflineStageParts::from_value_hash(
                                    completed_offline_stage_for_contract,
                                    "test_user_entry_code_hash".to_string(),
                                ),
                            ),
                        },
                    ],
                    user_entry_address: "test_user_entry".to_string(),
                },
            )?;

            let msg_hash: Vec<u8> =
                test_user_op().get_user_op_hash("5FF137D4b0FDCD49DcA30c7CF57E578a026d2789", 5);
            let msg_to_sign = BigInt::from_bytes(msg_hash.as_slice());

            // Collect partial signatures from other parties
            let (first_sign_manual, _sig) =
                SignManual::new(msg_to_sign.clone(), completed_offline_stages[1].clone()).unwrap();

            // Collect partial signatures from other parties
            let partial_sigs_missing_last_one: Vec<PartialSignature> = completed_offline_stages
                [2..completed_offline_stages.len()]
                .iter()
                .map(|c| {
                    let (_sign, sig) = SignManual::new(msg_to_sign.clone(), c.clone()).unwrap();
                    sig
                })
                .collect();
            let (_needs_one_more_sig_sign_manual, all_but_one_sig) = first_sign_manual
                .add(&partial_sigs_missing_last_one)
                .unwrap();

            let sign_msg = ExecuteMsg::Sign {
                participants: participating_parties.iter().map(|p| *p as u8).collect(),
                user_entry_address: "test_user_entry".to_string(),
                user_entry_code_hash: "test_user_entry_code_hash".to_string(),
                entry_point: "5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string(),
                chain_id: "5".to_string(),
                user_operation: Box::from(test_user_op().clone()),
                other_partial_sigs: vec![msg::PartialSignature(
                    Secp256k1Scalar::from_slice(all_but_one_sig.0.to_bytes().as_ref()).unwrap(),
                )],
            };

            // Have the contract complete signing
            assert!(
                execute(deps.as_mut(), mock_env(), info.clone(), sign_msg).is_ok(),
                "Signing failed for participant group {:?}",
                participating_parties
            );
        }

        Ok(())
    }

    #[test]
    fn query_contract_sign_bytes_succeeds_for_all_participant_combos() -> StdResult<()> {
        let mut deps = mock_dependencies();
        let info = instantiate_contract(deps.as_mut());
        let params = Parameters {
            threshold: 1,
            share_count: 3,
        };
        let local_keys = simulate_keygen(&params);

        let all_participating_parties_combos =
            vec![vec![1, 2], vec![1, 3], vec![2, 3], vec![1, 2, 3]];
        for participating_parties in all_participating_parties_combos {
            let completed_offline_stages =
                simulate_offline_stage(&local_keys, participating_parties.as_slice());
            let completed_offline_stage_for_contract = completed_offline_stages[0].clone();
            execute(
                deps.as_mut(),
                mock_env(),
                info.clone(),
                ExecuteMsg::SetShares {
                    participants_to_completed_offline_stages: vec![
                        ParticipantsToCompletedOfflineStageParts {
                            participants: participating_parties.iter().map(|p| *p as u8).collect(),
                            completed_offline_stage: Some(
                                CompletedOfflineStageParts::from_value_hash(
                                    completed_offline_stage_for_contract,
                                    "test_user_entry_code_hash".to_string(),
                                ),
                            ),
                        },
                    ],
                    user_entry_address: "test_user_entry".to_string(),
                },
            )?;

            let hash_to_sign: Vec<u8> =
                hex::decode("0f6dbe57888cd614439bee3a4b11d13f5784b149e0a7ce91735a77b40475c48d")
                    .unwrap();
            let msg_to_sign = BigInt::from_bytes(hash_to_sign.as_slice());

            // Collect partial signatures from other parties
            let (first_sign_manual, _sig) =
                SignManual::new(msg_to_sign.clone(), completed_offline_stages[1].clone()).unwrap();

            // Collect partial signatures from other parties
            let partial_sigs_missing_last_one: Vec<PartialSignature> = completed_offline_stages
                [2..completed_offline_stages.len()]
                .iter()
                .map(|c| {
                    let (_sign, sig) = SignManual::new(msg_to_sign.clone(), c.clone()).unwrap();
                    sig
                })
                .collect();
            let (_needs_one_more_sig_sign_manual, all_but_one_sig) = first_sign_manual
                .add(&partial_sigs_missing_last_one)
                .unwrap();

            let sign_query_msg = QueryMsg::SignBytes {
                participants: participating_parties.iter().map(|p| *p as u8).collect(),
                user_entry_address: "test_user_entry".to_string(),
                user_entry_code_hash: "test_user_entry_code_hash".to_string(),
                bytes: hex::encode(hash_to_sign.clone()),
                other_partial_sigs: vec![msg::PartialSignature(
                    Secp256k1Scalar::from_slice(all_but_one_sig.0.to_bytes().as_ref()).unwrap(),
                )],
                prepend: false,
                bytes_signed_by_signers: vec!["".to_string()], // skipped when in test, as can't ask user entry for signers
                is_already_hashed: None,                       // should assume true
            };

            // Have the contract complete signing
            let _res = query(deps.as_ref(), mock_env(), sign_query_msg).unwrap();
        }

        Ok(())
    }
}
