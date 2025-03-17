macros::cosmwasm_imports!(ConversionOverflowError, OverflowError, StdError);

use std::num::TryFromIntError;
use std::str::Utf8Error;

use crate::authorization::Authorization;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    ConversionOverflow(#[from] ConversionOverflowError),

    #[error("{0}")]
    TryFromInt(#[from] TryFromIntError),

    #[error("{0}")]
    Utf8(#[from] Utf8Error),

    #[error(transparent)]
    JsonError(#[from] serde_json_wasm::de::Error),

    /// Errors related to sudo (no-auth) account factory
    #[error("{0}")]
    AccountCreator(#[from] AccountCreatorError),

    /// Errors related to authorization or permissions
    #[error("{0}")]
    Auth(#[from] AuthorizationError),

    /// Errors related to Ethereum user op checking
    #[error("{0}")]
    Eth(#[from] EthError),

    /// Errors related to message types or deserialization
    #[error("{0}")]
    Factory(#[from] FactoryError),

    /// Errors which should never happen (logic problem)
    #[error("{0}")]
    Flow(#[from] FlowError),

    /// Errors related to migration
    #[error("{0}")]
    Migrate(#[from] MigrateError),

    /// Errors related to message types or deserialization
    #[error("{0}")]
    Msg(#[from] MessageError),

    /// Errors related to Permissioned Addresses
    #[error("{0}")]
    PermAddy(#[from] PermissionedAddressError),

    /// Errors related to the resetting of spend limits
    #[error("{0}")]
    Reset(#[from] ResetError),

    /// Errors specific to session keys
    #[error("{0}")]
    Sessionkey(#[from] SessionkeyError),

    /// Errors related to checking or spending within limits
    #[error("{0}")]
    SpendLimit(#[from] SpendLimitError),

    /// Errors related to simulated swapping
    #[error("{0}")]
    Swap(#[from] SwapError),

    /// Errors related to updating owner
    #[error("{0}")]
    Update(#[from] UpdateError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    /// Errors happening in reply
    #[error("{0}")]
    Reply(#[from] ReplyError),

    /// Errors related to unifier contract
    #[error("{0}")]
    Unifier(#[from] UnifierError),

    /// Errors related to debt
    #[error("{0}")]
    Debt(#[from] DebtError),

    /// Errors related to account
    #[error("{0}")]
    Account(#[from] AccountError),

    /// Errors related to gatekeeper
    #[error("{0}")]
    Gatekeeper(#[from] GatekeeperError),

    #[error("not implemented yet")]
    NotImplemented {},
}

impl From<ContractError> for StdError {
    fn from(value: ContractError) -> Self {
        StdError::generic_err(value.to_string())
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum AccountCreatorError {
    #[error("Cannot have multiple messages when calling sudo account creation")]
    InvalidMsgsLength {},
    #[error("Sudo only available for NewAccount MsgExecuteContract message")]
    InvalidMsg {},
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum AuthorizationError {
    #[error("Caller is not owner. Location: {0}")]
    Unauthorized(String),
    #[error("Expected owner {0}, got {1:?}. Location: {2}")]
    UnauthorizedInfo(String, Option<String>, String),
    #[error(
        "This address is not permitted to spend this token, or to spend this many of this token."
    )]
    SpendNotAuthorized {},
    #[error("User state contract address not set in user account contract")]
    UserStateContractNotSet {},
    #[error("User state code hash not set in user account contract")]
    UserStateCodeHashNotSet {},
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum EthError {
    #[error("Eth user op contains no calldata")]
    NoCallData {},
    #[error("No eth interpreter provided")]
    NoEthInterpreter {},
    #[error("No eth interpreter code hash provided")]
    NoEthInterpreterCodeHash {},
    #[error("Eth message checking currently only supports contract allowlists")]
    NotAnEthContractAllowlist {},
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum FactoryError {
    #[error("User account address not set")]
    UserAccountAddressNotSet {},
    #[error("gatekeeper address not set")]
    GatekeeperAddressesNotSet {},
    #[error("Sessionkey gatekeeper contract address not set")]
    SessionGatekeeperAddressesNotSet {},
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum FlowError {
    #[error("Mismatched rule type and rule params at location {0}")]
    MismatchedRuleTypes(String),
}

#[derive(Error, Debug, PartialEq)]
pub enum MessageError {
    #[error("Spend-limited cw20 transactions cannot have additional funds attached.")]
    AttachedFundsNotAllowed {},
    #[error("Spend-limited WasmMsg txes must be cw20 Send or Transfer messages.")]
    OnlyTransferSendAllowed {},
    #[error("Message deserialization error.  Spend-limited WasmMsg txes are limited to a Cw20ExecuteMsg Send or Transfer.")]
    ErrorDeserializingCw20Message {},
    #[error("WASM message is not Execute. Spend-limited WasmMsg txes are limited to a Cw20ExecuteMsg Send or Transfer.")]
    WasmMsgMustBeExecute {},
    #[error("Spend-limited transactions are not allowed to be {0}; they must be BankMsg or WasmMsg (Cw20ExecuteMsg Send or Transfer)."    )]
    BadMessageType(String),
    #[error("Uninitialized message.")]
    UninitializedMessage {},
    #[error("No authorization for target contract, found in {loc:?}")]
    NoSuchAuthorization { loc: String },
    #[error("Too many submessages in JSON (incorrect batching)")]
    TooManyMessages {},
    #[error("Field mismatch: field {key:?} must contain parameter {value:?}")]
    FieldMismatch { key: String, value: String },
    #[error("Missing required field: field {key:?} must contain parameter {value:?}")]
    MissingRequiredField { key: String, value: String },
    #[error("Multiple matching authorizations. Please be more specific or use rm_all_matching_authorizations. Found: {vector:?}")]
    MultipleMatchingAuthorizations { vector: Vec<(u64, Authorization)> },
    #[error("No execute message contents")]
    NoExecuteMessage {},
    #[error("Eth messages cannot be executed directly. Use query can_execute")]
    ErrorExecuteEthMessages {},
    #[error("Cannot parse secret message without feature 'secretwasm'")]
    ErrorExecuteOsmosisMessages {},
    #[error("Cannot execute Osmosis messages on other chains. Use the signer.")]
    ErrorParseSecretMessage {},
    #[error("Custom CosmosMsg not yet supported")]
    InvalidCustomCosmosMsg {},
    #[error("This CosmosMsg type not yet supported")]
    InvalidCosmosMsg {},
    #[error("This OsmosisMsg type not yet supported")]
    InvalidOsmosisMsg {},
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MigrateError {
    #[error("A specified gatekeeper code id does not have a matching gatekeeper attached to this account.")]
    GatekeeperNotFound {},
    #[error("Sender must be the user account contract in order to assist migration")]
    UserAccountMismatch {},
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum PermissionedAddressError {
    #[error("This permissioned address does not exist or is trying to exceed its spend limit.")]
    PermissionedAddressDoesNotExistOrOverLimit {},
    #[error("This address is already authorized as a Permissioned Address. Remove it first in order to update it.")]
    PermissionedAddressExists {},
    #[error("This address is not authorized as a spend limit Permissioned Address.")]
    PermissionedAddressDoesNotExist {},
    #[error("This address is not authorized as a Beneficiary.")]
    BeneficiaryDoesNotExist {},
    #[error("The dormancy period to activate this beneficiary has not yet passed.")]
    BeneficiaryCooldownNotExpired {},
    #[error("{0}")]
    BeneficiaryResetError(String),
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ResetError {
    #[error("Failed to advance the reset day: {0}")]
    DayUpdateError(String),
    #[error("Failed to advance the reset month")]
    MonthUpdateError {},
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum SessionkeyError {
    #[error("Session key is expired")]
    Expired {},
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum SpendLimitError {
    #[error("Cannot send 0 funds")]
    CannotSpendZero {},
    #[error("Spendlimit rule expired")]
    Expired {},
    #[error("Unable to pay debt of {0} uusd")]
    UnableToRepayDebt(String),
    #[error("You cannot spend more than your available spend limit. Trying to spend {0} {1}")]
    CannotSpendMoreThanLimit(String, String),
    #[error("Unable to get current asset price to check spend limit for asset. If this transaction is urgent, use your multisig to sign. SUBMSG: {0} CONTRACT: {1} ERROR: {2}")]
    PriceCheckFailed(String, String, String),
    #[error("{0}")]
    CompoundError(String),
    #[error("multiple matching rules; use manual rule management")]
    MultipleMatchingRules {},
    #[error("{0}, failed to get balance")]
    GetBalanceError(String),
    #[error("Spendlimit can only handle beneficiary/inheritance rules")]
    InvalidRuleForSpendLimit {},
    #[error("{0}")]
    QueryPermissionedAddressesError(String),
    #[error("{0}")]
    QueryCanSendError(String),
    #[error("{0}")]
    QueryLegacyOwnerError(String),
}

#[derive(Error, Debug, PartialEq)]
pub enum SwapError {
    #[error("Mismatched pair contract.")]
    MismatchedPairContract {},
    #[error("{0}")]
    BadSwapDenoms(String),
    #[error("Pair contract for asset {} to {} not found", 0, 1)]
    PairContractNotFound(String, String),
    #[error("{0}")]
    QuerySwapRouteError(String),
    // #[error("Mismatched pair contract.")]
    // MismatchedPairContract {},
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum UpdateError {
    #[error("Contract cannot be migrated while an owner update is pending")]
    CannotMigrateUpdatePending {},
    #[error("Contract update delay cannot be changed while an owner update is pending")]
    CannotUpdateUpdatePending {},
    #[error("Caller is not pending new owner. Propose new owner first.")]
    CallerIsNotPendingNewOwner {},
    #[error("Next hash was not updated as expected.")]
    NextHashNotUpdated {},
}

#[derive(Error, Debug, PartialEq)]
pub enum ReplyError {
    #[error(
        "Error {:?} updating Code Admin! Trying to update to {} for address {:?}",
        0,
        1,
        2
    )]
    CannotUpdateCodeAdmin(String, String, Option<String>),
    #[error("Cannot parse reply msg! Error {0}")]
    CannotParseReplyMsg(String),
    #[error("Invalid instantiate reply id")]
    InvalidInstantiateReplyId {},
    #[error("unable to save {0} gatekeeper address")]
    FailedToHandleReply(String),
}

#[derive(Error, Debug, PartialEq)]
pub enum UnifierError {
    #[error("{0}, failed to query asset unifier for default unified asset")]
    FailedQueryAssetUnifier(String),
    #[error("Eth price conversion not supported yet")]
    EthPriceConversionError {},
    #[error("Asset {0} not found in OsmoPool {1}")]
    AssetNotFoundInOsmoPool(String, String),
}

#[derive(Error, Debug, PartialEq)]
pub enum DebtError {
    #[error("debt cannot be negative")]
    NegativeDebt {},
    #[error("adjustment overflowed")]
    AdjustmentOverflow {},
    #[error("denom mismatch: {} != {}", 0, 1)]
    DenomMismatch(String, String),
    #[error("only owner can add debt")]
    AddDebtUnauthorized {},
    #[error("only user account contract can clear debt")]
    ClearDebtUnauthorized {},
    #[error("Account has outstanding debt")]
    OutstandingDebt {},
    #[error("No known debt gatekeeper address")]
    UnkownDebtGatekeeperAddress {},
    #[error("debt code hash unavailable")]
    InvalidDebtCodeHash {},
}

#[derive(Error, Debug, PartialEq)]
pub enum AccountError {
    #[error("Unable to save account")]
    CannotSaveAccount {},
    #[error("Unable to load account")]
    CannotLoadAccount {},
    #[error("User state contract address not set: {0}")]
    UserStateContractAddressNotSet(String),
    #[error("The only global rule type allowed currently is Blocklist")]
    InvalidGlobalRule {},
    #[error("Unexpected error: can_execute is Yes with reason {0} but message or spend is not marked ok")]
    UnexpectedCanExecuteError(String),
    #[error("User state not attached")]
    UserStateNotAttached {},
    #[error("User state code hash not set: {0}")]
    UserStateCodeHashNotSet(String),
    #[error("Signers cannot be empty")]
    SignersEmpty {},
    #[error("Threshold must be less than number of signers")]
    ThresholdTooHigh {},
    #[error("Threshold cannot require all signers if >2 signers")]
    ThresholdAllSigners {},
}

#[derive(Error, Debug, PartialEq)]
pub enum GatekeeperError {
    #[error("{0}, {1}, contract {2}, message {3}")]
    QueryGatekeeperError(String, String, String, String),
}

// Migration doesn't currently use semver
// impl From<semver::Error> for ContractError {
//     fn from(err: semver::Error) -> Self {
//         Self::SemVer(err.to_string())
//     }
// }
