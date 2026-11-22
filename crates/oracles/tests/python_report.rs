//! The gate: the Rust report of a document equals the python report, field for field.

use sbml4humans_oracles::compare;

// what this slice does not build yet. Every later slice removes lines from this list,
// and the list is empty when the port is complete.
const EXCLUDED: &[&str] = &[
    "/models/0/listOfReactions",
    "/models/0/listOfPorts",
    "/models/0/fbc",
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
    // parameter k1 into one. The container itself stays compared, so a second term would differ.
    // Removed by the slice which reads the annotation RDF.
    "/models/0/listOfParameters/0/cvterms/0",
    // The one field the builder knowingly fabricates: it writes the `-` python writes for an
    // element whose unit does not derive, for every compartment, species and parameter. Python
    // derives the unit of an element, `l` for a compartment with a size, `\frac{mol}{l}` for a
    // species with an amount, so the value is wrong for a document which sets units at all.
    // Removed by the slice which derives units.
    "**/derivedUnits",
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
    // anti-vacuity: the walk reaches the fixture, so an empty report is not equal to it
    assert!(!compare(&ours, &serde_json::json!({}), EXCLUDED).is_empty());
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

#[test]
fn a_key_present_on_one_side_only_is_a_difference_in_both_directions() {
    let with = serde_json::json!({"a": 1, "b": 2});
    let without = serde_json::json!({"a": 1});
    assert_eq!(compare(&with, &without, &[]).len(), 1);
    assert_eq!(compare(&without, &with, &[]).len(), 1);
    assert_eq!(compare(&with, &without, &["/b"]).len(), 0);
    assert_eq!(compare(&without, &with, &["/b"]).len(), 0);
}

#[test]
fn arrays_of_a_different_length_differ_once_per_surplus_item() {
    let short = serde_json::json!([1, 2]);
    let long = serde_json::json!([1, 2, 3, 4]);
    assert_eq!(compare(&short, &long, &[]).len(), 2);
    assert_eq!(compare(&long, &short, &[]).len(), 2);
    assert_eq!(compare(&long, &short, &["/2"]).len(), 1);
}

#[test]
fn a_type_mismatch_is_a_difference() {
    use serde_json::json;
    assert_eq!(compare(&json!({}), &json!([]), &[]).len(), 1);
    assert_eq!(compare(&json!(0), &json!("0"), &[]).len(), 1);
    assert_eq!(compare(&json!(false), &json!(null), &[]).len(), 1);
}

#[test]
fn an_integer_and_a_float_of_the_same_value_agree() {
    use serde_json::json;
    assert_eq!(compare(&json!({"b": 1}), &json!({"b": 1.0}), &[]).len(), 0);
    assert_eq!(compare(&json!({"b": 1.0}), &json!({"b": 1}), &[]).len(), 0);
    assert_eq!(compare(&json!({"b": 1}), &json!({"b": 1.5}), &[]).len(), 1);
}

#[test]
fn an_exclusion_hides_a_pointer_and_its_descendants_and_not_a_sibling_with_the_same_prefix() {
    use serde_json::json;
    let ours = json!({"kind": {"x": 1}, "kindOf": 1});
    let theirs = json!({"kind": {"x": 2}, "kindOf": 2});
    assert_eq!(compare(&ours, &theirs, &[]).len(), 2);
    assert_eq!(compare(&ours, &theirs, &["/kind"]).len(), 1);
    assert_eq!(compare(&ours, &theirs, &["/kind/x"]).len(), 1);
    assert_eq!(compare(&ours, &theirs, &["/kind", "/kindOf"]).len(), 0);
}

#[test]
fn an_exact_exclusion_carries_the_escaping_of_rfc_6901() {
    use serde_json::json;
    let ours = json!({"a/b": 1, "c~d": 1});
    let theirs = json!({"a/b": 2, "c~d": 2});
    assert_eq!(compare(&ours, &theirs, &["/a~1b", "/c~0d"]).len(), 0);
    assert_eq!(compare(&ours, &theirs, &["/a/b", "/c~d"]).len(), 2);
}

#[test]
fn a_trailing_segment_exclusion_hides_that_segment_wherever_it_sits() {
    use serde_json::json;
    let ours = json!({"a": {"x": 1}, "b": [{"x": {"deep": 1}}], "x": 1});
    let theirs = json!({"a": {"x": 2}, "b": [{"x": {"deep": 2}}], "x": 2});
    assert_eq!(compare(&ours, &theirs, &[]).len(), 3);
    assert_eq!(compare(&ours, &theirs, &["**/x"]).len(), 0);
}

#[test]
fn a_trailing_segment_exclusion_matches_the_whole_segment_and_not_its_end() {
    use serde_json::json;
    let ours = json!({"a": {"yx": 1}, "ax": 1});
    let theirs = json!({"a": {"yx": 2}, "ax": 2});
    assert_eq!(compare(&ours, &theirs, &["**/x"]).len(), 2);
    assert_eq!(compare(&ours, &theirs, &["**/yx", "**/ax"]).len(), 0);
}

#[test]
fn an_empty_exclusion_hides_nothing_rather_than_the_whole_report() {
    use serde_json::json;
    let ours = json!({"a": 1});
    let theirs = json!({"a": 2});
    assert_eq!(compare(&ours, &theirs, &[""]).len(), 1);
    assert_eq!(compare(&ours, &theirs, &["**/"]).len(), 1);
}
