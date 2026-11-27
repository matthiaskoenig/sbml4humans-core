//! The wire contract of the report of sbml4humans.

/// The contract the report is written in and read with.
pub const REPORT_CONTRACT: &str = "report/1";

/// The contract of a report differs from the one this build speaks.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the contract is {found}, this build speaks {expected}")]
pub struct ContractError {
    /// The contract found in the report.
    pub found: String,
    /// The contract this build speaks.
    pub expected: String,
}

/// Accepts `found` only when it is exactly `expected`.
///
/// # Errors
///
/// Returns a [`ContractError`] naming both sides when they differ.
pub fn check_contract(found: &str, expected: &str) -> Result<(), ContractError> {
    if found == expected {
        Ok(())
    } else {
        Err(ContractError {
            found: found.to_string(),
            expected: expected.to_string(),
        })
    }
}

/// The complete source envelope and manifest contract.
pub const SOURCE_CONTRACT: &str = "source/1";
