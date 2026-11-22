//! Maps the document model onto the report.

use sbml4humans_model::{Document, Element, Model};

use crate::{
    Double, LinkGraph, Package, Report, ReportCompartment, ReportDocument, ReportModel,
    ReportParameter, ReportSpecies, SBase, pk,
};

/// The derived units python writes when no unit is derived, deriving a unit belongs to a later slice.
const NO_DERIVED_UNITS: &str = "-";

/// The shared fields of an element, keyed in `scope` under the key of `index`.
fn sbase(scope: &str, sbml_type: &'static str, element: &Element, index: usize) -> SBase {
    SBase {
        pk: pk::of(scope, sbml_type, &pk::key(element, index)),
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

fn derived_units() -> Option<String> {
    Some(NO_DERIVED_UNITS.to_owned())
}

fn model(model: &Model, index: usize) -> ReportModel {
    let scope = pk::key(&model.element, index);
    let compartments = model
        .compartments
        .iter()
        .enumerate()
        .map(|(i, c)| ReportCompartment {
            sbase: sbase(&scope, "Compartment", &c.element, i),
            spatial_dimensions: c.spatial_dimensions.map(Double),
            size: c.size.map(Double),
            constant: c.constant,
            units: c.units.clone(),
            units_latex: None,
            derived_units: derived_units(),
        });
    let species = model
        .species
        .iter()
        .enumerate()
        .map(|(i, s)| ReportSpecies {
            sbase: sbase(&scope, "Species", &s.element, i),
            compartment: s.compartment.clone(),
            initial_amount: s.initial_amount.map(Double),
            initial_concentration: s.initial_concentration.map(Double),
            substance_units: s.substance_units.clone(),
            has_only_substance_units: s.has_only_substance_units,
            boundary_condition: s.boundary_condition,
            constant: s.constant,
            units_latex: None,
            derived_units: derived_units(),
            conversion_factor: None,
            fbc: None,
        });
    let parameters = model
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| ReportParameter {
            sbase: sbase(&scope, "Parameter", &p.element, i),
            value: p.value.map(Double),
            constant: p.constant,
            units: p.units.clone(),
            units_latex: None,
            derived_units: derived_units(),
        });
    ReportModel {
        sbase: sbase("document", "Model", &model.element, index),
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
    let mut doc_sbase = sbase("document", "SBMLDocument", &document.element, 0);
    // The document is the one element keyed by a fixed name, python writes `document/SBMLDocument:document`.
    doc_sbase.pk = pk::of("document", "SBMLDocument", "document");
    Report {
        document: ReportDocument {
            sbase: doc_sbase,
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
        models: document
            .model
            .iter()
            .enumerate()
            .map(|(i, m)| model(m, i))
            .collect(),
        external_model_definitions: vec![],
        link_graph: LinkGraph::default(),
        ode_system: None,
        ode_error: None,
    }
}
