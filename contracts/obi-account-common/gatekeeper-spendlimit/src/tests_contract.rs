pub const LEGACY_OWNER_STR: &str = "alice";
pub const _PERMISSIONED_ADDRESS: &str = "hotcarl";

#[cfg(test)]
mod tests {

    use super::*;
    use crate::contract::instantiate;
    use crate::tests_helpers::get_test_instantiate_message;

    macros::cosmwasm_testing_imports!(mock_dependencies, mock_env, mock_info);
    macros::cosmwasm_imports!(Api);

    #[test]
    fn instantiate_and_modify_owner() {
        let mut deps = mock_dependencies();
        let current_env = mock_env();
        let _res = instantiate(
            deps.as_mut(),
            current_env.clone(),
            mock_info(LEGACY_OWNER_STR, &[]),
            get_test_instantiate_message(current_env),
        )
        .unwrap();
    }
}
