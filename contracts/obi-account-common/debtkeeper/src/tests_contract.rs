pub const OWNER: &str = "alice";

#[cfg(test)]
mod tests {
    /* use crate::defaults::get_local_pair_contracts; */
    use super::*;
    use crate::contract::instantiate;
    use crate::tests_helpers::get_test_instantiate_message;

    macros::cosmwasm_testing_imports!(mock_dependencies, mock_env, mock_info);
    macros::cosmwasm_imports!(coin, from_binary, Coin, Uint128, Uint256);

    #[test]
    fn instantiate_test() {
        let mut deps = mock_dependencies();
        let current_env = mock_env();
        let _res = instantiate(
            deps.as_mut(),
            current_env.clone(),
            mock_info(OWNER, &[]),
            get_test_instantiate_message(
                current_env,
                Coin {
                    amount: Uint128::from(0u128),
                    denom: "ujunox".to_string(),
                },
                false,
            ),
        )
        .unwrap();
    }

    /* #[test]
    fn migrate() {
        let mut deps = mock_dependencies();
        let current_env = mock_env();
        instantiate_contract(
            &mut deps,
            current_env,
            Coin {
                amount: Uint128::from(1_000_000u128),
                denom: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                    .to_string(),
            },
        );
        let mut cfg = STATE.load(&deps.storage).unwrap();
        cfg.set_pair_contracts("EMPTY".to_string()).unwrap();
        STATE.save(&mut deps.storage, &cfg).unwrap();
        let cfg = STATE.load(&deps.storage).unwrap();
        assert_eq!(cfg.pair_contracts, vec![]);
        migrate();
        let local_contracts = get_local_pair_contracts().to_vec();
        assert_eq!(cfg.pair_contracts, local_contracts);
    } */
}
