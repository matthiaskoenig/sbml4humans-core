//! The document model keeps what a reader puts into it.
use sbml4humans_model::{Document, Element, Model, Package, Rule, RuleKind};

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
fn a_model_of_no_elements_holds_every_list_of_the_core_empty() {
    // the order of a list is the order of the document, which the filler of
    // `sbml4humans-libsbml` keeps and its tests of a read assert
    let m = Model::default();
    assert!(m.function_definitions.is_empty() && m.unit_definitions.is_empty());
    assert!(m.compartments.is_empty() && m.species.is_empty() && m.parameters.is_empty());
    assert!(m.initial_assignments.is_empty() && m.rules.is_empty() && m.constraints.is_empty());
    assert!(m.reactions.is_empty() && m.events.is_empty());
}

#[test]
fn the_default_rule_is_the_one_class_which_carries_no_variable() {
    // `Rule::default()` carries no variable, and the algebraic rule is the one
    // of the three classes which sets none, so the default rule is no rule
    // with a variable missing
    let rule = Rule::default();
    assert_eq!(rule.kind, RuleKind::Algebraic);
    assert_eq!(rule.variable, None);
}
