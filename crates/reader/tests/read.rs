//! Native-independent encoding, formula and extension boundaries.
use sbml4humans_reader::{read, read_formula};
const XML: &str = "<sbml xmlns=\"http://www.sbml.org/sbml/level3/version2/core\" level=\"3\" version=\"2\"><model id=\"m\" name=\"café\"/></sbml>";
#[test]
fn legacy_and_utf16_sources_preserve_unicode() {
    let latin = format!("<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>{XML}")
        .chars()
        .map(|c| c as u8)
        .collect::<Vec<_>>();
    let mut little = vec![0xff, 0xfe];
    let mut big = vec![0xfe, 0xff];
    for unit in XML.encode_utf16() {
        little.extend(unit.to_le_bytes());
        big.extend(unit.to_be_bytes());
    }
    for source in [latin, little, big, XML.as_bytes().to_vec()] {
        let document = read(&source).unwrap();
        assert_eq!(
            document.model.unwrap().element.name.as_deref(),
            Some("café")
        );
    }
    assert!(read(&[0xff, 0xfe, 0]).is_err());
}
#[test]
fn legacy_formula_grouping_uses_heap_stacks() {
    let formula = format!("{}x{}", "(".repeat(10_000), ")".repeat(10_000));
    let ast = read_formula(&formula).unwrap().unwrap();
    assert_eq!(ast.name.as_deref(), Some("x"));
    for invalid in ["x +", "f(,x)", "(x", "x y"] {
        assert!(read_formula(invalid).is_err(), "{invalid}");
    }
}
#[test]
fn a_document_with_only_model_definitions_is_preserved() {
    let xml = "<sbml xmlns=\"http://www.sbml.org/sbml/level3/version2/core\" xmlns:comp=\"http://www.sbml.org/sbml/level3/version1/comp/version1\" level=\"3\" version=\"2\" comp:required=\"true\"><comp:listOfModelDefinitions><comp:modelDefinition id=\"library\"/></comp:listOfModelDefinitions></sbml>";
    let doc = read(xml.as_bytes()).unwrap();
    assert!(doc.model.is_none());
    assert_eq!(
        doc.model_definitions[0].element.id.as_deref(),
        Some("library")
    );
}
