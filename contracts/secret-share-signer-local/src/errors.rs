use secret_cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum SecretShareSignerError {
    #[error("{0}")]
    // let thiserror implement From<StdError> for you
    Std(#[from] StdError),
    #[error("No CompletedOfflineStage share was set for {0} with participants {1:?}")]
    NoCompletedOfflineStageShareSetForUserEntry(String, Vec<u8>),
    #[error("UserOp spends ERC20 but does not include required fee of {0} - use multisend with correct fee")]
    FeeRequiredInUserOp(String),
    #[error("UserOp pays fee but to wrong address: {0}")]
    BadFeePayAddress(String),
    #[error("Only the owner of the user_entry contract included can SetShares")]
    Unauthorized {},
}
