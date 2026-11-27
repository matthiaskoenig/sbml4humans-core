//! The display policy of unit factors is independent of the SBML reader.

#[test]
fn unit_factors_match_the_python_registry() {
    let data: serde_json::Value =
        serde_json::from_slice(include_bytes!("../../../testdata/oracles/units.json")).unwrap();
    let mut differences = Vec::new();
    for case in data["cases"].as_array().unwrap() {
        let actual = sbml4humans_report::units::factor(
            case["kind"].as_str().unwrap(),
            case["exponent"].as_f64().unwrap(),
            case["scale"].as_i64().unwrap() as i32,
            case["multiplier"].as_f64().unwrap(),
        )
        .unwrap();
        let expected = case["rendered"].as_str();
        if actual.as_deref() != expected {
            differences.push(format!(
                "{}^{} * {}e{}: ours {actual:?}, Python {expected:?}",
                case["kind"], case["exponent"], case["multiplier"], case["scale"]
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "{} differences:\n{}",
        differences.len(),
        differences
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn mathematical_unit_inference_matches_libsbml_in_global_and_local_scopes() {
    let data: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../testdata/oracles/unit_inference.json"
    ))
    .unwrap();
    let mut differences = Vec::new();
    for case in data["cases"].as_array().unwrap() {
        let input = sbml4humans_libsbml::read(case["xml"].as_str().unwrap().as_bytes()).unwrap();
        let report = sbml4humans_report::build(&input).unwrap();
        let model = &report.models[0];
        let actual = if case["context"] == "rule" {
            model.list_of_rules[0].derived_units.as_deref()
        } else {
            model.list_of_reactions[0]
                .kinetic_law
                .as_ref()
                .unwrap()
                .derived_units
                .as_deref()
        };
        if actual != case["rendered"].as_str() {
            differences.push(format!(
                "L{}V{} {} {}: ours {actual:?}, libSBML {}",
                case["level"], case["version"], case["context"], case["formula"], case["rendered"]
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "{} differences:\n{}",
        differences.len(),
        differences
            .iter()
            .take(16)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
