use std::env::current_dir;
use std::fs::create_dir_all;

use classes::msg_user_state::LastActivityResponse;
use classes::msg_user_state::{ExecuteMsg, InstantiateMsg, QueryMsg};
use classes::user_state::AbstractionRules;
use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(LastActivityResponse), &out_dir);
    export_schema(&schema_for!(AbstractionRules), &out_dir);
}
