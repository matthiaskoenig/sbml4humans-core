//! What libsbml reads of a document, against the minimal model of sbml4humans.

use sbml4humans_libsbml::{ReadError, read};

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
