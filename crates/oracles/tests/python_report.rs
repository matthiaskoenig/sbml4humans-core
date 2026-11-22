//! The gate: the Rust report of a document equals the python report, field for field.

use sbml4humans_oracles::compare;

// what this slice does not build yet. Every later slice removes lines from this list,
// and the list is empty when the port is complete.
const EXCLUDED: &[&str] = &[
    "/document/annotationXml",
    "/models/0/listOfReactions",
    "/models/0/listOfPorts",
    "/models/0/fbc",
    "/models/0/kind",
    "/models/0/substanceUnits",
    "/models/0/listOfFunctionDefinitions",
    "/models/0/listOfUnitDefinitions",
    "/models/0/listOfInitialAssignments",
    "/models/0/listOfRules",
    "/models/0/listOfConstraints",
    "/models/0/listOfEvents",
    "/models/0/listOfSubmodels",
    "/models/0/listOfGeneProducts",
    "/models/0/listOfObjectives",
    "/models/0/listOfFluxBounds",
    "/models/0/listOfUserDefinedConstraints",
    "/models/0/listOfQualitativeSpecies",
    "/models/0/listOfTransitions",
    // The annotation of an element is not read into cvterms yet, python reads the bqbiol:is of the
    // parameter k1 into one. Removed by the slice which reads the annotation RDF.
    "/models/0/listOfParameters/0/cvterms",
    "/linkGraph",
    "/odeSystem",
    "/odeError",
];

#[test]
fn the_report_of_minimal_model_equals_the_python_report() {
    let source = std::fs::read("../../testdata/minimal_model.xml").unwrap();
    let ours = serde_json::to_value(sbml4humans_report::build(
        &sbml4humans_libsbml::read(&source).unwrap(),
    ))
    .unwrap();
    let theirs: serde_json::Value = serde_json::from_slice(
        &std::fs::read("../../testdata/oracles/minimal_model.json").unwrap(),
    )
    .unwrap();
    let differences = compare(&ours, &theirs, EXCLUDED);
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

#[test]
fn the_comparison_is_structural_and_not_textual() {
    let a = serde_json::json!({"b": 1, "a": [1.0, 2.0]});
    let b = serde_json::json!({"a": [1.0, 2.0], "b": 1});
    assert!(compare(&a, &b, &[]).is_empty());
    let c = serde_json::json!({"a": [2.0, 1.0], "b": 1});
    assert_eq!(compare(&a, &c, &[]).len(), 2);
    let d = serde_json::json!({"a": [1.0, 2.0], "b": 2});
    assert_eq!(compare(&a, &d, &["/b"]).len(), 0);
}
