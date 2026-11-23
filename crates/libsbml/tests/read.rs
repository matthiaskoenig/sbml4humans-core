//! What libsbml reads of a document, against the minimal model of sbml4humans.

use sbml4humans_libsbml::{ReadError, read};
use sbml4humans_model::{AstKind, MAX_AST_DEPTH, RuleKind};

fn minimal() -> Vec<u8> {
    std::fs::read("../../testdata/minimal_model.xml").unwrap()
}

#[test]
fn reads_the_level_version_and_packages_of_a_document() {
    let doc = read(&minimal()).unwrap();
    assert_eq!((doc.level, doc.version), (3, 1));
    let packages: Vec<_> = doc
        .packages
        .iter()
        .map(|p| (p.prefix.as_str(), p.version, p.required))
        .collect();
    assert_eq!(packages, vec![("comp", 1, true), ("fbc", 2, false)]);
    assert!(doc.element.notes.as_deref().unwrap().contains("sbmlutils"));
}

#[test]
fn reads_the_compartments_species_and_parameters_in_document_order() {
    let m = read(&minimal()).unwrap().model.unwrap();
    assert_eq!(m.element.id.as_deref(), Some("minimal_model"));
    assert_eq!(m.compartments.len(), 1);
    assert_eq!(m.compartments[0].element.id.as_deref(), Some("cell"));
    assert_eq!(m.compartments[0].spatial_dimensions, Some(3.0));
    assert_eq!(m.compartments[0].size, Some(1.0));
    assert_eq!(m.compartments[0].constant, Some(true));
    assert_eq!(
        m.compartments[0].element.xml.as_deref(),
        Some("<compartment id=\"cell\" spatialDimensions=\"3\" size=\"1\" constant=\"true\"/>")
    );
    let species: Vec<_> = m
        .species
        .iter()
        .map(|s| s.element.id.as_deref().unwrap())
        .collect();
    assert_eq!(species, vec!["S1", "S2"]);
    assert_eq!(m.species[0].compartment, "cell");
    assert_eq!(m.species[0].initial_concentration, Some(10.0));
    assert_eq!(m.species[0].initial_amount, None);
    assert_eq!(m.parameters.len(), 1);
    let k1 = &m.parameters[0];
    assert_eq!(k1.element.meta_id.as_deref(), Some("meta_k1"));
    assert_eq!(k1.element.sbo.as_deref(), Some("SBO:0000002"));
    assert_eq!(k1.element.name.as_deref(), Some("rate constant for J0"));
    assert_eq!(k1.value, Some(0.1));
    assert!(
        k1.element
            .annotation
            .as_deref()
            .unwrap()
            .contains("bqbiol:is")
    );
}

#[test]
fn every_element_carries_the_line_it_starts_on() {
    let m = read(&minimal()).unwrap().model.unwrap();
    assert!(m.compartments[0].element.line > 0);
    assert!(m.species[1].element.line > m.species[0].element.line);
}

/// A Level 3 Version 2 document of the element types of the core which carry a math.
///
/// It holds what the fill of those types has to tell apart: a function
/// definition whose math is a lambda, a unit definition of two units and one
/// of a single unit with a fractional exponent, an initial assignment without
/// an id and one with an id next to its symbol, an assignment rule, a rate
/// rule and two algebraic rules, and a constraint with a message next to one
/// without.
const CORE_ELEMENTS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2">
  <model id="core_elements">
    <listOfFunctionDefinitions>
      <functionDefinition id="f" name="twice">
        <math xmlns="http://www.w3.org/1998/Math/MathML">
          <lambda>
            <bvar><ci>x</ci></bvar>
            <apply><times/><cn type="integer">2</cn><ci>x</ci></apply>
          </lambda>
        </math>
      </functionDefinition>
    </listOfFunctionDefinitions>
    <listOfUnitDefinitions>
      <unitDefinition id="mmole_per_litre">
        <listOfUnits>
          <unit kind="mole" exponent="1" scale="-3" multiplier="1"/>
          <unit kind="litre" exponent="-1" scale="0" multiplier="1"/>
        </listOfUnits>
      </unitDefinition>
      <unitDefinition id="root_metre">
        <listOfUnits>
          <unit kind="metre" exponent="0.5" scale="-2" multiplier="2.5"/>
        </listOfUnits>
      </unitDefinition>
    </listOfUnitDefinitions>
    <listOfCompartments>
      <compartment id="c" spatialDimensions="3" size="1" constant="true"/>
    </listOfCompartments>
    <listOfSpecies>
      <species id="S1" compartment="c" initialConcentration="1" hasOnlySubstanceUnits="false" boundaryCondition="false" constant="false"/>
      <species id="S2" compartment="c" initialConcentration="0" hasOnlySubstanceUnits="false" boundaryCondition="false" constant="false"/>
      <species id="S3" compartment="c" initialConcentration="0" hasOnlySubstanceUnits="false" boundaryCondition="false" constant="false"/>
    </listOfSpecies>
    <listOfParameters>
      <parameter id="k1" value="0.1" constant="true"/>
      <parameter id="k2" constant="false"/>
      <parameter id="k3" value="0" constant="false"/>
      <parameter id="k4" constant="false"/>
    </listOfParameters>
    <listOfInitialAssignments>
      <initialAssignment symbol="S2">
        <math xmlns="http://www.w3.org/1998/Math/MathML">
          <apply><times/><ci>k1</ci><cn type="integer">2</cn></apply>
        </math>
      </initialAssignment>
      <initialAssignment id="ia_S3" symbol="S3">
        <math xmlns="http://www.w3.org/1998/Math/MathML"><ci>k1</ci></math>
      </initialAssignment>
    </listOfInitialAssignments>
    <listOfRules>
      <assignmentRule variable="k2">
        <math xmlns="http://www.w3.org/1998/Math/MathML">
          <apply><times/><cn type="integer">2</cn><ci>k1</ci></apply>
        </math>
      </assignmentRule>
      <rateRule variable="k3">
        <math xmlns="http://www.w3.org/1998/Math/MathML"><ci>k1</ci></math>
      </rateRule>
      <algebraicRule>
        <math xmlns="http://www.w3.org/1998/Math/MathML">
          <apply><minus/><ci>k4</ci><ci>k1</ci></apply>
        </math>
      </algebraicRule>
      <algebraicRule metaid="meta_algebraic">
        <math xmlns="http://www.w3.org/1998/Math/MathML">
          <apply><minus/><ci>S1</ci><ci>k1</ci></apply>
        </math>
      </algebraicRule>
    </listOfRules>
    <listOfConstraints>
      <constraint>
        <math xmlns="http://www.w3.org/1998/Math/MathML">
          <apply><gt/><ci>S1</ci><cn type="integer">0</cn></apply>
        </math>
        <message>
          <p xmlns="http://www.w3.org/1999/xhtml">S1 has to stay positive</p>
        </message>
      </constraint>
      <constraint>
        <math xmlns="http://www.w3.org/1998/Math/MathML">
          <apply><lt/><ci>S2</ci><cn type="integer">10</cn></apply>
        </math>
      </constraint>
    </listOfConstraints>
  </model>
</sbml>"#;

#[test]
fn the_elements_which_carry_a_math_are_filled_from_the_document() {
    let document = read(CORE_ELEMENTS.as_bytes()).unwrap();
    let model = document.model.unwrap();
    assert_eq!(model.function_definitions.len(), 1);
    assert_eq!(
        model.function_definitions[0].element.id.as_deref(),
        Some("f")
    );
    assert!(model.function_definitions[0].math.is_some());
    assert_eq!(
        model.unit_definitions[0].units[0].kind.as_deref(),
        Some("mole")
    );
    assert_eq!(model.unit_definitions[0].units[0].scale, Some(-3));
    assert_eq!(model.unit_definitions[1].units[0].exponent, Some(0.5));
    assert_eq!(model.initial_assignments[0].symbol, "S2");
    // libsbml aliases getId() to the symbol, the report carries the id attribute alone
    assert_eq!(model.initial_assignments[0].element.id, None);
    assert_eq!(model.rules[0].kind, RuleKind::Assignment);
    assert_eq!(model.rules[0].variable.as_deref(), Some("k2"));
    assert_eq!(model.rules[2].kind, RuleKind::Algebraic);
    assert_eq!(model.rules[2].variable, None);
    assert_eq!(model.constraints.len(), 2);
    assert!(
        model.constraints[0]
            .message
            .as_ref()
            .unwrap()
            .starts_with("<message>")
    );
    assert_eq!(model.constraints[1].message, None);
}

#[test]
fn the_math_of_an_element_is_the_tree_of_the_document_and_the_shared_fields_are_filled() {
    let model = read(CORE_ELEMENTS.as_bytes()).unwrap().model.unwrap();
    let f = &model.function_definitions[0];
    assert_eq!(f.element.name.as_deref(), Some("twice"));
    assert!(
        f.element
            .xml
            .as_deref()
            .unwrap()
            .starts_with("<functionDefinition")
    );
    assert!(f.element.line > 0);
    let lambda = f.math.as_ref().unwrap();
    assert_eq!(lambda.kind, AstKind::Lambda);
    // the bound variable and the body of the lambda
    assert_eq!(lambda.children.len(), 2);
    assert_eq!(lambda.children[1].kind, AstKind::Times);
    assert_eq!(
        model.initial_assignments[1].math.as_ref().unwrap().kind,
        AstKind::Name
    );
    assert_eq!(model.rules[1].math.as_ref().unwrap().kind, AstKind::Name);
    assert_eq!(
        model.constraints[0].math.as_ref().unwrap().kind,
        AstKind::RelationalGt
    );
}

#[test]
fn a_unit_definition_keeps_its_units_in_the_order_of_the_document() {
    let model = read(CORE_ELEMENTS.as_bytes()).unwrap().model.unwrap();
    let ids: Vec<_> = model
        .unit_definitions
        .iter()
        .map(|u| u.element.id.as_deref().unwrap())
        .collect();
    assert_eq!(ids, vec!["mmole_per_litre", "root_metre"]);
    let kinds: Vec<_> = model.unit_definitions[0]
        .units
        .iter()
        .map(|u| u.kind.as_deref().unwrap())
        .collect();
    assert_eq!(kinds, vec!["mole", "litre"]);
    assert_eq!(model.unit_definitions[0].units[0].exponent, Some(1.0));
    assert_eq!(model.unit_definitions[0].units[0].multiplier, Some(1.0));
    assert_eq!(model.unit_definitions[0].units[1].exponent, Some(-1.0));
    let root = &model.unit_definitions[1].units[0];
    assert_eq!(root.kind.as_deref(), Some("metre"));
    assert_eq!(root.scale, Some(-2));
    assert_eq!(root.multiplier, Some(2.5));
}

#[test]
fn an_initial_assignment_and_a_rule_carry_the_id_attribute_rather_than_their_target() {
    let model = read(CORE_ELEMENTS.as_bytes()).unwrap().model.unwrap();
    assert_eq!(model.initial_assignments.len(), 2);
    assert_eq!(
        model.initial_assignments[1].element.id.as_deref(),
        Some("ia_S3")
    );
    assert_eq!(model.initial_assignments[1].symbol, "S3");
    // every rule of the document sets its target in `variable`, none an id
    for rule in &model.rules {
        assert_eq!(rule.element.id, None, "{rule:?}");
    }
    assert_eq!(
        model.rules[3].element.meta_id.as_deref(),
        Some("meta_algebraic")
    );
}

#[test]
fn the_rules_keep_their_kind_and_their_order() {
    let model = read(CORE_ELEMENTS.as_bytes()).unwrap().model.unwrap();
    let kinds: Vec<_> = model.rules.iter().map(|r| r.kind).collect();
    assert_eq!(
        kinds,
        vec![
            RuleKind::Assignment,
            RuleKind::Rate,
            RuleKind::Algebraic,
            RuleKind::Algebraic,
        ]
    );
    assert_eq!(model.rules[1].variable.as_deref(), Some("k3"));
    assert_eq!(model.rules[3].variable, None);
}

#[test]
fn a_math_of_the_document_which_is_nested_too_deep_ends_the_read() {
    // the reader of a math is the one place which refuses a math, and it
    // refuses one of a document as it refuses one of a string
    let mut body = String::new();
    for _ in 0..MAX_AST_DEPTH {
        body.push_str("<apply><plus/><cn type=\"integer\">1</cn>");
    }
    body.push_str("<cn type=\"integer\">1</cn>");
    for _ in 0..MAX_AST_DEPTH {
        body.push_str("</apply>");
    }
    let sbml = format!(
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
    );
    assert!(matches!(read(sbml.as_bytes()), Err(ReadError::MathTooDeep)));
}

#[test]
fn an_element_without_a_math_is_filled_without_one() {
    // the math is optional in Level 3 Version 2, every one of these elements is empty
    let sbml = br#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2">
  <model id="m">
    <listOfFunctionDefinitions><functionDefinition id="f"/></listOfFunctionDefinitions>
    <listOfInitialAssignments><initialAssignment/></listOfInitialAssignments>
    <listOfRules><algebraicRule/></listOfRules>
    <listOfConstraints><constraint/></listOfConstraints>
  </model>
</sbml>"#;
    let model = read(sbml).unwrap().model.unwrap();
    assert_eq!(model.function_definitions[0].math, None);
    assert_eq!(model.initial_assignments[0].math, None);
    assert_eq!(model.rules[0].math, None);
    assert_eq!(model.constraints[0].math, None);
    // the symbol and the variable are read unguarded, as python reads them, and
    // the empty string libsbml answers of a document which names none is nothing
    assert_eq!(model.initial_assignments[0].symbol, "");
    assert_eq!(model.rules[0].variable, None);
}

#[test]
fn a_document_without_a_model_which_libsbml_logs_an_error_of_is_a_libsbml_error() {
    // the model is required in L3V1, so libsbml logs the error 20201 of the read
    let sbml = br#"<?xml version="1.0" encoding="UTF-8"?><sbml xmlns="http://www.sbml.org/sbml/level3/version1/core" level="3" version="1"/>"#;
    match read(sbml) {
        Err(ReadError::Libsbml(message)) => assert!(message.contains("20201"), "{message}"),
        other => panic!("expected a Libsbml error, got {other:?}"),
    }
}

#[test]
fn a_well_formed_document_without_a_model_which_libsbml_accepts_holds_no_model() {
    // the model is optional in L3V2, so libsbml logs no error and there is none to report
    let sbml = br#"<?xml version="1.0" encoding="UTF-8"?><sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2"/>"#;
    assert!(matches!(read(sbml), Err(ReadError::NoModel(_))));
}

#[test]
fn bytes_which_do_not_parse_are_a_libsbml_error_and_not_a_missing_model() {
    for source in [b"not xml at all".as_slice(), b"<sbml><model>", b""] {
        match read(source) {
            Err(ReadError::Libsbml(message)) => {
                assert!(message.contains("not well-formed"), "{message}");
            }
            other => panic!("expected a Libsbml error for {source:?}, got {other:?}"),
        }
    }
}

#[test]
fn bytes_which_are_no_document_are_an_error_and_not_a_panic() {
    for source in [
        b"".as_slice(),
        b"not xml at all",
        b"<sbml><model>",
        b"\x1f\x8b",
    ] {
        assert!(read(source).is_err(), "expected an error for {source:?}");
    }
}

#[test]
fn a_latin1_document_reads_with_the_name_transcoded_to_utf8() {
    // the name of the compartment is the one byte 0xE4, which is not valid UTF-8 on its own
    let sbml = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><sbml xmlns=\"http://www.sbml.org/sbml/level3/version1/core\" level=\"3\" version=\"1\"><model id=\"m\"><listOfCompartments><compartment id=\"c\" name=\"\xe4\" constant=\"true\"/></listOfCompartments></model></sbml>";
    let doc = read(sbml).expect("a latin-1 document reads");
    let compartment = &doc.model.unwrap().compartments[0];
    assert_eq!(compartment.element.id.as_deref(), Some("c"));
    // expat transcodes the encoding the document declares, so what libsbml hands the filler is
    // already the UTF-8 of `ä`. The lossy conversion of the filler is defense in depth against a
    // string of libsbml which is no UTF-8, not the path this document takes.
    assert_eq!(compartment.element.name.as_deref(), Some("ä"));
}

// what the constraints of the crate ask of it, next to what it reads

#[test]
fn a_nul_byte_in_the_source_is_an_error_and_not_a_truncation() {
    let mut source = std::fs::read("../../testdata/minimal_model.xml").unwrap();
    source.insert(200, 0);
    match read(&source) {
        Err(ReadError::Libsbml(message)) => assert!(message.contains("nul byte"), "{message}"),
        other => panic!("expected a Libsbml error, got {other:?}"),
    }
}

#[test]
fn an_attribute_is_none_exactly_when_libsbml_reports_it_as_not_set() {
    let sbml = br#"<?xml version="1.0" encoding="UTF-8"?><sbml xmlns="http://www.sbml.org/sbml/level2/version4" level="2" version="4"><model id="m"><listOfCompartments><compartment id="c" size="2"/></listOfCompartments><listOfSpecies><species id="s" compartment="c" initialAmount="3"/></listOfSpecies></model></sbml>"#;
    let doc = read(sbml).unwrap();
    assert_eq!((doc.level, doc.version), (2, 4));
    // libsbml gives every Level 2 document the layout and the render plugin, which the
    // report of the python implementation lists as well. Which plugins a document gets
    // depends on the package set the linked libsbml was built with: scripts/build-libsbml.sh
    // pins that set to the one python-libsbml 5.21.2 registers,
    // testdata/oracles/MANIFEST.json records it for both sides of the gate, and a prefix
    // built without layout or without render fails here.
    let packages: Vec<_> = doc.packages.iter().map(|p| p.prefix.as_str()).collect();
    assert_eq!(packages, vec!["layout", "render"]);
    let m = doc.model.unwrap();
    assert_eq!(m.compartments[0].size, Some(2.0));
    // libsbml reports the defaults the specification of Level 2 gives the
    // attributes of a compartment as set, which the python report shows too
    assert_eq!(m.compartments[0].spatial_dimensions, Some(3.0));
    assert_eq!(m.compartments[0].constant, Some(true));
    assert_eq!(m.compartments[0].units, None);
    assert_eq!(m.species[0].initial_amount, Some(3.0));
    assert_eq!(m.species[0].initial_concentration, None);
    assert_eq!(m.species[0].substance_units, None);
    assert_eq!(m.species[0].compartment, "c");
}

#[test]
fn the_reads_of_many_threads_do_not_race() {
    let source = std::fs::read("../../testdata/minimal_model.xml").unwrap();
    let readers: Vec<_> = (0..8)
        .map(|_| {
            let source = source.clone();
            std::thread::spawn(move || read(&source).unwrap().model.unwrap().species.len())
        })
        .collect();
    for reader in readers {
        assert_eq!(reader.join().unwrap(), 2);
    }
}
