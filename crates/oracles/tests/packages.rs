//! Package projections are compared to the complete Python wire contract.
use sbml4humans_oracles::compare;

#[test]
fn pure_rust_package_reports_match_python() {
    let mut failures = Vec::new();
    for name in [
        "minimal_model",
        "comp_deletion",
        "distrib_spans",
        "qual_example",
        "fbc_example",
        "fbc_constraints_v3",
        "fbc_mass_charge",
        "fbc_bounds_v1",
    ] {
        let source = std::fs::read(format!("../../testdata/{name}.xml")).unwrap();
        let document = sbml4humans_reader::read(&source).unwrap();
        let report = sbml4humans_report::build(&document).unwrap();
        let ours = serde_json::to_value(report).unwrap();
        let theirs = serde_json::from_slice(
            &std::fs::read(format!("../../testdata/oracles/{name}.json")).unwrap(),
        )
        .unwrap();
        let differences = compare(&ours, &theirs, &["/odeSystem", "/odeError"]);
        eprintln!("{name}: {} differences", differences.len());
        failures.extend(
            differences
                .into_iter()
                .take(12)
                .map(|d| format!("{name}: {d}")),
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
