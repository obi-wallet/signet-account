use std::env::current_dir;
use std::fs::create_dir_all;

use classes::{
    gatekeeper_common::LegacyOwnerResponse,
    msg_user_account::{ExecuteMsg, InstantiateMsg, QueryMsg},
    user_account::{
        CanExecuteResponse, GatekeeperCodeIds, GatekeeperContractsResponse, PendingOwnerResponse,
        SignersResponse,
    },
};
use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(LegacyOwnerResponse), &out_dir);
    export_schema(&schema_for!(PendingOwnerResponse), &out_dir);
    export_schema(&schema_for!(SignersResponse), &out_dir);
    export_schema(&schema_for!(GatekeeperContractsResponse), &out_dir);
    export_schema(&schema_for!(CanExecuteResponse), &out_dir);
    export_schema(&schema_for!(GatekeeperCodeIds), &out_dir);
}
