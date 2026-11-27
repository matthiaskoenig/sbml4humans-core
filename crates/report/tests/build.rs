//! The report builder maps the document model onto the report.

use sbml4humans_libsbml::{ReadError, read};
use sbml4humans_model::{Compartment, Document, Element, MAX_AST_DEPTH, Model};
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

/// Every field key, excluding the authored primary keys of the graph's node map.
fn keys_of(value: &serde_json::Value) -> Vec<String> {
    let nodes = value.pointer("/linkGraph/nodes");
    let mut keys = Vec::new();
    let mut stack = vec![value];
    while let Some(value) = stack.pop() {
        match value {
            serde_json::Value::Object(fields) => {
                let identifier_map = nodes.is_some_and(|nodes| std::ptr::eq(value, nodes));
                for (key, value) in fields {
                    if !identifier_map {
                        keys.push(key.clone());
                    }
                    stack.push(value);
                }
            }
            serde_json::Value::Array(items) => stack.extend(items),
            _ => {}
        }
    }
    keys
}

// The JSON of a whole report, which is the contract the frontend reads: the six keys of the report
// of one document, which the python report writes the same six of, and the camelCase of every key
// below them. The key set is asserted whole, so a field added to the report without a line of the
// frontend which reads it fails here.
#[test]
fn the_json_of_a_whole_report_holds_the_keys_of_the_contract() {
    let json = report_of(CORE_ELEMENTS);
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "document",
            "externalModelDefinitions",
            "linkGraph",
            "models",
            "odeError",
            "odeSystem",
        ]
    );
    assert_eq!(json["document"]["sbmlType"], "SBMLDocument");
    assert_eq!(json["models"].as_array().unwrap().len(), 1);
    assert_eq!(json["externalModelDefinitions"], serde_json::json!([]));
    // The graph is populated; keep its field contract checked alongside the report.
    assert_eq!(
        json["linkGraph"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["edges", "nodes"]
    );
    assert!(
        json["linkGraph"]["nodes"]
            .as_object()
            .unwrap()
            .contains_key("document/SBMLDocument:document")
    );
    // Equations remain a later slice.
    assert_eq!(json["odeSystem"], serde_json::Value::Null);
    assert_eq!(json["odeError"], serde_json::Value::Null);
    // no key of the report holds an underscore: a type whose `rename_all` was forgotten would
    // write `sbml_type` and `list_of_species`, which no component of the frontend reads
    let snake: Vec<String> = keys_of(&json)
        .into_iter()
        .filter(|key| key.contains('_'))
        .collect();
    assert!(snake.is_empty(), "{snake:?}");
}

/// A Level 3 document whose references set the three doubles JSON has no literal for.
const NON_FINITE_STOICHIOMETRY: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2">
  <model id="nan_stoich">
    <listOfCompartments><compartment id="c" constant="true"/></listOfCompartments>
    <listOfSpecies>
      <species id="S1" compartment="c" hasOnlySubstanceUnits="false" boundaryCondition="false" constant="false"/>
      <species id="S2" compartment="c" hasOnlySubstanceUnits="false" boundaryCondition="false" constant="false"/>
      <species id="S3" compartment="c" hasOnlySubstanceUnits="false" boundaryCondition="false" constant="false"/>
    </listOfSpecies>
    <listOfReactions>
      <reaction id="J0" reversible="false">
        <listOfReactants>
          <speciesReference species="S1" stoichiometry="NaN" constant="true"/>
          <speciesReference species="S2" stoichiometry="INF" constant="true"/>
        </listOfReactants>
        <listOfProducts>
          <speciesReference species="S3" stoichiometry="-INF" constant="true"/>
        </listOfProducts>
      </reaction>
    </listOfReactions>
  </model>
</sbml>"#;

// A double of SBML may be infinite or not a number (core §3.1.5), which JSON has no literal for:
// python writes the three as the strings `"Infinity"`, `"-Infinity"` and `"NaN"`
// (`ser_json_inf_nan="strings"`, `model.py:32-34`), and a `null` instead would read as an
// attribute the document does not set at all. The three values and the equation below were
// generated by the python implementation from this very document.
#[test]
fn a_stoichiometry_which_is_not_finite_is_the_string_the_contract_names() {
    let json = report_of(NON_FINITE_STOICHIOMETRY);
    let reaction = &json["models"][0]["listOfReactions"][0];
    assert_eq!(reaction["listOfReactants"][0]["stoichiometry"], "NaN");
    assert_eq!(reaction["listOfReactants"][1]["stoichiometry"], "Infinity");
    assert_eq!(reaction["listOfProducts"][0]["stoichiometry"], "-Infinity");
    // the half equation writes `?` for a coefficient which is not a number and the `str()` of the
    // float for an infinite one
    assert_eq!(reaction["equation"], "? S1 + inf S2 ➞ -inf S3");
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

// Every element type of the report which carries a math, with the math of each one absent. The
// nulls and the keys below were generated by the python implementation from this very document.
#[test]
fn an_element_whose_math_is_unset_is_reported_with_a_null_math() {
    let json = report_of(NO_MATHS);
    let model = &json["models"][0];
    let null = serde_json::Value::Null;
    for list in [
        "listOfFunctionDefinitions",
        "listOfInitialAssignments",
        "listOfRules",
        "listOfConstraints",
    ] {
        assert_eq!(model[list][0]["math"], null, "{list}");
    }
    let law = &model["listOfReactions"][0]["kineticLaw"];
    assert_eq!(law["pk"], "no_maths/KineticLaw:J0.kineticLaw");
    assert_eq!(law["math"], null);
    let event = &model["listOfEvents"][0];
    for element in ["trigger", "priority", "delay"] {
        assert_eq!(event[element]["math"], null, "{element}");
    }
    let assignment = &event["listOfEventAssignments"][0];
    assert_eq!(assignment["pk"], "no_maths/EventAssignment:E0.p");
    assert_eq!(assignment["math"], null);
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
    // the reaction carries no identifier of any kind, so the key is the name of its element and
    // its place in the list, and the reference below it is keyed after that key
    assert_eq!(bare["pk"], "core_elements/Reaction:reaction.1");
    assert_eq!(
        bare["listOfReactants"][0]["pk"],
        "core_elements/SpeciesReference:reaction.1.reactant.S3"
    );
    // the second event carries nothing, not even a trigger, and is keyed by its place
    let bare_event = &json["models"][0]["listOfEvents"][1];
    assert_eq!(bare_event["pk"], "core_elements/Event:event.1");
    assert_eq!(bare_event["trigger"], serde_json::Value::Null);
    assert_eq!(bare_event["priority"], serde_json::Value::Null);
    assert_eq!(bare_event["delay"], serde_json::Value::Null);
    assert_eq!(bare_event["listOfEventAssignments"], serde_json::json!([]));
}

/// A document of three algebraic rules, the second of them keyed by a metaId of its own.
const ALGEBRAIC_RULES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2">
  <model id="algebraic">
    <listOfParameters>
      <parameter id="k1" value="1" constant="true"/>
      <parameter id="k2" constant="false"/>
    </listOfParameters>
    <listOfRules>
      <algebraicRule>
        <math xmlns="http://www.w3.org/1998/Math/MathML"><apply><minus/><ci>k2</ci><ci>k1</ci></apply></math>
      </algebraicRule>
      <algebraicRule metaid="meta_algebraic">
        <math xmlns="http://www.w3.org/1998/Math/MathML"><apply><minus/><ci>k1</ci><ci>k2</ci></apply></math>
      </algebraicRule>
      <algebraicRule>
        <math xmlns="http://www.w3.org/1998/Math/MathML"><apply><plus/><ci>k1</ci><ci>k2</ci></apply></math>
      </algebraicRule>
    </listOfRules>
  </model>
</sbml>"#;

// The place which keys an algebraic rule without an identifier of its own counts every algebraic
// rule of the model, as `rules` of `sbmlinfo.py:1031-1049` counts it: a rule which carries a metaId
// is keyed by that metaId and takes its place in the count all the same, so the rule after it is
// `algebraicRule.2` and not `algebraicRule.1`. A counter of the rules without an identifier would
// pass every other test of the report, since no other document of the suite holds a third algebraic
// rule. The three keys were generated by the python implementation from this very document.
#[test]
fn an_algebraic_rule_is_keyed_by_its_place_among_every_algebraic_rule() {
    let json = report_of(ALGEBRAIC_RULES);
    let pks: Vec<&str> = json["models"][0]["listOfRules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["pk"].as_str().unwrap())
        .collect();
    assert_eq!(
        pks,
        [
            "algebraic/AlgebraicRule:algebraicRule.0",
            "algebraic/AlgebraicRule:meta_algebraic",
            "algebraic/AlgebraicRule:algebraicRule.2",
        ]
    );
}

/// A Level 2 Version 4 document whose reaction states none of the attributes its level defaults.
///
/// Its third reactant carries a `stoichiometryMath`, the one case in which a reference of Level 1
/// or Level 2 has no stoichiometry of its own although the level defines a default.
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
          <speciesReference species="S2">
            <stoichiometryMath>
              <math xmlns="http://www.w3.org/1998/Math/MathML"><cn type="integer">3</cn></math>
            </stoichiometryMath>
          </speciesReference>
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
    // the third reactant writes a `stoichiometryMath`, which is a stoichiometry of its own: the
    // default of the level is not the value of such a reference, so the report states none, and the
    // half equation names the species without a coefficient
    assert_eq!(
        reaction["listOfReactants"][2]["pk"],
        "m/SpeciesReference:R1.reactant.S2"
    );
    assert_eq!(
        reaction["listOfReactants"][2]["stoichiometry"],
        serde_json::Value::Null
    );
    // the `constant` of a species reference is a Level 3 attribute
    assert_eq!(
        reaction["listOfReactants"][0]["constant"],
        serde_json::Value::Null
    );
    assert_eq!(reaction["equation"], "S1 + S1 + S2 ⇆ 2.0 S2");
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

/// A document whose reactions hold every shape of the key a parent derives from a place.
///
/// The first reaction carries an id and the second states nothing at all, which is the pair the
/// gate kept excluded while python keyed the second by a digest. The third carries a metaId which
/// reads like the place of the fourth, and the second reactant of the first carries one which reads
/// like the key its sibling is about; an SId holds no dot and can be neither, a metaId is an XML
/// `ID` and can be both.
const DERIVED_KEYS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2">
  <model id="m">
    <listOfCompartments>
      <compartment id="c" constant="true"/>
    </listOfCompartments>
    <listOfSpecies>
      <species id="S1" compartment="c" hasOnlySubstanceUnits="false" boundaryCondition="false" constant="false"/>
    </listOfSpecies>
    <listOfReactions>
      <reaction id="J0" reversible="false">
        <listOfReactants>
          <speciesReference species="S1" constant="true"/>
          <speciesReference metaid="J0.reactant.S1" species="S1" constant="true"/>
        </listOfReactants>
      </reaction>
      <reaction reversible="false">
        <listOfReactants>
          <speciesReference species="S1" constant="true"/>
        </listOfReactants>
      </reaction>
      <reaction metaid="reaction.3" reversible="false"/>
      <reaction reversible="false"/>
    </listOfReactions>
  </model>
</sbml>"#;

// A reaction which states nothing at all is keyed by the name of its element and its place in the
// list, and everything below it is keyed after that key. It is the shape the gate had to exclude
// while python keyed such an element by the sha1 digest of its xml, which no port could match, and
// which python has keyed by its place since 0.13.0. The place counts over the whole list, so the
// reaction which carries the id takes the place 0 and the one after it is `reaction.1`.
#[test]
fn a_reaction_which_states_nothing_is_keyed_by_its_place_and_keys_its_reference() {
    let json = report_of(DERIVED_KEYS);
    let reactions = &json["models"][0]["listOfReactions"];
    assert_eq!(reactions[0]["pk"], "m/Reaction:J0");
    assert_eq!(
        reactions[0]["listOfReactants"][0]["pk"],
        "m/SpeciesReference:J0.reactant.S1.1"
    );
    assert_eq!(reactions[1]["pk"], "m/Reaction:reaction.1");
    assert_eq!(
        reactions[1]["listOfReactants"][0]["pk"],
        "m/SpeciesReference:reaction.1.reactant.S1"
    );
}

// The keys the file gives the siblings are reserved, so a key the report derives never names
// another element of that list (`_unique_keys`, `sbmlinfo.py:239-262`). A metaId can read like a
// derived key, both like the place of a sibling and like what a sibling is about, and the element
// whose key that is passes over it and takes the first occurrence which is free. An id cannot, so
// this is the one direction of the reservation a document can reach.
#[test]
fn a_metaid_which_reads_like_a_derived_key_is_reserved_against_it() {
    let json = report_of(DERIVED_KEYS);
    let reactions = &json["models"][0]["listOfReactions"];
    // the metaId of the second reactant is the key the first is about, so the first is numbered
    assert_eq!(
        reactions[0]["listOfReactants"][1]["pk"],
        "m/SpeciesReference:J0.reactant.S1"
    );
    assert_eq!(
        reactions[0]["listOfReactants"][0]["pk"],
        "m/SpeciesReference:J0.reactant.S1.1"
    );
    // and the metaId of the third reaction is the place of the fourth, which is numbered as well
    assert_eq!(reactions[2]["pk"], "m/Reaction:reaction.3");
    assert_eq!(reactions[3]["pk"], "m/Reaction:reaction.3.1");
}

/// A kinetic law whose local parameters hold every shape of the key their law gives them.
///
/// SBML requires an id of a local parameter and makes it unique within its law (core §4.11.6), so
/// the first three parameters are a document which is invalid in two ways a reader still reads.
const LOCAL_PARAMETERS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2">
  <model id="m">
    <listOfCompartments>
      <compartment id="c" constant="true"/>
    </listOfCompartments>
    <listOfSpecies>
      <species id="S1" compartment="c" hasOnlySubstanceUnits="false" boundaryCondition="false" constant="false"/>
    </listOfSpecies>
    <listOfReactions>
      <reaction id="J0" reversible="false">
        <listOfReactants>
          <speciesReference species="S1" constant="true"/>
        </listOfReactants>
        <kineticLaw>
          <math xmlns="http://www.w3.org/1998/Math/MathML"><ci>k</ci></math>
          <listOfLocalParameters>
            <localParameter value="1"/>
            <localParameter id="k" value="2"/>
            <localParameter id="k" value="3"/>
            <localParameter metaid="J0.kineticLaw.m" id="n" value="4"/>
            <localParameter id="m" value="5"/>
          </listOfLocalParameters>
        </kineticLaw>
      </reaction>
    </listOfReactions>
  </model>
</sbml>"#;

// A local parameter is keyed by its law and its id and never by the id alone, since the id is
// scoped to the law and two laws may each hold one of the same id (`use_id=False`,
// `sbmlinfo.py:1345-1375`). A parameter which carries no id is keyed by its place among the
// parameters, a repeated id is told apart by its occurrence, and the metaId of a parameter keys it
// and is reserved against the keys of its siblings.
#[test]
fn a_local_parameter_is_keyed_by_its_law_and_a_repetition_by_its_occurrence() {
    let json = report_of(LOCAL_PARAMETERS);
    let parameters =
        &json["models"][0]["listOfReactions"][0]["kineticLaw"]["listOfLocalParameters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|lp| lp["pk"].as_str().unwrap())
            .collect::<Vec<&str>>();
    assert_eq!(
        parameters,
        &[
            "m/LocalParameter:J0.kineticLaw.localParameter.0",
            "m/LocalParameter:J0.kineticLaw.k",
            "m/LocalParameter:J0.kineticLaw.k.1",
            "m/LocalParameter:J0.kineticLaw.m",
            "m/LocalParameter:J0.kineticLaw.m.1",
        ]
    );
    // the id of the fourth parameter is `n` and its metaId keys it, and the `id` field of the
    // report object carries the id of the parameter whatever keys it
    let fourth = &json["models"][0]["listOfReactions"][0]["kineticLaw"]["listOfLocalParameters"][3];
    assert_eq!(fourth["id"], "n");
    assert_eq!(fourth["metaId"], "J0.kineticLaw.m");
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
    // neither carries an identifier of any kind and neither names a target, so the key is the
    // name of its element and its place in its list
    assert_eq!(
        model["listOfRules"][0]["pk"],
        "m/AssignmentRule:assignmentRule.0"
    );
    assert_eq!(
        model["listOfInitialAssignments"][0]["pk"],
        "m/InitialAssignment:initialAssignment.0"
    );
}

/// A Level 3 Version 2 document whose one rule holds a flat sum of `terms` terms.
fn document_with_a_flat_sum(terms: usize) -> Vec<u8> {
    let mut body = String::from("<apply><plus/>");
    for _ in 0..terms {
        body.push_str("<cn type=\"integer\">1</cn>");
    }
    body.push_str("</apply>");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2">
  <model id="m">
    <listOfParameters><parameter id="p" constant="false"/></listOfParameters>
    <listOfRules>
      <assignmentRule variable="p">
        <math xmlns="http://www.w3.org/1998/Math/MathML">{body}</math>
      </assignmentRule>
    </listOfRules>
  </model>
</sbml>"#
    )
    .into_bytes()
}

#[test]
fn a_wide_sum_at_the_bound_is_reported_in_full() {
    // The MathML reader of libsbml nests a flat n-ary sum into a binary tree to
    // the left, one apply per term, so a flat sum of `MAX_AST_DEPTH` terms is a
    // tree of exactly that many levels and the whole report of the document
    // rests on every walk of such a tree costing no call frame per level. The
    // bound of 1,000 this port had before this slice refused a sum of 1,001
    // terms and the document with it, where the python implementation reports
    // it in full.
    //
    // This runs on the ordinary thread of a test, which cargo gives two
    // megabytes, and that is the point of it: the bound stands at half of what
    // `SBase::toSBML()` of libsbml survives on such a stack, so a document at
    // the bound has to be read and reported within it. A run which dies here
    // rather than failing says the bound is too high; the doc comment of
    // `MAX_AST_DEPTH` carries the measurement.
    let json = report_of(&document_with_a_flat_sum(MAX_AST_DEPTH));
    let math = &json["models"][0]["listOfRules"][0]["math"];
    let formula = math["formula"].as_str().expect("the formula is written");
    assert_eq!(formula.matches(" + ").count(), MAX_AST_DEPTH - 1);
    let latex = math["latex"].as_str().expect("and so is the latex");
    assert_eq!(latex.matches('+').count(), MAX_AST_DEPTH - 1);
    // the leading space is where the `$` of the inline math of `mmltex.xsl`
    // stood, which every latex of the report carries
    assert!(latex.starts_with(" 1+1+1"), "{}", &latex[..16]);
}

#[test]
fn a_sum_one_term_wider_than_the_bound_costs_the_whole_report() {
    // The refusal is the reader's and ends the read of the document, so the
    // blast radius of a math past the bound is the whole report and not the
    // math of the one element which holds it. What the bound stands at, and
    // why a document which reaches it is one no model holds, is the doc
    // comment of `MAX_AST_DEPTH`.
    let refused = read(&document_with_a_flat_sum(MAX_AST_DEPTH + 1));
    assert!(
        matches!(refused, Err(ReadError::MathTooDeep)),
        "one term past the bound"
    );
}
