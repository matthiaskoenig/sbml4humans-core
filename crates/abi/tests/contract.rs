//! The contract constant and its check.

use sbml4humans_abi::{ContractError, REPORT_CONTRACT, check_contract};

#[test]
fn the_report_contract_is_report_1() {
    assert_eq!(REPORT_CONTRACT, "report/1");
}

#[test]
fn an_exact_match_is_accepted_and_anything_else_names_both_sides() {
    assert!(check_contract(REPORT_CONTRACT, REPORT_CONTRACT).is_ok());
    let err = check_contract("report/2", REPORT_CONTRACT).unwrap_err();
    assert_eq!(
        err,
        ContractError {
            found: "report/2".to_string(),
            expected: "report/1".to_string()
        }
    );
}
