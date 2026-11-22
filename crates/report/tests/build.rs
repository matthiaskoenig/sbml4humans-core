//! The report builder maps the document model onto the report.

use sbml4humans_model::{Compartment, Document, Element, Model};
use sbml4humans_report::{build, pk};

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
    let report = build(&doc_with(model));
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
    let report = build(&doc_with(model));
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
    let report = build(&doc_with(model));
    assert_eq!(report.models[0].sbase.pk, "model/Model:model");
    assert_eq!(
        report.models[0].list_of_compartments[0].sbase.pk,
        "model/Compartment:cell"
    );
}
