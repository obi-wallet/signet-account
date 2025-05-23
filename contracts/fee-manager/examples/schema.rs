use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

#[cfg(not(feature = "cosmwasm"))]
use fee_manager::msg::{ExecuteMsg, FeeDetailsResponse, InstantiateMsg, QueryMsg};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    #[cfg(not(feature = "cosmwasm"))]
    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    #[cfg(not(feature = "cosmwasm"))]
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    #[cfg(not(feature = "cosmwasm"))]
    export_schema(&schema_for!(QueryMsg), &out_dir);
    #[cfg(not(feature = "cosmwasm"))]
    export_schema(&schema_for!(FeeDetailsResponse), &out_dir);
}
