//! The report builder maps the document model onto the report.

use sbml4humans_libsbml::read;
use sbml4humans_model::{Compartment, Document, Element, Model};
use sbml4humans_report::{build, pk};

/// A Level 3 Version 2 document with one element of every type of the core.
///
/// It is the document of the oracle gate, read from the workspace root so that the document these
/// tests assert the report of is the document the gate compares against the python report.
const CORE_ELEMENTS: &[u8] = include_bytes!("../../../testdata/core_elements.xml");

/// A document whose every element of a type which carries a math carries none.
const NO_MATHS: &[u8] = include_bytes!("../testdata/no_maths.xml");

/// The report of a document, as the JSON the frontend reads.
fn report_of(source: &[u8]) -> serde_json::Value {
    serde_json::to_value(build(&read(source).unwrap()).unwrap()).unwrap()
}

fn doc_with(model: Model) -> Document {
    Document {
        level: 3,
        version: 1,
        packages: vec![],
        element: Element::default(),
        model: Some(model),
    }
}

#[test]
fn the_pk_is_scope_type_and_key() {
    assert_eq!(
        pk::of("document", "SBMLDocument", "document"),
        "document/SBMLDocument:document"
    );
    assert_eq!(
        pk::of("minimal_model", "Compartment", "cell"),
        "minimal_model/Compartment:cell"
    );
}

#[test]
fn the_document_is_keyed_in_the_document_scope_and_an_element_in_the_scope_of_its_model() {
    let model = Model {
        element: Element {
            id: Some("minimal_model".into()),
            ..Element::default()
        },
        compartments: vec![Compartment {
            element: Element {
                id: Some("cell".into()),
                ..Element::default()
            },
            ..Compartment::default()
        }],
        ..Model::default()
    };
    let report = build(&doc_with(model)).unwrap();
    assert_eq!(report.document.sbase.pk, "document/SBMLDocument:document");
    assert_eq!(
        report.models[0].sbase.pk,
        "minimal_model/Model:minimal_model"
    );
    assert_eq!(
        report.models[0].list_of_compartments[0].sbase.pk,
        "minimal_model/Compartment:cell"
    );
}

#[test]
fn an_element_without_an_id_is_keyed_by_its_meta_id_and_siblings_without_either_do_not_collide() {
    let model = Model {
        element: Element {
            id: Some("m".into()),
            ..Element::default()
        },
        compartments: vec![
            Compartment {
                element: Element {
                    meta_id: Some("meta_a".into()),
                    ..Element::default()
                },
                ..Compartment::default()
            },
            Compartment {
                element: Element::default(),
                ..Compartment::default()
            },
            Compartment {
                element: Element::default(),
                ..Compartment::default()
            },
        ],
        ..Model::default()
    };
    let report = build(&doc_with(model)).unwrap();
    let pks: Vec<_> = report.models[0]
        .list_of_compartments
        .iter()
        .map(|c| c.sbase.pk.clone())
        .collect();
    assert_eq!(pks[0], "m/Compartment:meta_a");
    assert_ne!(pks[1], pks[2]);
    assert!(pks[1].starts_with("m/Compartment:"));
}

#[test]
fn a_model_without_an_id_or_a_meta_id_is_keyed_model_in_the_scope_model() {
    let model = Model {
        compartments: vec![Compartment {
            element: Element {
                id: Some("cell".to_owned()),
                ..Element::default()
            },
            ..Compartment::default()
        }],
        ..Model::default()
    };
    let report = build(&doc_with(model)).unwrap();
    assert_eq!(report.models[0].sbase.pk, "model/Model:model");
    assert_eq!(
        report.models[0].list_of_compartments[0].sbase.pk,
        "model/Compartment:cell"
    );
}

#[test]
fn the_report_carries_the_elements_which_hold_a_math() {
    let json = report_of(CORE_ELEMENTS);
    let model = &json["models"][0];
    assert_eq!(
        model["listOfFunctionDefinitions"][0]["pk"],
        "core_elements/FunctionDefinition:f"
    );
    assert_eq!(
        model["listOfFunctionDefinitions"][0]["sbmlType"],
        "FunctionDefinition"
    );
    assert!(model["listOfFunctionDefinitions"][0]["math"]["formula"].is_string());
    assert_eq!(
        model["listOfUnitDefinitions"][0]["listOfUnits"][0]["kind"],
        "mole"
    );
    assert_eq!(
        model["listOfUnitDefinitions"][0]["listOfUnits"][0]["scale"],
        -3
    );
    assert_eq!(
        model["listOfInitialAssignments"][0]["pk"],
        "core_elements/InitialAssignment:S2"
    );
    assert_eq!(model["listOfInitialAssignments"][0]["symbol"], "S2");
    assert_eq!(model["listOfRules"][0]["sbmlType"], "AssignmentRule");
    assert_eq!(model["listOfRules"][0]["variable"], "k2");
    assert_eq!(
        model["listOfRules"][2]["pk"],
        "core_elements/AlgebraicRule:algebraicRule.0"
    );
    // AlgebraicRule declares no variable, so the key is absent and not null
    assert!(model["listOfRules"][2].get("variable").is_none());
    assert_eq!(
        model["listOfConstraints"][1]["pk"],
        "core_elements/Constraint:constraint.1"
    );
    assert_eq!(
        model["listOfConstraints"][1]["message"],
        serde_json::Value::Null
    );
}

#[test]
fn an_element_whose_math_is_unset_is_reported_with_a_null_math() {
    let json = report_of(NO_MATHS);
    assert_eq!(
        json["models"][0]["listOfRules"][0]["math"],
        serde_json::Value::Null
    );
}

#[test]
fn every_element_which_carries_a_math_is_keyed_and_typed_as_python_keys_and_types_it() {
    let json = report_of(CORE_ELEMENTS);
    let model = &json["models"][0];
    let pks = |list: &str| -> Vec<String> {
        model[list]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["pk"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(
        pks("listOfUnitDefinitions"),
        [
            "core_elements/UnitDefinition:mmole_per_litre",
            "core_elements/UnitDefinition:root_metre",
            "core_elements/UnitDefinition:bare_mole",
            "core_elements/UnitDefinition:no_unit",
        ]
    );
    // the id attribute of an assignment wins over the symbol libsbml aliases to its id
    assert_eq!(
        pks("listOfInitialAssignments"),
        [
            "core_elements/InitialAssignment:S2",
            "core_elements/InitialAssignment:ia_S3",
        ]
    );
    // an assignment and a rate rule are keyed by the variable they set, an algebraic rule by its
    // place among the algebraic rules, and a metaId wins over that place
    assert_eq!(
        pks("listOfRules"),
        [
            "core_elements/AssignmentRule:k2",
            "core_elements/RateRule:k3",
            "core_elements/AlgebraicRule:algebraicRule.0",
            "core_elements/AlgebraicRule:meta_algebraic",
        ]
    );
    assert_eq!(
        pks("listOfConstraints"),
        [
            "core_elements/Constraint:constraint.0",
            "core_elements/Constraint:constraint.1",
        ]
    );
    let types: Vec<&str> = model["listOfRules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["sbmlType"].as_str().unwrap())
        .collect();
    assert_eq!(
        types,
        [
            "AssignmentRule",
            "RateRule",
            "AlgebraicRule",
            "AlgebraicRule"
        ]
    );
}

#[test]
fn a_unit_carries_its_four_attributes_and_no_key_of_an_element() {
    let json = report_of(CORE_ELEMENTS);
    let definitions = &json["models"][0]["listOfUnitDefinitions"];
    assert_eq!(
        definitions[0]["listOfUnits"][1],
        serde_json::json!({"kind": "litre", "exponent": -1.0, "scale": 0, "multiplier": 1.0})
    );
    // Level 3 allows a fractional exponent, which the report writes as the double it is
    assert_eq!(
        definitions[1]["listOfUnits"][0],
        serde_json::json!({"kind": "metre", "exponent": 0.5, "scale": -2, "multiplier": 2.5})
    );
    // a unit which states nothing but its kind writes the three others as null
    assert_eq!(
        definitions[2]["listOfUnits"][0],
        serde_json::json!({
            "kind": "mole", "exponent": null, "scale": null, "multiplier": null
        })
    );
    assert_eq!(definitions[3]["listOfUnits"], serde_json::json!([]));
}

#[test]
fn only_the_types_whose_python_class_declares_a_field_write_that_field() {
    let json = report_of(CORE_ELEMENTS);
    let model = &json["models"][0];
    // python derives the units of an assignment and of a rule and of neither of the three others
    for list in ["listOfInitialAssignments", "listOfRules"] {
        for element in model[list].as_array().unwrap() {
            assert!(element.get("derivedUnits").is_some(), "{list}");
        }
    }
    for list in [
        "listOfFunctionDefinitions",
        "listOfUnitDefinitions",
        "listOfConstraints",
    ] {
        for element in model[list].as_array().unwrap() {
            assert!(element.get("derivedUnits").is_none(), "{list}");
            assert!(element.get("variable").is_none(), "{list}");
        }
    }
    // the unit definition is the one of the five which carries a latex of its units
    assert!(
        model["listOfUnitDefinitions"][0]
            .get("unitsLatex")
            .is_some()
    );
    let message = model["listOfConstraints"][0]["message"].as_str().unwrap();
    assert!(message.starts_with("<message>"), "{message}");
    assert!(message.contains("S1 has to stay positive"), "{message}");
}

#[test]
fn the_report_carries_the_reactions_and_the_events() {
    let json = report_of(CORE_ELEMENTS);
    let reaction = &json["models"][0]["listOfReactions"][0];
    assert_eq!(reaction["pk"], "core_elements/Reaction:J0");
    // the first reactant carries an id of its own and is keyed by it and takes no part in the
    // numbering, so the second, which is about the same species, is keyed without a suffix
    assert_eq!(
        reaction["listOfReactants"][0]["pk"],
        "core_elements/SpeciesReference:sr_S1"
    );
    assert_eq!(
        reaction["listOfReactants"][1]["pk"],
        "core_elements/SpeciesReference:J0.reactant.S1"
    );
    assert_eq!(
        reaction["listOfProducts"][0]["pk"],
        "core_elements/SpeciesReference:J0.product.S2"
    );
    assert_eq!(
        reaction["listOfModifiers"][0]["pk"],
        "core_elements/ModifierSpeciesReference:J0.modifier.E"
    );
    assert_eq!(
        reaction["listOfModifiers"][0]["sbmlType"],
        "ModifierSpeciesReference"
    );
    assert_eq!(
        reaction["kineticLaw"]["pk"],
        "core_elements/KineticLaw:J0.kineticLaw"
    );
    assert!(reaction["kineticLaw"]["math"]["formula"].is_string());
    let parameter = &reaction["kineticLaw"]["listOfLocalParameters"][0];
    assert_eq!(
        parameter["pk"],
        "core_elements/LocalParameter:J0.kineticLaw.Km"
    );
    assert_eq!(parameter["sbmlType"], "LocalParameter");
    // the id of a local parameter is scoped to its kinetic law and keys it never, and the object
    // of the report carries it all the same
    assert_eq!(parameter["id"], "Km");
    assert_eq!(parameter["value"], 0.5);
    assert_eq!(parameter["units"], "mmole_per_litre");
    // the reaction writes `reversible="false"`, so the arrow is the irreversible one
    assert_eq!(reaction["equation"], "2.0 S1 + S1 ➞ S2");
    let event = &json["models"][0]["listOfEvents"][0];
    assert_eq!(event["pk"], "core_elements/Event:E0");
    assert_eq!(event["trigger"]["pk"], "core_elements/Trigger:E0.trigger");
    assert_eq!(event["trigger"]["initialValue"], false);
    assert_eq!(event["trigger"]["persistent"], true);
    assert_eq!(
        event["priority"]["pk"],
        "core_elements/Priority:E0.priority"
    );
    assert_eq!(event["delay"]["pk"], "core_elements/Delay:E0.delay");
    assert_eq!(event["useValuesFromTriggerTime"], true);
    assert_eq!(
        event["listOfEventAssignments"][0]["pk"],
        "core_elements/EventAssignment:E0.S1"
    );
    // libsbml aliases the id of an event assignment to the variable it sets, so the `id` attribute
    // of the document keys the assignment and the variable stays a field of its own
    assert_eq!(
        event["listOfEventAssignments"][1]["pk"],
        "core_elements/EventAssignment:ea_k4"
    );
    assert_eq!(event["listOfEventAssignments"][1]["id"], "ea_k4");
    assert_eq!(event["listOfEventAssignments"][1]["variable"], "k4");
    // nothing of the second reaction is set, Level 3 reports no default
    let bare = &json["models"][0]["listOfReactions"][1];
    assert_eq!(bare["reversible"], serde_json::Value::Null);
    assert_eq!(bare["fast"], serde_json::Value::Null);
    assert_eq!(bare["kineticLaw"], serde_json::Value::Null);
    assert_eq!(
        bare["listOfReactants"][0]["stoichiometry"],
        serde_json::Value::Null
    );
    // the raw stoichiometry of libsbml is a NaN for such a reference, which python names by the id
    // of the reference and by a `?` where it carries none, and the arrow of an unset `reversible`
    // is the reversible one although the report writes no `reversible` at all
    assert_eq!(bare["equation"], "? S3 ⇆ ");
    // the reaction carries no identifier of any kind, so the key is its place in the list, which
    // is the fallback of `pk::key` and not the digest of the xml python falls back to
    assert_eq!(bare["pk"], "core_elements/Reaction:1");
    // the second event carries nothing, not even a trigger, and is keyed by its place
    let bare_event = &json["models"][0]["listOfEvents"][1];
    assert_eq!(bare_event["pk"], "core_elements/Event:event.1");
    assert_eq!(bare_event["trigger"], serde_json::Value::Null);
    assert_eq!(bare_event["priority"], serde_json::Value::Null);
    assert_eq!(bare_event["delay"], serde_json::Value::Null);
    assert_eq!(bare_event["listOfEventAssignments"], serde_json::json!([]));
}

/// A Level 2 Version 4 document whose reaction states none of the attributes its level defaults.
const LEVEL2_REACTION: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level2/version4" level="2" version="4">
  <model id="m">
    <listOfCompartments><compartment id="c"/></listOfCompartments>
    <listOfSpecies>
      <species id="S1" compartment="c"/>
      <species id="S2" compartment="c"/>
    </listOfSpecies>
    <listOfReactions>
      <reaction id="R1">
        <listOfReactants>
          <speciesReference species="S1"/>
          <speciesReference species="S1"/>
        </listOfReactants>
        <listOfProducts>
          <speciesReference species="S2" stoichiometry="2"/>
        </listOfProducts>
        <kineticLaw>
          <math xmlns="http://www.w3.org/1998/Math/MathML"><ci>k</ci></math>
          <listOfParameters><parameter id="k" value="1"/></listOfParameters>
        </kineticLaw>
      </reaction>
    </listOfReactions>
  </model>
</sbml>"#;

#[test]
fn a_reaction_of_a_level_2_document_is_reported_with_the_defaults_of_its_level() {
    let json = report_of(LEVEL2_REACTION);
    let reaction = &json["models"][0]["listOfReactions"][0];
    // Level 1 and Level 2 make a reaction reversible and slow unless it says otherwise, and give a
    // reference which writes neither a number nor a `stoichiometryMath` the stoichiometry 1
    assert_eq!(reaction["reversible"], true);
    assert_eq!(reaction["fast"], false);
    assert_eq!(reaction["listOfReactants"][0]["stoichiometry"], 1.0);
    assert_eq!(reaction["listOfProducts"][0]["stoichiometry"], 2.0);
    // the `constant` of a species reference is a Level 3 attribute
    assert_eq!(
        reaction["listOfReactants"][0]["constant"],
        serde_json::Value::Null
    );
    assert_eq!(reaction["equation"], "S1 + S1 ⇆ 2.0 S2");
    // two references about the same species and neither with an id: the repetition is numbered
    assert_eq!(
        reaction["listOfReactants"][0]["pk"],
        "m/SpeciesReference:R1.reactant.S1"
    );
    assert_eq!(
        reaction["listOfReactants"][1]["pk"],
        "m/SpeciesReference:R1.reactant.S1.1"
    );
    // a Level 2 law writes its parameters as `parameter` and libsbml answers a `Parameter` of
    // them, which the report types a `LocalParameter` like the one of Level 3
    let parameter = &reaction["kineticLaw"]["listOfLocalParameters"][0];
    assert_eq!(parameter["sbmlType"], "LocalParameter");
    assert_eq!(parameter["pk"], "m/LocalParameter:R1.kineticLaw.k");
    assert_eq!(parameter["value"], 1.0);
}

#[test]
fn a_reaction_of_level_2_version_1_is_reported_without_a_fast() {
    // Level 2 Version 1 defined no default for `fast`, which Level 2 Version 2 added, so the
    // report of a reaction of such a document states nothing about it
    let sbml = br#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level2" level="2" version="1">
  <model id="m">
    <listOfCompartments><compartment id="c"/></listOfCompartments>
    <listOfSpecies><species id="S1" compartment="c"/></listOfSpecies>
    <listOfReactions>
      <reaction id="R1">
        <listOfReactants><speciesReference species="S1"/></listOfReactants>
      </reaction>
    </listOfReactions>
  </model>
</sbml>"#;
    let json = report_of(sbml);
    let reaction = &json["models"][0]["listOfReactions"][0];
    assert_eq!(reaction["reversible"], true);
    assert_eq!(reaction["fast"], serde_json::Value::Null);
}

#[test]
fn a_rule_of_a_document_which_names_no_variable_is_reported_with_an_empty_one() {
    // the variable is required of an assignment rule, so no valid document reaches this; the
    // report of an invalid one writes the empty string python writes and refuses nothing
    let sbml = br#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2">
  <model id="m">
    <listOfRules><assignmentRule/></listOfRules>
    <listOfInitialAssignments><initialAssignment/></listOfInitialAssignments>
  </model>
</sbml>"#;
    let json = report_of(sbml);
    let model = &json["models"][0];
    assert_eq!(model["listOfRules"][0]["variable"], "");
    assert_eq!(model["listOfInitialAssignments"][0]["symbol"], "");
    // neither carries an identifier of any kind, so the key is the place in its list, which is
    // the fallback of `pk::key` and not the digest of the xml python falls back to
    assert_eq!(model["listOfRules"][0]["pk"], "m/AssignmentRule:0");
    assert_eq!(
        model["listOfInitialAssignments"][0]["pk"],
        "m/InitialAssignment:0"
    );
}
