use crate::permissioned_address::PermissionedAddressParams;
use common::authorization::Authorization;
macros::cosmwasm_imports!(Binary);

#[uniserde::uniserde]
pub enum Rule {
    Spendlimit(PermissionedAddressParams),
    Inheritance(PermissionedAddressParams),
    Allow(Authorization),
    Block(Authorization), // blocklist uses account address as actor for universal block
    Custom((String, Binary)), // actor and binary of rule; deserialization is up to gatekeeper
}
