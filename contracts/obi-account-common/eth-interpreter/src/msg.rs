cosmwasm_imports!(Binary);
use classes::simulation::Asset;
use common::eth::EthUserOp;
use cosmwasm_schema::QueryResponses;
use macros::cosmwasm_imports;
use uniserde::uniserde;

#[uniserde]
pub enum ExecuteMsg {
    WrappedMigrate { new_code_id: u64, msg: Binary },
}

#[uniserde]
pub struct InstantiateMsg {}

#[uniserde]
pub struct MigrateMsg {}

#[uniserde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ParseUserOpResponse)]
    ParseUserOp { user_op: EthUserOp },
}

#[uniserde::uniserde]
pub struct ParseUserOpResponse {
    pub spend: Vec<Asset>,
    pub contract_address: Option<String>,
    pub fee: Vec<Asset>,
    pub fee_recipient: Option<String>,
    pub fields: Option<Vec<(String, String)>>,
    pub function_signatures: Vec<String>,
}
