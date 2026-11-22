//! The JSON the types of the report produce.

use sbml4humans_report::{Double, ReportCompartment, SBase};

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
        sbase: SBase {
            pk: "m/Compartment:cell".into(),
            sbml_type: "Compartment",
            id: Some("cell".into()),
            ..SBase::default()
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
