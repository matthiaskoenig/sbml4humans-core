//! The equation report, including failures, compared to Python.
use sbml4humans_oracles::compare;
#[test]
fn equation_reports_match_python() {
    let mut failures = Vec::new();
    for name in [
        "minimal_model",
        "level2_model",
        "level2_reactions",
        "level2_graph_units",
        "core_elements",
        "core_graph_references",
        "sbase_annotations",
        "distrib_spans",
        "qual_example",
        "fbc_example",
        "fbc_constraints_v3",
        "fbc_bounds_v1",
        "fbc_mass_charge",
        "comp_deletion",
        "comp_internal",
        "comp_replacement",
        "comp_replaced_by",
        "comp_deleted",
        "comp_factors",
    ] {
        let source = std::fs::read(format!("../../testdata/{name}.xml")).unwrap();
        let input = sbml4humans_reader::read(&source).unwrap();
        let ours = serde_json::to_value(sbml4humans_report::build(&input).unwrap()).unwrap();
        let theirs = serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(format!("../../testdata/oracles/{name}.json")).unwrap(),
        )
        .unwrap();
        let ours = serde_json::json!({"system":ours["odeSystem"],"error":ours["odeError"]});
        let theirs = serde_json::json!({"system":theirs["odeSystem"],"error":theirs["odeError"]});
        let differences = compare(&ours, &theirs, &[]);
        eprintln!("{name}: {} differences", differences.len());
        failures.extend(
            differences
                .into_iter()
                .take(8)
                .map(|d| format!("{name}: {d}")),
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
