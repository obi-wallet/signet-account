use common::common_execute_reasons::CanExecute;
use secret_cosmwasm_std::{ensure, to_binary, Addr, Binary, Deps, Env, StdError, Uint256};

#[cfg(not(test))]
use crate::contract::verify_signatures_without_nexthash;
use crate::contract::{check_tx, sign, sign_bytes, verify_user_op_fees};
use crate::errors::SecretShareSignerError;
use crate::msg::{PartialSignature, QueryMsg, UserOpFeesValidResponse};
use crate::state::get_pubkey;
use common::eth::{prep_hex_message, prepend_and_or_hash, EthUserOp};

const TEST_USER_ENTRY: &str = "test_user_entry";
const TEST_CHAIN_ID: &str = "5";

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, SecretShareSignerError> {
    match msg {
        QueryMsg::UserOpFeesValid {
            user_op,
            chain_id,
            user_entry_address,
        } => {
            deps.api.debug("Querying if user op fees valid");
            let (valid, comment) =
                match verify_user_op_fees(deps, env, user_op, chain_id, &user_entry_address) {
                    Ok(valid) => (valid, "no error".to_string()),
                    Err(e) => (false, e.to_string()),
                };
            Ok(to_binary(&UserOpFeesValidResponse { valid, comment })?)
        }
        QueryMsg::UserOpTxValid {
            user_op,
            chain_id: _,
            user_entry_address,
            user_entry_code_hash,
            sender,
        } => {
            deps.api.debug("Querying if user op tx valid");
            let can_execute: CanExecute = check_tx(
                deps,
                Addr::unchecked(sender),
                user_op,
                Addr::unchecked(user_entry_address),
                user_entry_code_hash,
            )
            .map_err(|e| StdError::generic_err(format!("{} {}", e, macros::loc_string!())))?
            .can_execute;
            match can_execute {
                CanExecute::Yes(reason) => Ok(to_binary(&UserOpFeesValidResponse {
                    valid: true,
                    comment: format!("{:?}", reason),
                })?),
                CanExecute::No(reason) => Ok(to_binary(&UserOpFeesValidResponse {
                    valid: false,
                    comment: format!("{:?}", reason),
                })?),
                CanExecute::Maybe(reason) => Ok(to_binary(&UserOpFeesValidResponse {
                    valid: false,
                    comment: format!("{:?}", reason),
                })?),
            }
        }
        QueryMsg::PassportPubkey { user_entry_address } => query_pubkey(deps, user_entry_address),
        QueryMsg::SignUserop {
            participants,
            user_entry_address,
            user_entry_code_hash,
            entry_point,
            chain_id,
            user_operation,
            other_partial_sigs,
            userop_signed_by_signers,
        } => query_sign(
            deps,
            env,
            participants,
            user_entry_address,
            user_entry_code_hash,
            entry_point,
            chain_id,
            user_operation,
            other_partial_sigs,
            userop_signed_by_signers,
            None, // enable authorized sender later
        ),
        QueryMsg::SignBytes {
            participants,
            user_entry_address,
            user_entry_code_hash,
            other_partial_sigs,
            bytes,
            bytes_signed_by_signers,
            prepend,
            is_already_hashed,
        } => query_sign_bytes(
            deps,
            env,
            participants,
            user_entry_address,
            user_entry_code_hash,
            other_partial_sigs,
            bytes,
            bytes_signed_by_signers,
            prepend,
            is_already_hashed,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn query_sign(
    deps: Deps,
    env: Env,
    participants: Vec<u8>,
    user_entry_address: String,
    user_entry_code_hash: String,
    entry_point: String,
    chain_id: String,
    user_op: EthUserOp,
    other_partial_sigs: Vec<PartialSignature>,
    // Permits require executes to manage; we can quickly verify here
    // without requiring an additional execute to add permits
    // since authorized addresses are added to user accounts
    #[allow(unused_variables)] userop_signed_by_signers: Vec<String>, // OR by authorized sender
    authorized_sender: Option<String>,
) -> Result<Binary, SecretShareSignerError> {
    let parsed_chain_id: u128 = chain_id.parse::<u128>().map_err(|e| {
        SecretShareSignerError::Std(StdError::generic_err(format!(
            "unable to parse chain id: {}",
            e
        )))
    })?;
    #[allow(unused_variables)]
    let userop_hash = user_op.hash(entry_point.clone(), Uint256::from(parsed_chain_id));
    deps.api.debug("Verifying attached userop signatures");
    let sender = match authorized_sender {
        Some(sender) => {
            deps.api.debug("Passing off to check_tx()");
            // Check whether the sender is allowed to perform the user op
            if user_entry_address == TEST_USER_ENTRY {
                ensure!(
                    chain_id == TEST_CHAIN_ID,
                    SecretShareSignerError::Unauthorized {}
                );
            } else {
                let can_execute = check_tx(
                    deps,
                    deps.api.addr_validate(&sender)?,
                    user_op.clone(),
                    deps.api.addr_validate(&user_entry_address)?,
                    #[allow(clippy::redundant_clone)]
                    user_entry_code_hash.clone(),
                )?;
                deps.api.debug("Asserting can_execute is Yes");
                assert!(matches!(can_execute.can_execute, CanExecute::Yes(_)));
            }
            deps.api
                .debug("Established authorization: is authorized sender");
            Some(sender)
        }
        None => {
            #[cfg(not(test))]
            assert!(verify_signatures_without_nexthash(
                deps,
                hex::encode(userop_hash),
                userop_signed_by_signers,
                deps.api.addr_validate(&user_entry_address)?,
                user_entry_code_hash,
            )
            .map_err(|e| SecretShareSignerError::Std(e.into()))?);
            deps.api
                .debug("Established authorization: is owner (signers > threshold)");
            None
        }
    };
    deps.api.debug("Passing off to sign()");
    let signature = sign(
        deps,
        env,
        sender,
        participants,
        user_entry_address.as_str(),
        entry_point.as_str(),
        chain_id.as_str(),
        &user_op,
        other_partial_sigs,
    )?;
    to_binary(&signature).map_err(SecretShareSignerError::Std)
}

#[allow(clippy::too_many_arguments)]
fn query_sign_bytes(
    deps: Deps,
    env: Env,
    participants: Vec<u8>,
    user_entry_address: String,
    #[allow(unused_variables)] user_entry_code_hash: String,
    other_partial_sigs: Vec<PartialSignature>,
    // Permits require executes to manage; we can quickly verify here
    // without requiring an additional execute to add permits
    // since authorized addresses are added to user accounts
    bytes: String,
    #[allow(unused_variables)] bytes_signed_by_pubkey_hex: Vec<String>,
    prepend: bool,
    is_already_hashed: Option<bool>,
) -> Result<Binary, SecretShareSignerError> {
    deps.api.debug("Verifying attached userop signature");
    let is_already_hashed = is_already_hashed.unwrap_or(!prepend);

    let hash_bytes = if is_already_hashed {
        #[allow(clippy::redundant_clone)]
        prep_hex_message(bytes.clone())
    } else {
        #[allow(clippy::redundant_clone)]
        prepend_and_or_hash(prep_hex_message(bytes.clone()), prepend)
    };
    #[cfg(not(test))]
    assert!(verify_signatures_without_nexthash(
        deps,
        bytes,
        bytes_signed_by_pubkey_hex,
        deps.api.addr_validate(&user_entry_address)?,
        user_entry_code_hash
    )
    .map_err(|e| SecretShareSignerError::Std(e.into()))?);
    deps.api.debug("Passing off to sign_bytes()");
    let signature = sign_bytes(
        deps,
        env,
        participants,
        user_entry_address.as_str(),
        other_partial_sigs,
        hash_bytes,
    )?;
    to_binary(&signature).map_err(SecretShareSignerError::Std)
}

fn query_pubkey(deps: Deps, user_entry_address: String) -> Result<Binary, SecretShareSignerError> {
    deps.api.debug("Querying pubkey");
    to_binary(&get_pubkey(
        deps.storage,
        &deps.api.addr_canonicalize(&user_entry_address)?,
    ))
    .map_err(SecretShareSignerError::Std)
}
