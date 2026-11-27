//! The owned annotation data is independent of the lifetime of the native document.

use sbml4humans_libsbml::read;

fn annotated() -> sbml4humans_model::Document {
    read(include_bytes!("../../../testdata/sbase_annotations.xml")).unwrap()
}

#[test]
fn document_and_nested_model_terms_keep_authored_order() {
    let document = annotated();
    assert_eq!(document.element.cvterms[0].qualifier, "BQM_IS");
    let model = document.model.unwrap();
    // SBO annotations are a report concern, not fabricated by the input reader.
    assert_eq!(model.element.cvterms.len(), 2);
    let described = &model.element.cvterms[0];
    assert_eq!(described.qualifier, "BQM_IS_DESCRIBED_BY");
    assert_eq!(
        described.resources,
        [
            "https://identifiers.org/pubmed/31219795",
            "https://identifiers.org/pubmed/00000001",
        ]
    );
    let taxon = &model.element.cvterms[1];
    assert_eq!(taxon.qualifier, "BQB_HAS_TAXON");
    assert_eq!(taxon.nested[0].qualifier, "BQB_IS_DESCRIBED_BY");
    assert_eq!(
        taxon.nested[0].resources,
        ["https://identifiers.org/pubmed/31219795"]
    );
    assert!(taxon.nested[0].nested.is_empty());
}

#[test]
fn creators_optional_fields_and_date_offsets_survive_native_document_drop() {
    let element = annotated().model.unwrap().element;
    let history = element.history.unwrap();
    assert_eq!(
        history.created_date.as_deref(),
        Some("2018-01-02T03:04:05+02:00")
    );
    assert_eq!(
        history.modified_dates,
        ["2019-02-03T04:05:06Z", "2020-03-04T05:06:07Z"]
    );
    assert_eq!(history.creators[0].family_name.as_deref(), Some("König"));
    assert_eq!(
        history.creators[0].organization.as_deref(),
        Some("Biology Institute")
    );
    assert_eq!(
        history.creators[0].email.as_deref(),
        Some("model@example.org")
    );
    assert_eq!(history.creators[1].given_name.as_deref(), Some("Marie"));
    assert!(history.creators[1].organization.is_none());
    assert!(history.creators[1].email.is_none());
}

#[test]
fn rdf_subject_and_prefix_handling_matches_the_native_reader() {
    let model = annotated().model.unwrap();
    let parameters = &model.parameters;
    assert!(parameters[0].element.cvterms.is_empty());
    assert!(parameters.iter().all(|p| p.element.history.is_none()));
    assert_eq!(
        parameters[1].element.cvterms[0].qualifier,
        "BQB_HAS_PROPERTY"
    );
    assert_eq!(
        parameters[2].element.cvterms[0].nested[0].resources,
        ["https://identifiers.org/SBO:0000002"]
    );
    assert!(parameters[3].element.cvterms.is_empty());
    assert!(parameters[4].element.cvterms.is_empty());
}
