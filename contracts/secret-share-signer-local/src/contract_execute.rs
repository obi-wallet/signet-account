use classes::msg_user_entry::UserAccountAddressResponse;
use secret_cosmwasm_std::{
    ensure, to_binary, Addr, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::contract::sign;
use crate::errors::SecretShareSignerError;
use crate::msg::{
    ExecuteMsg, LegacyOwnerResponse, PartialSignature, ParticipantsToCompletedOfflineStageParts,
    UserAccountQueryMsg,
};
use crate::state::insert_completed_offline_stage;
use common::eth::EthUserOp;

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, SecretShareSignerError> {
    match msg {
        ExecuteMsg::SetShares {
            participants_to_completed_offline_stages,
            user_entry_address,
        } => {
            deps.api.debug("checking if account owner");
            ensure!(
                is_account_owner(
                    deps.as_ref(),
                    info.sender.clone(),
                    user_entry_address.clone(),
                    participants_to_completed_offline_stages[0]
                        .completed_offline_stage
                        .clone()
                        .unwrap()
                        .user_entry_code_hash,
                )?,
                SecretShareSignerError::Unauthorized {}
            );
            set_share(
                deps,
                env,
                info,
                user_entry_address,
                participants_to_completed_offline_stages,
            )
        }
        ExecuteMsg::Sign {
            participants,
            user_entry_address,
            user_entry_code_hash,
            entry_point,
            chain_id,
            user_operation,
            other_partial_sigs,
        } => execute_sign(
            deps,
            env,
            info,
            participants,
            user_entry_address,
            user_entry_code_hash,
            entry_point,
            chain_id,
            user_operation.as_ref(),
            other_partial_sigs,
        ),
    }
}

fn is_account_owner(
    deps: Deps,
    sender: Addr,
    user_entry_addr: String,
    user_entry_code_hash: String,
) -> Result<bool, SecretShareSignerError> {
    if user_entry_addr == *"test_user_entry" {
        ensure!(
            user_entry_code_hash == *"test_user_entry_code_hash",
            SecretShareSignerError::Unauthorized {}
        );
        return Ok(true);
    }
    let valid_user_entry_addr = deps.api.addr_validate(&user_entry_addr)?;
    let res: StdResult<UserAccountAddressResponse> = deps.querier.query_wasm_smart(
        user_entry_code_hash,
        valid_user_entry_addr,
        &classes::msg_user_entry::QueryMsg::UserAccountAddress {},
    );
    let user_account_address_response: UserAccountAddressResponse = res?;
    let legacy_owner_response: LegacyOwnerResponse = deps.querier.query_wasm_smart(
        user_account_address_response.user_account_code_hash,
        user_account_address_response.user_account_address,
        &UserAccountQueryMsg::LegacyOwner {},
    )?;
    deps.api.debug("LEGACY OWNER CHECKED in SIGNER");
    Ok(legacy_owner_response.legacy_owner == sender)
}

fn set_share(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    user_entry_address: String,
    participants_to_completed_offline_stages: Vec<ParticipantsToCompletedOfflineStageParts>,
) -> Result<Response, SecretShareSignerError> {
    deps.api.debug("entered set_share");
    for participants_to_completed_offline_stage in participants_to_completed_offline_stages {
        deps.api.debug("adding participant");
        insert_completed_offline_stage(
            deps.storage,
            &deps.api.addr_canonicalize(user_entry_address.as_str())?,
            participants_to_completed_offline_stage.participants,
            participants_to_completed_offline_stage.completed_offline_stage,
        )?;
    }
    Ok(Response::default())
}

#[allow(clippy::too_many_arguments)]
fn execute_sign(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    participants: Vec<u8>,
    user_entry_address: String,
    user_entry_code_hash: String,
    entry_point: String,
    chain_id: String,
    user_op: &EthUserOp,
    other_partial_sigs: Vec<PartialSignature>,
) -> Result<Response, SecretShareSignerError> {
    if user_entry_address == *"test_user_entry" {
        ensure!(chain_id == "5", SecretShareSignerError::Unauthorized {});
    } else {
        let res: UserAccountAddressResponse = deps.querier.query_wasm_smart(
            user_entry_code_hash,
            user_entry_address.clone(),
            &classes::msg_user_entry::QueryMsg::UserAccountAddress {},
        )?;
        let res2: LegacyOwnerResponse = deps.querier.query_wasm_smart(
            res.user_account_code_hash,
            res.user_account_address,
            &classes::msg_user_account::QueryMsg::LegacyOwner {},
        )?;
        ensure!(
            info.sender == res2.legacy_owner,
            SecretShareSignerError::Unauthorized {}
        );
    }
    let signature = sign(
        deps.as_ref(),
        env,
        Some(info.sender.to_string()),
        participants,
        user_entry_address.as_str(),
        entry_point.as_str(),
        chain_id.as_str(),
        user_op,
        other_partial_sigs,
    )?;
    // todo: update spendlimit here, if appropriate
    Ok(Response::default().set_data(to_binary(&signature)?))
}
