use schemars::JsonSchema;

use serde::{Deserialize, Serialize};

use ec_curve::secp256k1::point::Secp256k1Point;
use scrt_sss::Secp256k1Scalar;

use common::eth::EthUserOp;

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct InstantiateMsg {
    pub fee_manager_address: String,
    pub fee_manager_code_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Accept and store a list of CompletedOfflineStageParts and their associated signing
    /// participant ids for the sender address. This can also be used to delete previously set
    /// CompletedOfflineStageParts for a given set of participants if None is passed in for
    /// completed_offline_stage
    SetShares {
        participants_to_completed_offline_stages: Vec<ParticipantsToCompletedOfflineStageParts>,
        // Transactions are checked with this point (whether they match abstraction rules).
        // Only the owner of the associated account can `SetShares`.
        user_entry_address: String,
    },
    Sign {
        participants: Vec<u8>,
        user_entry_address: String,
        user_entry_code_hash: String,
        entry_point: String,
        chain_id: String,
        user_operation: Box<EthUserOp>,
        other_partial_sigs: Vec<PartialSignature>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PartialSignature(pub Secp256k1Scalar);

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Success,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    UserOpFeesValid {
        user_op: EthUserOp,
        chain_id: String,
        user_entry_address: String,
    },
    UserOpTxValid {
        user_op: EthUserOp,
        chain_id: String,
        user_entry_address: String,
        user_entry_code_hash: String,
        sender: String,
    },
    PassportPubkey {
        user_entry_address: String,
    },
    SignUserop {
        participants: Vec<u8>,
        user_entry_address: String,
        user_entry_code_hash: String,
        entry_point: String,
        chain_id: String,
        user_operation: EthUserOp,
        other_partial_sigs: Vec<PartialSignature>,
        userop_signed_by_signers: Vec<String>, // as hex
    },
    SignBytes {
        participants: Vec<u8>,
        user_entry_address: String,
        user_entry_code_hash: String,
        other_partial_sigs: Vec<PartialSignature>,
        bytes: String,                        // as hex
        bytes_signed_by_signers: Vec<String>, // as hex
        prepend: bool,
        is_already_hashed: Option<bool>,
    },
}

/// CompletedOfflineStageParts is from rounds 1-6 of the signing process and is computed offline
/// it is unique for each combination of signers that are participating in threshold signing process
/// Meaning if you have 5 shares and a threshold of three, CompletedOfflineStageSigningParts
/// will be different for each combination of participants. For example, if participants [1,2,3,4]
/// are signing vs [1,2,3,5] the set of 4 CompletedOfflineStageSigningParts that are generated from
/// for each combination will be unique.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(non_snake_case)]
pub struct CompletedOfflineStageParts {
    pub k_i: Secp256k1Scalar,
    pub R: Secp256k1Point,
    pub sigma_i: Secp256k1Scalar,
    pub pubkey: Secp256k1Point,
    pub user_entry_code_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ParticipantsToCompletedOfflineStageParts {
    /// the participants that are signing numbered [1, total_share_count]
    pub participants: Vec<u8>,
    /// the completed offline stage parts computed for this contract for this set of participants
    pub completed_offline_stage: Option<CompletedOfflineStageParts>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct UserOpFeesValidResponse {
    pub valid: bool,
    pub comment: String,
}

// due to some unknown cargo issue, we cannot import these
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeeManagerQueryMsg {
    FeeDetails { chain_id: String },
}

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
    serde(rename_all = "snake_case")
)]
pub struct FeeDetailsResponse {
    pub fee_divisor: u64,
    pub fee_pay_address: String,
}

// due to some unknown cargo issue, we cannot import these
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserAccountQueryMsg {
    LegacyOwner {},
}

#[cfg(all(feature = "secretwasm", not(feature = "cosmwasm")))]
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq, schemars::JsonSchema,
)]
#[cfg_attr(
    all(feature = "secretwasm", not(feature = "cosmwasm")),
    serde(rename_all = "snake_case")
)]
pub struct LegacyOwnerResponse {
    pub legacy_owner: String,
}
