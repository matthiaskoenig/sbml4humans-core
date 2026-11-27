//! Native validation keeps diagnostics and confines comp to admitted sources.
use sbml4humans_libsbml::{
    ElementPositions, ValidationContent, ValidationSession, ValidationSource,
};

fn source(location: &str, xml: &[u8], uri: &str) -> ValidationSource {
    ValidationSource {
        location: location.into(),
        content: ValidationContent::Xml(xml.into()),
        uri: uri.into(),
        positions: ElementPositions {
            document_pk: "document/SBMLDocument:document".into(),
            elements: vec![(1, 1, 1, "model/Model:model".into())],
        },
    }
}

#[test]
fn diagnostics_are_ordered_and_checks_run_once() {
    let input = include_bytes!("../../../testdata/validation.xml");
    let mut session =
        ValidationSession::new(vec![source("./model.xml", input, "model.xml")]).unwrap();
    let read = session.issues("model.xml", false).unwrap();
    let checked = session.issues("./model.xml", true).unwrap();
    assert!(checked.len() > read.len());
    assert!(!checked.iter().any(|issue| issue.rule == 21152));
    assert!(
        checked
            .iter()
            .any(|issue| issue.rule == 10601 && issue.severity == "error")
    );
    assert!(
        checked
            .iter()
            .all(|issue| !issue.pk.is_empty() && !issue.message.is_empty())
    );
    assert_eq!(checked, session.issues("model.xml", true).unwrap());
}

const CHILD: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2"><model id="child"/></sbml>"#;
const MAIN: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" xmlns:comp="http://www.sbml.org/sbml/level3/version1/comp/version1" level="3" version="2" comp:required="true">
<comp:listOfExternalModelDefinitions><comp:externalModelDefinition comp:id="ext" comp:source="child.xml"/></comp:listOfExternalModelDefinitions>
<model id="root"><comp:listOfSubmodels><comp:submodel comp:id="s" comp:modelRef="ext"/></comp:listOfSubmodels></model></sbml>"#;

#[test]
fn comp_never_reads_an_unadmitted_file() {
    let directory =
        std::env::temp_dir().join(format!("s4h-closed-validation-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(directory.clone());
    std::fs::write(directory.join("child.xml"), CHILD).unwrap();
    let uri = directory.join("main.xml").to_string_lossy().into_owned();
    let mut closed = ValidationSession::new(vec![source("./main.xml", MAIN, &uri)]).unwrap();
    let refused = closed.issues("main.xml", true).unwrap();
    assert!(refused.iter().any(|issue| issue.severity == "error"));

    let child_uri = directory.join("child.xml").to_string_lossy().into_owned();
    let mut admitted = ValidationSession::new(vec![
        source("./main.xml", MAIN, &uri),
        source("./child.xml", CHILD, &child_uri),
    ])
    .unwrap();
    let issues = admitted.issues("main.xml", true).unwrap();
    assert!(
        !issues.iter().any(|issue| issue.severity == "error"),
        "{issues:?}"
    );
    // A previous admitted session must not leak its resolver into the next check.
    let mut closed_again = ValidationSession::new(vec![source("./main.xml", MAIN, &uri)]).unwrap();
    assert_eq!(refused, closed_again.issues("main.xml", true).unwrap());
}

#[test]
fn duplicate_normalized_locations_are_refused() {
    let result = ValidationSession::new(vec![
        source("./child.xml", CHILD, "one"),
        source("child.xml", CHILD, "two"),
    ]);
    assert!(result.is_err());
}
