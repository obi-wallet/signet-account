pub const LEGACY_OWNER: &str = "alice";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{instantiate, query_legacy_owner};

    use crate::tests_helpers::get_test_instantiate_message;

    use classes::asset_unifier::LegacyOwnerResponse;

    macros::cosmwasm_imports!(Coin, Uint128);
    macros::cosmwasm_testing_imports!(mock_dependencies, mock_env, mock_info);

    #[test]
    fn instantiate_and_modify_owner() {
        let mut deps = mock_dependencies();
        let current_env = mock_env();

        let _res = instantiate(
            deps.as_mut(),
            current_env.clone(),
            mock_info(LEGACY_OWNER, &[]),
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

        // ensure expected config
        let expected = LegacyOwnerResponse {
            legacy_owner: LEGACY_OWNER.to_string(),
        };
        assert_eq!(query_legacy_owner(deps.as_ref()).unwrap(), expected);

        // update owner
        // not implemented
    }
}
