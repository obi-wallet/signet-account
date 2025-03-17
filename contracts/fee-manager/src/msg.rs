#[cfg(feature = "cosmwasm")]
use cosmwasm_schema::cw_serde;

#[uniserde::uniserde]
pub struct InstantiateMsg {
    pub fee_divisors: (String, u64),
    pub fee_pay_addresses: (String, String),
}

#[uniserde::uniserde]
pub enum ExecuteMsg {
    SetFee {
        chain_id: String,
        new_fee_divisor: u64,
    },
    SetFeeAddress {
        chain_id: String,
        new_fee_address: String,
    },
}

#[uniserde::uniserde]
pub struct MigrateMsg {}

#[uniserde::uniserde]
pub enum QueryMsg {
    FeeDetails { chain_id: String },
}

#[uniserde::uniserde]
pub struct FeeDetailsResponse {
    pub fee_divisor: u64,
    pub fee_pay_address: String,
}
