//! The document model keeps what a reader puts into it.
use sbml4humans_model::{Document, Element, Model, Package};

#[test]
fn a_document_keeps_its_level_version_packages_and_model() {
    let doc = Document {
        level: 3,
        version: 1,
        packages: vec![
            Package {
                prefix: "comp".into(),
                version: 1,
                required: true,
            },
            Package {
                prefix: "fbc".into(),
                version: 2,
                required: false,
            },
        ],
        element: Element {
            notes: Some("<notes/>".into()),
            ..Element::default()
        },
        model: Some(Model {
            element: Element {
                id: Some("m".into()),
                ..Element::default()
            },
            ..Model::default()
        }),
    };
    assert_eq!(doc.packages[0].prefix, "comp");
    assert!(doc.packages[0].required);
    assert_eq!(doc.model.as_ref().unwrap().element.id.as_deref(), Some("m"));
    assert_eq!(doc.element.notes.as_deref(), Some("<notes/>"));
}

#[test]
fn every_list_of_a_model_keeps_the_order_of_the_document() {
    let m = Model::default();
    assert!(m.function_definitions.is_empty() && m.unit_definitions.is_empty());
    assert!(m.compartments.is_empty() && m.species.is_empty() && m.parameters.is_empty());
    assert!(m.initial_assignments.is_empty() && m.rules.is_empty() && m.constraints.is_empty());
}
