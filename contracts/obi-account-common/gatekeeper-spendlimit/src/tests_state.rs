#[cfg(test)]
mod tests {
    macros::cosmwasm_testing_imports!(mock_dependencies, mock_env);
    macros::cosmwasm_imports!(Addr);
    use classes::gatekeeper_common::{is_legacy_owner, LEGACY_OWNER};

    #[test]
    fn assert_is_legacy_owner() {
        let _now_env = mock_env();
        let mut deps = mock_dependencies();
        let owner: &str = "bob";
        LEGACY_OWNER
            .save(deps.as_mut().storage, &Some(owner.to_string()))
            .unwrap();
        assert!(is_legacy_owner(deps.as_ref(), Addr::unchecked(owner.to_string())).unwrap());
    }

    // these need to be updated since support for osmo
    // but their cases are covered in integration test
    /*
    #[test]
    fn daily_spend_limit() {
        let deps = mock_dependencies();
        let _owner: &str = "bob";
        let spender = "owner";
        let bad_spender: &str = "medusa";
        let dt = NaiveDateTime::new(
            NaiveDate::from_ymd(2022, 6, 3),
            NaiveTime::from_hms_milli(12, 00, 00, 000),
        );
        let mut now_env = mock_env();
        now_env.block.time = Timestamp::from_seconds(dt.timestamp() as u64);
        // 3 day spend limit period
        let _config = State {
            last_activity: now_env.block.time.seconds(),
        };
        let mut spendlimits = Spendlimits {
            permissioned_addresses: vec![PermissionedAddress::new(
                Addr::unchecked(spender),
                PermissionedAddressParams {
                    address: spender.to_string(),
                    cooldown: dt.timestamp() as u64,
                    period_type: PeriodType::Days,
                    period_multiple: 3,
                    offset: 0u32,
                    spend_limits: vec![CoinBalance {
                        amount: 100_000_000u128,
                        denom:
                            "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                                .to_string(),
                        limit_remaining: 100_000_000u128,
                        current_balance: 0u128,
                    }],
                    denom: Some("true".to_string()),
                    default: Some(true),
                    inheritance_records: vec![],
                },
                false,
            )],
        };

        spendlimits
            .check_and_update_spend_limits(
                deps.as_ref(),
                now_env.clone(),
                "LOCAL_TEST".to_string(),
                now_env.block.time,
                spender.to_string(),
                vec![Coin {
                    denom: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                        .to_string(),
                    amount: Uint128::from(1_000_000u128),
                }],
            )
            .unwrap();
        spendlimits
            .check_and_update_spend_limits(
                deps.as_ref(),
                now_env.clone(),
                "LOCAL_TEST".to_string(),
                now_env.block.time,
                bad_spender.to_string(),
                vec![Coin {
                    denom: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                        .to_string(),
                    amount: Uint128::from(1_000_000u128),
                }],
            )
            .unwrap_err();
        // now we shouldn't be able to total over our spend limit
        spendlimits
            .check_and_update_spend_limits(
                deps.as_ref(),
                now_env.clone(),
                "LOCAL_TEST".to_string(),
                now_env.block.time,
                spender.to_string(),
                vec![Coin {
                    denom: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                        .to_string(),
                    amount: Uint128::from(99_500_000u128),
                }],
            )
            .unwrap_err();
        // our even 1 over our spend limit
        spendlimits
            .check_and_update_spend_limits(
                deps.as_ref(),
                now_env.clone(),
                "LOCAL_TEST".to_string(),
                now_env.block.time,
                spender.to_string(),
                vec![Coin {
                    denom: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                        .to_string(),
                    amount: Uint128::from(99_000_001u128),
                }],
            )
            .unwrap_err();

        // but go 3 days + 1 second into the future and we should
        let mut env_future = now_env;
        env_future.block.time =
            Timestamp::from_seconds(env_future.block.time.seconds() + 259206u64);
        spendlimits
            .check_and_update_spend_limits(
                deps.as_ref(),
                env_future.clone(),
                "LOCAL_TEST".to_string(),
                env_future.block.time,
                spender.to_string(),
                vec![Coin {
                    denom: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                        .to_string(),
                    amount: Uint128::from(100_000_000u128),
                }],
            )
            .unwrap();
    }

    #[test]
    fn monthly_spend_limit() {
        let deps = mock_dependencies();
        let _owner: &str = "bob";
        let spender = "owner";
        let bad_spender: &str = "medusa";
        let dt = NaiveDateTime::new(
            NaiveDate::from_ymd(2022, 6, 3),
            NaiveTime::from_hms_milli(12, 00, 00, 000),
        );
        let mut now_env = mock_env();
        now_env.block.time = Timestamp::from_seconds(dt.timestamp() as u64);

        // Let's do a 38 month spend limit period
        // and for kicks use a contract address for LOOP
        let _config = State {
            last_activity: now_env.block.time.seconds(),
        };
        let mut spendlimits = Spendlimits {
            permissioned_addresses: vec![PermissionedAddress::new(
                Addr::unchecked(spender),
                PermissionedAddressParams {
                    address: spender.to_string(),
                    cooldown: dt.timestamp() as u64,
                    period_type: PeriodType::Months,
                    period_multiple: 38,
                    offset: 0u32,
                    spend_limits: vec![CoinBalance {
                        amount: 100_000_000u128,
                        denom:
                            "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                                .to_string(),
                        current_balance: 0u128,
                        limit_remaining: 100_000_000u128,
                    }],
                    denom: None, // 100 JUNO, 100 axlUSDC, 9000 LOOP
                    default: Some(true),
                    inheritance_records: vec![],
                },
                false,
            )],
        };

        spendlimits
            .check_and_update_spend_limits(
                deps.as_ref(),
                now_env.clone(),
                "LOCAL_TEST".to_string(),
                now_env.block.time,
                spender.to_string(),
                vec![Coin {
                    denom: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                        .to_string(),
                    amount: Uint128::from(1_000_000u128),
                }],
            )
            .unwrap();
        spendlimits
            .check_and_update_spend_limits(
                deps.as_ref(),
                now_env.clone(),
                "LOCAL_TEST".to_string(),
                now_env.block.time,
                bad_spender.to_string(),
                vec![Coin {
                    denom: "ujuno".to_string(),
                    amount: Uint128::from(1_000_000u128),
                }],
            )
            .unwrap_err();
        spendlimits
            .check_and_update_spend_limits(
                deps.as_ref(),
                now_env.clone(),
                "LOCAL_TEST".to_string(),
                now_env.block.time,
                spender.to_string(),
                vec![Coin {
                    denom: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                        .to_string(),
                    amount: Uint128::from(99_000_001u128),
                }],
            )
            .unwrap_err();

        // but go 38 months (minus a couple of days - reset is the 1st, not the 3rd)
        // into the future and we should be able to spend
        let dt = NaiveDateTime::new(
            NaiveDate::from_ymd(2025, 8, 1),
            NaiveTime::from_hms_milli(12, 00, 00, 000),
        );
        let mut env_future = mock_env();
        env_future.block.time = Timestamp::from_seconds(dt.timestamp() as u64);
        spendlimits
            .check_and_update_spend_limits(
                deps.as_ref(),
                now_env,
                "LOCAL_TEST".to_string(),
                env_future.block.time,
                spender.to_string(),
                vec![Coin {
                    denom: "ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4"
                        .to_string(),
                    amount: Uint128::from(99_000_001u128),
                }],
            )
            .unwrap();
    }
    */
}
