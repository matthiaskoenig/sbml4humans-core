//! The JSON the types of the report produce.

use sbml4humans_report::{Double, Math, ReportCompartment, SBase};

#[test]
fn a_double_which_is_not_finite_serializes_as_its_string() {
    assert_eq!(serde_json::to_string(&Double(1.5)).unwrap(), "1.5");
    assert_eq!(serde_json::to_string(&Double(3.0)).unwrap(), "3.0");
    assert_eq!(
        serde_json::to_string(&Double(f64::INFINITY)).unwrap(),
        "\"Infinity\""
    );
    assert_eq!(
        serde_json::to_string(&Double(f64::NEG_INFINITY)).unwrap(),
        "\"-Infinity\""
    );
    assert_eq!(serde_json::to_string(&Double(f64::NAN)).unwrap(), "\"NaN\"");
}

#[test]
fn an_element_serializes_the_shared_fields_in_camel_case_and_keeps_the_unset_ones_null() {
    let c = ReportCompartment {
        // the type is a field of the constructor and no field of the literal, so no element of a
        // report can be built without one
        sbase: SBase {
            id: Some("cell".into()),
            ..SBase::new("m/Compartment:cell".into(), "Compartment")
        },
        spatial_dimensions: Some(Double(3.0)),
        size: Some(Double(1.0)),
        constant: Some(true),
        units: None,
        units_latex: None,
        derived_units: Some("-".into()),
    };
    let v: serde_json::Value = serde_json::to_value(&c).unwrap();
    assert_eq!(v["pk"], "m/Compartment:cell");
    assert_eq!(v["sbmlType"], "Compartment");
    assert_eq!(v["spatialDimensions"], 3.0);
    assert_eq!(v["derivedUnits"], "-");
    assert!(v["metaId"].is_null() && v["units"].is_null() && v["unitsLatex"].is_null());
    assert_eq!(v["cvterms"], serde_json::json!([]));
    assert_eq!(v["lists"], serde_json::json!([]));
}

// `math_info` of the python implementation writes `{"latex": null, "formula": ...}` for a math it
// cannot typeset, so the key is there and holds `null`; the frontend reads the field and shows the
// formula alone where it is absent, and a math which lost the key would read as a math of no latex
// field at all.
#[test]
fn a_math_without_a_latex_serializes_the_key_as_null() {
    let without = Math {
        latex: None,
        formula: "1 + lambda()".into(),
    };
    assert_eq!(
        serde_json::to_string(&without).unwrap(),
        r#"{"latex":null,"formula":"1 + lambda()"}"#
    );
    let with = Math {
        latex: Some(" \\mathit{a}".into()),
        formula: "a".into(),
    };
    assert_eq!(
        serde_json::to_string(&with).unwrap(),
        r#"{"latex":" \\mathit{a}","formula":"a"}"#
    );
}
