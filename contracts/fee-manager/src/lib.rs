#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub mod contract;
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[cfg(test)]
pub mod contract_tests;
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub mod msg;
#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
pub mod state;
