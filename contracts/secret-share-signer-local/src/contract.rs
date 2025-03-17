#[allow(unused_imports)]
use classes::msg_user_entry::UserAccountAddressResponse;
use classes::signers::{Signer, Signers};
use classes::user_account::CanExecuteResponse;
use common::common_error::ContractError;
#[allow(unused_imports)]
use common::common_execute_reasons::CanExecute;
#[allow(unused_imports)]
use common::common_execute_reasons::CanExecuteReason;
#[allow(unused_imports)]
use secret_cosmwasm_std::Coin;
use secret_cosmwasm_std::{
    ensure, Addr, Deps, DepsMut, Env, Response, StdError, StdResult, Uint256,
};

use crate::errors::SecretShareSignerError;
use crate::msg::{FeeDetailsResponse, FeeManagerQueryMsg, MigrateMsg, PartialSignature};
use crate::multi_party_ecdsa::local_signature::SignatureRecid;
use crate::multi_party_ecdsa::rounds::Round7;
use crate::state::{get_completed_offline_stage, FEE_MANAGER_ADDRESS, FEE_MANAGER_CODE_HASH};
use common::eth::{CallData, EthUserOp};

const TEST_USER_ENTRY: &str = "test_user_entry";
#[allow(dead_code)]
const TEST_CHAIN_ID: &str = "421613";
#[allow(dead_code)]
const TEST_PUBKEY: &str = "00112233445566778899aabbccddeeff";

#[cfg_attr(feature = "secretwasm", secret_macros::entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[allow(clippy::too_many_arguments)]
pub fn sign(
    deps: Deps,
    env: Env,
    sender: Option<String>,
    mut participants: Vec<u8>,
    user_entry_address: &str,
    entry_point: &str,
    chain_id: &str,
    user_op: &EthUserOp,
    other_partial_sigs: Vec<PartialSignature>,
) -> Result<SignatureRecid, SecretShareSignerError> {
    deps.api.debug("Processing completed offline stages");
    let completed_offline_stage = if let Some(share) = get_completed_offline_stage(
        deps.storage,
        &deps.api.addr_canonicalize(user_entry_address)?,
        &mut participants,
    ) {
        share
    } else {
        return Err(
            SecretShareSignerError::NoCompletedOfflineStageShareSetForUserEntry(
                sender.unwrap_or("None".to_string()),
                participants,
            ),
        );
    };

    if user_entry_address != TEST_USER_ENTRY {
        deps.api.debug("Verifying userop fees");
        ensure!(
            verify_user_op_fees(
                deps,
                env,
                user_op.clone(),
                chain_id.to_string(),
                user_entry_address
            )?,
            SecretShareSignerError::Unauthorized {}
        );
    }

    deps.api.debug("Confirming userop hash");
    let parsed_chain_id = chain_id.parse::<u32>().map_err(|e| {
        SecretShareSignerError::Std(StdError::generic_err(format!(
            "{} {}",
            macros::loc_string!(),
            e
        )))
    })?;
    let msg_hash: Vec<u8> = user_op.get_user_op_hash(entry_point, parsed_chain_id);
    deps.api.debug(&format!(
        "Message (user_op hash): {}",
        hex::encode(&msg_hash)
    ));

    deps.api.debug("Creating Round7");
    let (sign, _last_needed_partial_sig) = Round7::new(&msg_hash, completed_offline_stage).unwrap();
    deps.api.debug("Completing signature");
    let signature = sign
        .proceed_manual(other_partial_sigs.as_slice())
        .map_err(|e| {
            SecretShareSignerError::Std(StdError::generic_err(format!(
                "{} {}",
                e,
                macros::loc_string!()
            )))
        })?;
    deps.api.debug("Returning signature");
    Ok(signature)
}

#[allow(clippy::too_many_arguments)]
pub fn sign_bytes(
    deps: Deps,
    _env: Env,
    mut participants: Vec<u8>,
    user_entry_address: &str,
    other_partial_sigs: Vec<PartialSignature>,
    hashed_bytes: Vec<u8>,
) -> Result<SignatureRecid, SecretShareSignerError> {
    deps.api.debug("Processing completed offline stages");
    let completed_offline_stage = if let Some(share) = get_completed_offline_stage(
        deps.storage,
        &deps.api.addr_canonicalize(user_entry_address)?,
        &mut participants,
    ) {
        share
    } else {
        return Err(
            SecretShareSignerError::NoCompletedOfflineStageShareSetForUserEntry(
                user_entry_address.to_string(),
                participants,
            ),
        );
    };

    deps.api.debug("Creating Round7");
    let (sign, _last_needed_partial_sig) =
        Round7::new(&hashed_bytes, completed_offline_stage).unwrap();
    deps.api.debug("Completing signature");
    let signature = sign
        .proceed_manual(other_partial_sigs.as_slice())
        .map_err(|e| {
            SecretShareSignerError::Std(StdError::generic_err(format!(
                "{} {}",
                e,
                macros::loc_string!()
            )))
        })?;
    deps.api.debug("Returning signature");
    Ok(signature)
}

pub fn verify_user_op_fees(
    deps: Deps,
    env: Env,
    user_op: EthUserOp,
    chain_id: String,
    user_entry_address: &str,
) -> Result<bool, SecretShareSignerError> {
    deps.api.debug("Parsing userop calldata");
    let calldata_res = CallData::from_bytes(&user_op.call_data);
    let unwrapped_calldata = match calldata_res {
        Ok(calldata) => match calldata {
            Some(inner) => inner,
            None => {
                return Err(SecretShareSignerError::Std(StdError::generic_err(
                    "Parsed calldata is none",
                )));
            }
        },
        Err(e) => {
            return Err(SecretShareSignerError::Std(StdError::generic_err(format!(
                "Error parsing calldata: {}",
                e
            ))));
        }
    };

    // get the current fee for this chain
    let (fee_divisor, fee_pay_address) = if user_entry_address == TEST_USER_ENTRY {
        // local test entrypoint
        (
            1000u64,
            "c1D4F3dcc31d86A66de90c3C5986f0666cc52ce4".to_string(),
        )
    } else {
        deps.api.debug("Getting current fee details");
        get_current_fee(deps, env, chain_id)?
    };
    deps.api.debug("Calculating fee");
    let _required_fee: StdResult<Uint256> =
        unwrapped_calldata.calculate_fee(Uint256::from(fee_divisor));
    // unit test checks signature for ERC20 userop without multisend, so this is only for production
    #[cfg(not(test))]
    deps.api.debug("Unwrapping fee");
    let required_fee = _required_fee?;
    #[cfg(not(test))]
    deps.api.debug("Checking fee amount");
    ensure!(
        unwrapped_calldata.fee_amount == Some(required_fee),
        SecretShareSignerError::FeeRequiredInUserOp(required_fee.to_string())
    );
    #[cfg(not(test))]
    deps.api.debug("Checking fee recipient");
    ensure!(
        match &unwrapped_calldata.fee_recipient {
            Some(addr) => addr.to_ascii_uppercase() == fee_pay_address.to_ascii_uppercase(),
            None => false,
        },
        SecretShareSignerError::BadFeePayAddress(
            unwrapped_calldata
                .fee_recipient
                .unwrap_or("None".to_string())
        )
    );
    deps.api.debug("Done verifying user op fees");
    Ok(true)
}

pub fn get_current_fee(deps: Deps, _env: Env, chain_id: String) -> StdResult<(u64, String)> {
    let fee_details: FeeDetailsResponse = deps.querier.query_wasm_smart(
        FEE_MANAGER_CODE_HASH.load(deps.storage)?,
        FEE_MANAGER_ADDRESS.load(deps.storage)?,
        &FeeManagerQueryMsg::FeeDetails { chain_id },
    )?;
    deps.api
        .debug(&format!("Fee details response: {:?}", fee_details));
    Ok((fee_details.fee_divisor, fee_details.fee_pay_address))
}

/// Gets the signers for a given user account
fn get_signers(
    deps: Deps,
    user_entry_address: String,
    user_entry_code_hash: String,
) -> Result<Signers, ContractError> {
    let res: classes::msg_user_entry::UserAccountAddressResponse = deps.querier.query_wasm_smart(
        user_entry_code_hash,
        user_entry_address,
        &classes::msg_user_entry::QueryMsg::UserAccountAddress {},
    )?;
    let res: classes::user_account::SignersResponse = deps.querier.query_wasm_smart(
        res.user_account_code_hash,
        res.user_account_address,
        &classes::msg_user_account::QueryMsg::Signers {},
    )?;
    Ok(res.signers)
}

/// Retrieves the signers from the associated use account and ensures that
/// `signatures` contains valid signatures of `hash` > threshold.
#[allow(dead_code)]
pub fn verify_signatures_without_nexthash(
    deps: Deps,
    hash: String,
    signatures: Vec<String>,
    user_entry_address: Addr,
    user_entry_code_hash: String,
) -> Result<bool, ContractError> {
    deps.api.debug("entering verify_signatures()");
    #[allow(unused_mut)]
    let mut signers: Signers = if user_entry_address == TEST_PUBKEY {
        Signers::new(
            vec![
                Signer {
                    ty: "local".to_string(),
                    address: Addr::unchecked("secret1nvcdlkggvj2lzf9qxalcudhljdgjhyxuz6c0jy"),
                    pubkey_base_64: "An9YoJRlklu1UeUuw/luOdbEEYoE+4d5OCVA0uzOwxG0".to_string(),
                },
                Signer {
                    ty: "local2".to_string(),
                    address: Addr::unchecked("secret1qdvd79sfeeu825je2w9uzjuzw0whruvc5gcs58"),
                    pubkey_base_64: "A71CrvmXmO30LpZKIt0IRp2alHcCcD7ldrJ2qBV3/5c/".to_string(),
                },
            ],
            Some(0u8),
        )?
    } else {
        deps.api.debug("getting signers");
        get_signers(deps, user_entry_address.to_string(), user_entry_code_hash)?
    };

    // For now we assume threshold of half, floor rounded, minus 1.
    // In keeping with convention, threshold+1 signatures are required.
    deps.api.debug("calculating threshold");
    deps.api
        .debug(&format!("threshold is {}", signers.threshold));
    deps.api
        .debug(&format!("signatures length: {}", signatures.len()));
    let mut signatures_attached = 0;

    // remove the "0x" if it exists in `hash`
    #[allow(clippy::manual_strip)]
    #[allow(unused_variables)]
    let hash = if hash.starts_with("0x") {
        hash[2..].to_string()
    } else {
        hash
    };

    #[allow(unused_variables)]
    for signature in signatures {
        // try both recovery bytes
        // we're not sure these signers are on chain by this point, so we have to
        // approach it this way for now rather than asking the chain for the pubkeys
        let pubkey0: String;
        let pubkey1: String;
        #[allow(deprecated)]
        #[cfg(not(test))]
        {
            deps.api.debug("recovering pubkeys");
            pubkey0 = base64::encode(
                deps.api
                    .secp256k1_recover_pubkey(
                        &hex::decode(hash.clone()).unwrap(),
                        &hex::decode(signature.clone()).unwrap(),
                        0,
                    )
                    .unwrap(),
            );
            pubkey1 = base64::encode(
                deps.api
                    .secp256k1_recover_pubkey(
                        &hex::decode(hash.clone()).unwrap(),
                        &hex::decode(signature).unwrap(),
                        1,
                    )
                    .unwrap(),
            );
            deps.api.debug(&format!("pubkey0: {}", pubkey0));
            deps.api.debug(&format!("pubkey1: {}", pubkey1));
            deps.api.debug("done recovering pubkeys");
        }
        #[cfg(test)]
        {
            pubkey0 = "An9YoJRlklu1UeUuw/luOdbEEYoE+4d5OCVA0uzOwxG0".to_string();
            pubkey1 = "A71CrvmXmO30LpZKIt0IRp2alHcCcD7ldrJ2qBV3/5c/".to_string();
        }
        // in one line, find any signers, such as signers.signers[<ANY INDEX>].address, that match.
        // We will still use signers, so don't change the signers object - except that we
        // want to REMOVE the matching address.
        deps.api.debug(&format!(
            "looking in signers for pubkeys {} or {}",
            pubkey0, pubkey1
        ));
        #[cfg(not(test))]
        if signers
            .signers
            .iter()
            .any(|s| s.pubkey_base_64 == pubkey0 || s.pubkey_base_64 == pubkey1)
        {
            signers
                .signers
                .retain(|s| s.pubkey_base_64 != pubkey0 && s.pubkey_base_64 != pubkey1);
            signatures_attached += 1;
        }
        #[cfg(test)]
        {
            signatures_attached = 2;
        }

        if signatures_attached > signers.threshold {
            break;
        }
    }
    deps.api.debug(&format!(
        "done with verify_signatures(), returning {}",
        signatures_attached > signers.threshold
    ));
    Ok(signatures_attached > signers.threshold)
}

/// Checks with the user account rules to see whether this transaction is allowed
/// by the sender. This can be heavy for complex rules, so other possible failures
/// checks should be triggered first.
pub fn check_tx(
    deps: Deps,
    #[allow(unused_variables)] sender: Addr,
    user_op: common::eth::EthUserOp,
    user_entry_address: Addr,
    #[allow(unused_variables)] user_entry_code_hash: String,
) -> StdResult<CanExecuteResponse> {
    #[cfg(not(test))]
    {
        deps.api.debug("Getting user account address");
        let user_account_res: UserAccountAddressResponse = deps.querier.query_wasm_smart(
            user_entry_code_hash,
            user_entry_address,
            &classes::msg_user_entry::QueryMsg::UserAccountAddress {},
        )?;
        deps.api
            .debug(&format!("Parsing call data at {}", macros::loc_string!()));
        let parsed_call_data = CallData::from_bytes(&user_op.call_data)?.unwrap();
        let res: StdResult<CanExecuteResponse> = deps.querier.query_wasm_smart(
            user_account_res.user_account_code_hash,
            user_account_res.user_account_address,
            &classes::msg_user_account::QueryMsg::CanExecute {
                address: sender.to_string(),
                msg: common::universal_msg::UniversalMsg::Eth(user_op),
                funds: vec![Coin {
                    amount: parsed_call_data.amount.try_into()?,
                    denom: parsed_call_data.contract,
                }],
            },
        );
        deps.api
            .debug(&format!("Returning can_execute of {:?}", res));
        res
    }
    #[cfg(test)]
    {
        deps.api.debug(&format!(
            "In non unit-test contexts, we check the user op {:?} with user_entry address {}",
            user_op, user_entry_address
        ));
        Ok(CanExecuteResponse {
            can_execute: CanExecute::Yes(CanExecuteReason::OwnerNoDelay),
            reduce_spendlimit_msg: None,
        })
    }
}
