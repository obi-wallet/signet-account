#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;

#[derive(Copy)]
#[uniserde::uniserde]
pub enum CanExecute {
    Yes(CanExecuteReason),
    No(CannotExecuteReason),
    Maybe(PendingReason),
}

#[derive(Copy)]
#[uniserde::uniserde]
pub enum PendingReason {
    NotBlocklisted = 200,
    AllowlistPendingSpendCheck = 201,
    NoFundsPendingMessageCheck = 202,
    SpendlimitMessageRiderPendingSpendCheck = 203,
}

#[derive(Copy)]
#[uniserde::uniserde]
pub enum CanExecuteReason {
    OwnerNoDelay = 0,
    OwnerWithDebtButNoFundsSpent = 1,
    OwnerDelayComplete = 2,
    Allowance = 3,
    AllowanceAndAllowlist = 4,
    AllowanceWithReset = 5,
    AllowanceWithAllowlistAndReset = 6,
    Beneficiary = 7,
    BeneficiaryWithAllowlist = 8,
    BeneficiaryWithReset = 9,
    BeneficiaryWithAllowlistAndReset = 10,
    BeneficiaryFullControl = 12,
    NoFundsAndAllowlist = 13,
    SessionkeyAsOwner = 14,
    SessionkeyAsOwnerWithDebtButNoFundsSpent = 15,
    AllowanceWithBlanketAuthorizedToken = 16,
    SubrulesPass = 17,
}

pub fn readable(code: u8) -> String {
    let yes_reasons = vec![
        "Caller is owner. No delay required. Debt repay attached if applicable.",
        "Caller is owner and has debt but no funds are being spent.",
        "Caller is owner. Required delay complete. Debt repay attached if applicable.",
        "Caller is a permissioned address spending funds within recurring spend limit, and message is allowed by spend limit rider. Debt repay attached if applicable.",
        "Caller is a permissioned address spending funds, and message is on the allowlist. Debt repay attached if applicable.",
        "Caller is a permissioned address spending funds, with the spendlimit now resetting, and message is allowed by spend limit rider. Debt repay attached if applicable.",
        "Caller is a permissioned address spending funds, with the spendlimit now resetting, and message is on the allowlist. Debt repay attached if applicable.",
        "Caller is an active inheritance beneficiary, and message is allowed by spend limit rider. Debt repay attached if applicable.",
        "Caller is an active inheritance beneficiary, and message is on allowlist. Debt repay attached if applicable.",
        "Caller is an active inheritance beneficiary, and message is on allowlist. Debt repay attached if applicable.",
        "Caller is an active inheritance beneficiary, and message is on allowlist. Debt repay attached if applicable.",
        "Caller is an active inheritance beneficiary with full control. Debt repay attached if applicable.",
        "Caller is not spending funds, and message is on the allowlist. Debt repay attached if applicable.",
        "Caller is a session key with no restrictions. Debt repay attached if applicable.",
        "Caller is a session key with no restrictions. Debt exists, but no funds are being spent.",
        "Caller is a permissioned address spending a blanket-authorized token.",
        "Caller is a sessionkey and all sub_rules pass.",
    ];
    let no_reasons = vec![
        "Caller is owner, but has debt that is not payable in asset being spent.",
        "Caller is owner, but the delay for this action has not passed.",
        "Caller is owner, but this message is on the blocklist. Remove the block first.",
        "No matching rule found allowing this caller to take this action.",
        "Caller is a permissioned address spending funds, but this message is not on the allowlist and not covered by spend limit rider.",
        "Caller is a permissioned address, but this transaction exceeds the available spend limit for the current period.",
        "Caller is a permissioned address, but this message is on the blocklist.",
        "Caller is an inheritance beneficiary, but inheritance is not yet active.",
        "Caller is an active inheritance beneficiary, but this transaction exceeds the available limit for the current drip period.",
        "Caller is an active inheritance beneficiary, but this message is not on the allowlist and not covered by beneficiary rider.",
        "Caller is an active inheritance beneficiary, but this message is on the blocklist.",
        "Caller is a permissioned address, and multiple messages are not yet supported for permissioned addresses.",
        "Transaction uses funds, but no spendlimit gatekeeper is attached.",
        "Caller is a sessionkey but sub_rules do not pass.",
        "This message is blocklisted for this user.",
        "This message is blocklisted for all users.",
        "Unspecified reason.",
        "This rule has expired."
    ];
    let maybe_reasons = [
        "TEMPORARY STATUS: Message is not blocklisted.",
        "TEMPORARY STATUS: Allowlisted; pending spend check.",
        "TEMPORARY STATUS: No funds spent; pending message check.",
        "TEMPORARY STATUS: Spendlimit rider; pending spend check.",
    ];
    match code {
        200..=203 => maybe_reasons[code as usize - 200].to_string(),
        100..=117 => no_reasons[code as usize - 100].to_string(),
        0..=17 => yes_reasons[code as usize].to_string(),
        _ => panic!("Unknown reason"),
    }
}

#[derive(Copy)]
#[uniserde::uniserde]
pub enum CannotExecuteReason {
    OwnerDebtUnpayable = 100,
    OwnerDelayInProgress = 101,
    OwnerMessageBlocklisted = 102,
    NoMatchingRule = 103,
    AllowanceButNoAllowlist = 104,
    AllowanceExceeded = 105,
    AllowanceMessageBlocklisted = 106,
    BeneficiaryInheritanceNotActive = 107,
    BeneficiaryButNoAllowlist = 108,
    BeneficiaryDripExceeded = 109,
    BeneficiaryMessageBlocklisted = 110,
    MultipleMessagesNotYetSupported = 111,
    NoSpendlimitGatekeeper = 112,
    SubrulesFail = 113,
    Blocklist = 114,
    GlobalBlocklist = 115,
    Unspecified = 116,
    RuleExpired = 117,
}
