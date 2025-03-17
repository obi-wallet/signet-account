use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ContractError {
    #[error("This address is already authorized as a Permissioned Address. Remove it first in order to update it.")]
    PermissionedAddressExists {},

    #[error("This address is not authorized as a spend limit Permissioned Address.")]
    PermissionedAddressDoesNotExist {},

    #[error("This address is not authorized as a Beneficiary.")]
    BeneficiaryDoesNotExist {},

    #[error("This permissioned address does not exist or is trying to exceed its spend limit.")]
    PermissionedAddressDoesNotExistOrOverLimit {},

    #[error("You cannot spend more than your available spend limit. Trying to spend {0} {1}")]
    CannotSpendMoreThanLimit(String, String),

    #[error("Failed to advance the reset day: {0}")]
    DayUpdateError(String),

    #[error("Failed to advance the reset month")]
    MonthUpdateError {},

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

// Migration version management not used currently
// impl From<semver::Error> for ContractError {
//     fn from(err: semver::Error) -> Self {
//         Self::SemVer(err.to_string())
//     }
// }
