//! The types of the report, with the field names and the optionality of `model.py` of sbml4humans.

use serde::Serialize;
use serde_json::Value;

use crate::Double;

/// An annotation: a qualifier with its resources (`CVTerm`).
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CVTerm {
    /// The qualifier.
    pub qualifier: String,
    /// The `resources`.
    pub resources: Vec<String>,
    /// The `nested`.
    pub nested: Vec<CVTerm>,
}

/// A creator of the model history.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    /// The `given_name`, `null` when the document does not set it.
    pub given_name: Option<String>,
    /// The `family_name`, `null` when the document does not set it.
    pub family_name: Option<String>,
    /// The `organization`, `null` when the document does not set it.
    pub organization: Option<String>,
    /// The `email`, `null` when the document does not set it.
    pub email: Option<String>,
}

/// The history of an element: creators and dates.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelHistory {
    /// The `creators`.
    pub creators: Vec<Creator>,
    /// The `created_date`, `null` when the document does not set it.
    pub created_date: Option<String>,
    /// The `modified_dates`.
    pub modified_dates: Vec<String>,
}

/// The comp extension of an element.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompSBase {
    /// The `replaced_by`, `null` when the document does not set it.
    pub replaced_by: Option<serde_json::Value>,
    /// The `replaced_elements`.
    pub replaced_elements: Vec<serde_json::Value>,
}

/// A distrib uncertainty of an element.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Uncertainty {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `uncert_parameters`.
    pub uncert_parameters: Vec<serde_json::Value>,
}

/// One entry of the controlled annotation of fbc.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValuePair {
    /// The `key`, `null` when the document does not set it.
    pub key: Option<String>,
    /// The `value`, `null` when the document does not set it.
    pub value: Option<String>,
    /// The `uri`, `null` when the document does not set it.
    pub uri: Option<String>,
}

/// A ListOf container of an element which carries something of its own.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOf {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The name the list has in the file.
    pub element: String,
    /// The number of elements of the list.
    pub size: i64,
}

/// The conversion factor parameter of a model or species.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionFactor {
    /// The id of the parameter.
    pub sid: String,
    /// The `value`, `null` when the document does not set it.
    pub value: Option<Double>,
    /// The `units`, `null` when the document does not set it.
    pub units: Option<String>,
}

/// An SBML package used by the document.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    /// The prefix.
    pub prefix: String,
    /// The version.
    pub version: i64,
}

/// All objects of a report and the references between them.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkGraph {
    /// The nodes by pk.
    pub nodes: std::collections::BTreeMap<String, Value>,
    /// The `edges`.
    pub edges: Vec<serde_json::Value>,
}

/// Timing information of a report (`Debug` in python).
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugInfo {
    /// The time of the report.
    pub json_report_time: String,
}

/// The attributes every SBML object shares.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SBase {
    /// The primary key.
    pub pk: String,
    /// The type name of the element.
    pub sbml_type: &'static str,
    /// The `id`, `null` when the document does not set it.
    pub id: Option<String>,
    /// The `meta_id`, `null` when the document does not set it.
    pub meta_id: Option<String>,
    /// The `name`, `null` when the document does not set it.
    pub name: Option<String>,
    /// The `sbo`, `null` when the document does not set it.
    pub sbo: Option<String>,
    /// The `notes`, `null` when the document does not set it.
    pub notes: Option<String>,
    /// The `cvterms`.
    pub cvterms: Vec<CVTerm>,
    /// The `history`, `null` when the document does not set it.
    pub history: Option<ModelHistory>,
    /// The `xml`, `null` when the document does not set it.
    pub xml: Option<String>,
    /// The `comp`, `null` when the document does not set it.
    pub comp: Option<CompSBase>,
    /// The `uncertainties`.
    pub uncertainties: Vec<Uncertainty>,
    /// The `key_value_pairs`.
    pub key_value_pairs: Vec<KeyValuePair>,
    /// The `lists`.
    pub lists: Vec<ListOf>,
}

/// A compartment.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportCompartment {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `spatial_dimensions`, `null` when the document does not set it.
    pub spatial_dimensions: Option<Double>,
    /// The `size`, `null` when the document does not set it.
    pub size: Option<Double>,
    /// The `constant`, `null` when the document does not set it.
    pub constant: Option<bool>,
    /// The `units`, `null` when the document does not set it.
    pub units: Option<String>,
    /// The `units_latex`, `null` when the document does not set it.
    pub units_latex: Option<String>,
    /// The `derived_units`, `null` when the document does not set it.
    pub derived_units: Option<String>,
}

/// A species.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSpecies {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The id of the compartment.
    pub compartment: String,
    /// The `initial_amount`, `null` when the document does not set it.
    pub initial_amount: Option<Double>,
    /// The `initial_concentration`, `null` when the document does not set it.
    pub initial_concentration: Option<Double>,
    /// The `substance_units`, `null` when the document does not set it.
    pub substance_units: Option<String>,
    /// The `has_only_substance_units`, `null` when the document does not set it.
    pub has_only_substance_units: Option<bool>,
    /// The `boundary_condition`, `null` when the document does not set it.
    pub boundary_condition: Option<bool>,
    /// The `constant`, `null` when the document does not set it.
    pub constant: Option<bool>,
    /// The `units_latex`, `null` when the document does not set it.
    pub units_latex: Option<String>,
    /// The `derived_units`, `null` when the document does not set it.
    pub derived_units: Option<String>,
    /// The `conversion_factor`, `null` when the document does not set it.
    pub conversion_factor: Option<ConversionFactor>,
    /// The `fbc`, `null` when the document does not set it.
    pub fbc: Option<serde_json::Value>,
}

/// A global parameter.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportParameter {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `value`, `null` when the document does not set it.
    pub value: Option<Double>,
    /// The `constant`, `null` when the document does not set it.
    pub constant: Option<bool>,
    /// The `units`, `null` when the document does not set it.
    pub units: Option<String>,
    /// The `units_latex`, `null` when the document does not set it.
    pub units_latex: Option<String>,
    /// The `derived_units`, `null` when the document does not set it.
    pub derived_units: Option<String>,
}

/// A model or comp model definition with the lists of its elements.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportModel {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// `model` or `modelDefinition`.
    pub kind: &'static str,
    /// The `annotation_xml`, `null` when the document does not set it.
    pub annotation_xml: Option<String>,
    /// The `substance_units`, `null` when the document does not set it.
    pub substance_units: Option<String>,
    /// The `substance_units_latex`, `null` when the document does not set it.
    pub substance_units_latex: Option<String>,
    /// The `time_units`, `null` when the document does not set it.
    pub time_units: Option<String>,
    /// The `time_units_latex`, `null` when the document does not set it.
    pub time_units_latex: Option<String>,
    /// The `volume_units`, `null` when the document does not set it.
    pub volume_units: Option<String>,
    /// The `volume_units_latex`, `null` when the document does not set it.
    pub volume_units_latex: Option<String>,
    /// The `area_units`, `null` when the document does not set it.
    pub area_units: Option<String>,
    /// The `area_units_latex`, `null` when the document does not set it.
    pub area_units_latex: Option<String>,
    /// The `length_units`, `null` when the document does not set it.
    pub length_units: Option<String>,
    /// The `length_units_latex`, `null` when the document does not set it.
    pub length_units_latex: Option<String>,
    /// The `extent_units`, `null` when the document does not set it.
    pub extent_units: Option<String>,
    /// The `extent_units_latex`, `null` when the document does not set it.
    pub extent_units_latex: Option<String>,
    /// The `conversion_factor`, `null` when the document does not set it.
    pub conversion_factor: Option<ConversionFactor>,
    /// The `list_of_function_definitions`, in the order of `sbmlinfo.py`.
    pub list_of_function_definitions: Vec<serde_json::Value>,
    /// The `list_of_unit_definitions`, in the order of `sbmlinfo.py`.
    pub list_of_unit_definitions: Vec<serde_json::Value>,
    /// The `list_of_compartments`, in the order of `sbmlinfo.py`.
    pub list_of_compartments: Vec<ReportCompartment>,
    /// The `list_of_species`, in the order of `sbmlinfo.py`.
    pub list_of_species: Vec<ReportSpecies>,
    /// The `list_of_parameters`, in the order of `sbmlinfo.py`.
    pub list_of_parameters: Vec<ReportParameter>,
    /// The `list_of_initial_assignments`, in the order of `sbmlinfo.py`.
    pub list_of_initial_assignments: Vec<serde_json::Value>,
    /// The `list_of_rules`, in the order of `sbmlinfo.py`.
    pub list_of_rules: Vec<serde_json::Value>,
    /// The `list_of_constraints`, in the order of `sbmlinfo.py`.
    pub list_of_constraints: Vec<serde_json::Value>,
    /// The `list_of_reactions`, in the order of `sbmlinfo.py`.
    pub list_of_reactions: Vec<serde_json::Value>,
    /// The `list_of_events`, in the order of `sbmlinfo.py`.
    pub list_of_events: Vec<serde_json::Value>,
    /// The `list_of_submodels`, in the order of `sbmlinfo.py`.
    pub list_of_submodels: Vec<serde_json::Value>,
    /// The `list_of_ports`, in the order of `sbmlinfo.py`.
    pub list_of_ports: Vec<serde_json::Value>,
    /// The `list_of_gene_products`, in the order of `sbmlinfo.py`.
    pub list_of_gene_products: Vec<serde_json::Value>,
    /// The `list_of_objectives`, in the order of `sbmlinfo.py`.
    pub list_of_objectives: Vec<serde_json::Value>,
    /// The `list_of_flux_bounds`, in the order of `sbmlinfo.py`.
    pub list_of_flux_bounds: Vec<serde_json::Value>,
    /// The `list_of_user_defined_constraints`, in the order of `sbmlinfo.py`.
    pub list_of_user_defined_constraints: Vec<serde_json::Value>,
    /// The `list_of_qualitative_species`, in the order of `sbmlinfo.py`.
    pub list_of_qualitative_species: Vec<serde_json::Value>,
    /// The `list_of_transitions`, in the order of `sbmlinfo.py`.
    pub list_of_transitions: Vec<serde_json::Value>,
    /// The `fbc`, `null` when the document does not set it.
    pub fbc: Option<serde_json::Value>,
}

/// The document: level, version and the packages it uses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDocument {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The SBML level.
    pub level: i64,
    /// The SBML version.
    pub version: i64,
    /// The `packages`.
    pub packages: Vec<Package>,
    /// The `annotation_xml`, `null` when the document does not set it.
    pub annotation_xml: Option<String>,
}

/// The report of one SBML document.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// The document.
    pub document: ReportDocument,
    /// The `models`.
    pub models: Vec<ReportModel>,
    /// The `external_model_definitions`.
    pub external_model_definitions: Vec<serde_json::Value>,
    /// The link graph.
    pub link_graph: LinkGraph,
    /// The `ode_system`, `null` when the document does not set it.
    pub ode_system: Option<serde_json::Value>,
    /// The `ode_error`, `null` when the document does not set it.
    pub ode_error: Option<String>,
}

/// The report of one SBML entry of an archive.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportEntry {
    /// The report.
    pub report: Report,
    /// The timing information.
    pub debug: DebugInfo,
}
