//! Maps the document model onto the report.

use sbml4humans_model::{Document, Element, Model};

use crate::{
    Double, LinkGraph, Package, Report, ReportCompartment, ReportDocument, ReportModel,
    ReportParameter, ReportSpecies, SBase, pk,
};

/// The derived units python writes when no unit is derived, deriving a unit belongs to a later slice.
///
/// The builder writes it for every element, which is wrong for a document which sets units at all.
/// It is the one value of the report this slice fabricates, and the oracle gate declares it as such.
const NO_DERIVED_UNITS: &str = "-";

/// The scope and the key python gives the document, the one element keyed by a fixed name.
const DOCUMENT_KEY: &str = "document";

/// The key python gives a model which carries neither an id nor a metaId (`MAIN_MODEL_KEY`).
const MAIN_MODEL_KEY: &str = "model";

/// The shared fields of an element, keyed in `scope` under `key`.
fn sbase(scope: &str, sbml_type: &'static str, element: &Element, key: &str) -> SBase {
    SBase {
        pk: pk::of(scope, sbml_type, key),
        sbml_type,
        id: element.id.clone(),
        meta_id: element.meta_id.clone(),
        name: element.name.clone(),
        sbo: element.sbo.clone(),
        notes: element.notes.clone(),
        xml: element.xml.clone(),
        ..SBase::default()
    }
}

/// The key of an element of a list, its id, else its metaId, else its position in that list.
fn key_in_list(element: &Element, index: usize) -> String {
    pk::key(element, || index.to_string())
}

fn model(model: &Model) -> ReportModel {
    // the key of the model is the scope of every element below it
    let scope = pk::key(&model.element, || MAIN_MODEL_KEY.to_owned());
    let compartments = model
        .compartments
        .iter()
        .enumerate()
        .map(|(i, c)| ReportCompartment {
            sbase: sbase(
                &scope,
                "Compartment",
                &c.element,
                &key_in_list(&c.element, i),
            ),
            spatial_dimensions: c.spatial_dimensions.map(Double),
            size: c.size.map(Double),
            constant: c.constant,
            units: c.units.clone(),
            units_latex: None,
            derived_units: Some(NO_DERIVED_UNITS.to_owned()),
        });
    let species = model
        .species
        .iter()
        .enumerate()
        .map(|(i, s)| ReportSpecies {
            sbase: sbase(&scope, "Species", &s.element, &key_in_list(&s.element, i)),
            compartment: s.compartment.clone(),
            initial_amount: s.initial_amount.map(Double),
            initial_concentration: s.initial_concentration.map(Double),
            substance_units: s.substance_units.clone(),
            has_only_substance_units: s.has_only_substance_units,
            boundary_condition: s.boundary_condition,
            constant: s.constant,
            units_latex: None,
            derived_units: Some(NO_DERIVED_UNITS.to_owned()),
            conversion_factor: None,
            fbc: None,
        });
    let parameters = model
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| ReportParameter {
            sbase: sbase(&scope, "Parameter", &p.element, &key_in_list(&p.element, i)),
            value: p.value.map(Double),
            constant: p.constant,
            units: p.units.clone(),
            units_latex: None,
            derived_units: Some(NO_DERIVED_UNITS.to_owned()),
        });
    ReportModel {
        // the model is keyed by the key which is its own scope, python writes `<key>/Model:<key>`
        sbase: sbase(&scope, "Model", &model.element, &scope),
        kind: "model",
        annotation_xml: model.element.annotation.clone(),
        substance_units: None,
        substance_units_latex: None,
        time_units: None,
        time_units_latex: None,
        volume_units: None,
        volume_units_latex: None,
        area_units: None,
        area_units_latex: None,
        length_units: None,
        length_units_latex: None,
        extent_units: None,
        extent_units_latex: None,
        conversion_factor: None,
        list_of_function_definitions: vec![],
        list_of_unit_definitions: vec![],
        list_of_compartments: compartments.collect(),
        list_of_species: species.collect(),
        list_of_parameters: parameters.collect(),
        list_of_initial_assignments: vec![],
        list_of_rules: vec![],
        list_of_constraints: vec![],
        list_of_reactions: vec![],
        list_of_events: vec![],
        list_of_submodels: vec![],
        list_of_ports: vec![],
        list_of_gene_products: vec![],
        list_of_objectives: vec![],
        list_of_flux_bounds: vec![],
        list_of_user_defined_constraints: vec![],
        list_of_qualitative_species: vec![],
        list_of_transitions: vec![],
        fbc: None,
    }
}

/// Builds the report of a document. The walk is a flat loop over the lists, it never recurses.
pub fn build(document: &Document) -> Report {
    Report {
        document: ReportDocument {
            // the document is the one element whose scope and key are both a fixed name,
            // python writes `document/SBMLDocument:document`
            sbase: sbase(
                DOCUMENT_KEY,
                "SBMLDocument",
                &document.element,
                DOCUMENT_KEY,
            ),
            level: i64::from(document.level),
            version: i64::from(document.version),
            packages: document
                .packages
                .iter()
                .map(|p| Package {
                    prefix: p.prefix.clone(),
                    version: i64::from(p.version),
                })
                .collect(),
            annotation_xml: document.element.annotation.clone(),
        },
        models: document.model.iter().map(model).collect(),
        external_model_definitions: vec![],
        link_graph: LinkGraph::default(),
        ode_system: None,
        ode_error: None,
    }
}
