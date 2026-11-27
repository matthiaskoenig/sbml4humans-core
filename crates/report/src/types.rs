//! The types of the report, with the field names and the optionality of `model.py` of sbml4humans.

use serde::Serialize;

use crate::Double;

/// The math of an element, as latex and as a Level 3 formula string.
///
/// Both fields are what the python implementation writes for the same syntax
/// tree, which `crate::math` ports: `math` of that module writes one.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Math {
    /// The `latex`, `null` for the two maths the python implementation reports
    /// without one, which `crate::math::math` names.
    ///
    /// The key stays in the JSON either way, which is what python writes: the
    /// frontend reads the field and shows the formula alone where it is `null`.
    pub latex: Option<String>,
    /// The `formula`, which is written for any tree at any depth.
    pub formula: String,
}

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
    pub nodes: indexmap::IndexMap<String, LinkNode>,
    /// The `edges`.
    pub edges: Vec<LinkEdge>,
}

/// One SBML object of the report, with the same key as its element.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkNode {
    /// The primary key of the object.
    pub pk: String,
    /// The SBML class of the object.
    pub sbml_type: &'static str,
    /// Its authored identifier, without a fabricated identifier for unnamed objects.
    pub id: Option<String>,
    /// Its authored name.
    pub name: Option<String>,
    /// The key of its containing model, absent for documents and models themselves.
    pub model: Option<String>,
}

/// A relation between two objects, using the wire names of the Python report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeKind {
    /// The model listed by the document.
    Model,
    /// The compartment of a species or reaction.
    Compartment,
    /// A reactant of a reaction, or the species of its reference.
    Reactant,
    /// A product of a reaction, or the species of its reference.
    Product,
    /// A modifier of a reaction, or the species of its reference.
    Modifier,
    /// The kinetic law of a reaction.
    KineticLaw,
    /// A parameter local to a kinetic law.
    LocalParameter,
    /// The trigger of an event.
    Trigger,
    /// The priority of an event.
    Priority,
    /// The delay of an event.
    Delay,
    /// An assignment of an event.
    EventAssignment,
    /// The variable set by a rule or an event assignment.
    Variable,
    /// The symbol set by an initial assignment.
    Symbol,
    /// A unit definition named by an attribute or a number in a math.
    Units,
    /// The parameter named as a conversion factor.
    ConversionFactor,
    /// A symbol read by a math, including a call of a model function.
    Math,
}

/// One directed relation, local to the report unless its target names another entry.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkEdge {
    /// The key of the object which holds the reference.
    pub source: String,
    /// The key of the object the reference names.
    pub target: String,
    /// What the reference means.
    pub kind: EdgeKind,
    /// The archive entry of a non-local target; absent for core references.
    pub target_entry: Option<String>,
}

/// Timing information of a report (`Debug` in python).
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugInfo {
    /// The time of the report.
    pub json_report_time: String,
}

/// The attributes every SBML object shares.
///
/// Every `SBase` is built through [`SBase::new`], which takes the two fields no element of a report
/// may be without, so neither can be forgotten: `Default::default()` leaves `sbml_type` the empty
/// string, which would serialize as an `sbmlType` of no type at all. The `Default` of the struct is
/// there for the fields which follow those two, as the tail of a `..SBase::new(...)`, and for the
/// two types which flatten an `SBase` into a default of their own ([`Uncertainty`], [`ListOf`]); it
/// is no value to serialize on its own.
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

impl SBase {
    /// The shared fields of an element of `sbml_type`, keyed by `pk`, and nothing else set.
    ///
    /// It is the one constructor of an `SBase`: the type of an element is a field of the report the
    /// frontend reads of every object, so it is taken here rather than left to a `Default` which
    /// would write no type at all.
    pub fn new(pk: String, sbml_type: &'static str) -> Self {
        Self {
            pk,
            sbml_type,
            ..Self::default()
        }
    }
}

/// A function definition with its lambda.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportFunctionDefinition {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `math`, `null` when the document does not set it.
    pub math: Option<Math>,
}

/// One factor of a unit definition: a base unit with exponent, scale and multiplier.
///
/// A unit carries no identifier and nothing in SBML refers to it, so it is a
/// nested object of its definition and no element of the report with a primary
/// key of its own.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportUnit {
    /// The `kind`, `null` when the document does not set it.
    pub kind: Option<String>,
    /// The `exponent`, a double since Level 3 allows a fractional one, `null` when the document does not set it.
    pub exponent: Option<Double>,
    /// The `scale`, `null` when the document does not set it.
    pub scale: Option<i64>,
    /// The `multiplier`, `null` when the document does not set it.
    pub multiplier: Option<Double>,
}

/// A unit definition with its units and their rendered formula.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportUnitDefinition {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `units_latex`, `null` when the document does not set it.
    pub units_latex: Option<String>,
    /// The `list_of_units`, in the order of the document.
    pub list_of_units: Vec<ReportUnit>,
}

/// An initial assignment of a symbol.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportInitialAssignment {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The id of the element the assignment sets.
    pub symbol: String,
    /// The `math`, `null` when the document does not set it.
    pub math: Option<Math>,
    /// The `derived_units`, `null` when the document does not set it.
    pub derived_units: Option<String>,
}

/// A rule of one of the three rule classes of SBML.
///
/// Python has a class per class of rule and the report tells them apart by the
/// `sbmlType` of the element, which is the one field of the three classes which
/// differs in its value. Two of them declare a `variable` and the algebraic rule,
/// which sets none, declares none, so the field is left out of its object
/// altogether and is no `variable: null`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRule {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The id of the element the rule sets, `None` for an algebraic rule, whose class declares none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<String>,
    /// The `math`, `null` when the document does not set it.
    pub math: Option<Math>,
    /// The `derived_units`, `null` when the document does not set it.
    pub derived_units: Option<String>,
}

/// A constraint with its message.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportConstraint {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `math`, `null` when the document does not set it.
    pub math: Option<Math>,
    /// The `message`, `null` when the document does not set it.
    pub message: Option<String>,
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
    /// The id of the compartment, the empty string where libsbml answers nothing.
    ///
    /// Python declares the field a required string (`compartment: str`, `model.py:363`) and writes
    /// whatever `getCompartment()` hands it, which is the empty string for a species of an invalid
    /// document which names no compartment, so the empty string is the faithful value and no
    /// fabricated one. The attribute is required of a species (core §4.6), so no valid document
    /// reaches it.
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

/// A reactant or a product of a reaction.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSpeciesReference {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The id of the species the reference names.
    pub species: String,
    /// The `stoichiometry`, `null` when neither the document nor its level sets it.
    pub stoichiometry: Option<Double>,
    /// The `constant`, a Level 3 attribute, `null` when the document does not set it.
    pub constant: Option<bool>,
}

/// A modifier of a reaction, which names a species and nothing else.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportModifierSpeciesReference {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The id of the species the reference names.
    pub species: String,
}

/// A parameter of a kinetic law, local to its reaction.
///
/// A Level 3 law writes them as `localParameter` and a Level 1 or Level 2 law as `parameter`, where
/// libsbml answers a `Parameter`; both carry the id, the value and the units the report shows, so
/// python has one class for either and types both a `LocalParameter`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportLocalParameter {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `value`, `null` when the document does not set it.
    pub value: Option<Double>,
    /// The `units`, `null` when the document does not set it.
    pub units: Option<String>,
    /// The `units_latex`, `null` when the document does not set it.
    pub units_latex: Option<String>,
    /// The `derived_units`, `null` when the document does not set it.
    pub derived_units: Option<String>,
}

/// The kinetic law of a reaction with its local parameters.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportKineticLaw {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `math`, `null` when the document does not set it.
    pub math: Option<Math>,
    /// The `derived_units`, `null` when the document does not set it.
    pub derived_units: Option<String>,
    /// The `list_of_local_parameters`, in the order of the document.
    pub list_of_local_parameters: Vec<ReportLocalParameter>,
}

/// A reaction with its participants, its kinetic law and its readable equation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportReaction {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `reversible`, `null` when neither the document nor its level sets it.
    pub reversible: Option<bool>,
    /// The `fast`, `null` when neither the document nor its level and version sets it.
    pub fast: Option<bool>,
    /// The `compartment`, a Level 3 attribute, `null` when the document does not set it.
    pub compartment: Option<String>,
    /// The `list_of_reactants`, in the order of the document.
    pub list_of_reactants: Vec<ReportSpeciesReference>,
    /// The `list_of_products`, in the order of the document.
    pub list_of_products: Vec<ReportSpeciesReference>,
    /// The `list_of_modifiers`, in the order of the document.
    pub list_of_modifiers: Vec<ReportModifierSpeciesReference>,
    /// The `kinetic_law`, `null` when the reaction has none.
    pub kinetic_law: Option<ReportKineticLaw>,
    /// The readable equation, the two half equations of the reaction and a plain unicode arrow.
    pub equation: String,
    /// The `fbc`, `null` when the document does not set it.
    pub fbc: Option<serde_json::Value>,
}

/// The trigger of an event: the condition which fires it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportTrigger {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `math`, `null` when the document does not set it.
    pub math: Option<Math>,
    /// The `initial_value`, a Level 3 attribute, `null` when the document does not set it.
    pub initial_value: Option<bool>,
    /// The `persistent`, a Level 3 attribute, `null` when the document does not set it.
    pub persistent: Option<bool>,
}

/// The priority of an event: the order of the events of one moment.
///
/// It carries a math and nothing else, as the delay below does, and is a type of its own because
/// python has a class per element type and the frontend reads the `sbmlType` of either.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPriority {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `math`, `null` when the document does not set it.
    pub math: Option<Math>,
}

/// The delay of an event: the time between the trigger and the execution.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDelay {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `math`, `null` when the document does not set it.
    pub math: Option<Math>,
}

/// One assignment an event makes when it fires.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportEventAssignment {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The id of the element the assignment sets.
    pub variable: String,
    /// The `math`, `null` when the document does not set it.
    pub math: Option<Math>,
}

/// An event with its trigger, its priority, its delay and its assignments.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportEvent {
    /// The attributes every SBML object shares.
    #[serde(flatten)]
    pub sbase: SBase,
    /// The `use_values_from_trigger_time`, `null` when the document does not set it.
    pub use_values_from_trigger_time: Option<bool>,
    /// The `trigger`, `null` when the event has none.
    pub trigger: Option<ReportTrigger>,
    /// The `priority`, a Level 3 element, `null` when the event has none.
    pub priority: Option<ReportPriority>,
    /// The `delay`, `null` when the event has none.
    pub delay: Option<ReportDelay>,
    /// The `list_of_event_assignments`, in the order of the document.
    pub list_of_event_assignments: Vec<ReportEventAssignment>,
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
    pub list_of_function_definitions: Vec<ReportFunctionDefinition>,
    /// The `list_of_unit_definitions`, in the order of `sbmlinfo.py`.
    pub list_of_unit_definitions: Vec<ReportUnitDefinition>,
    /// The `list_of_compartments`, in the order of `sbmlinfo.py`.
    pub list_of_compartments: Vec<ReportCompartment>,
    /// The `list_of_species`, in the order of `sbmlinfo.py`.
    pub list_of_species: Vec<ReportSpecies>,
    /// The `list_of_parameters`, in the order of `sbmlinfo.py`.
    pub list_of_parameters: Vec<ReportParameter>,
    /// The `list_of_initial_assignments`, in the order of `sbmlinfo.py`.
    pub list_of_initial_assignments: Vec<ReportInitialAssignment>,
    /// The `list_of_rules`, in the order of `sbmlinfo.py`.
    pub list_of_rules: Vec<ReportRule>,
    /// The `list_of_constraints`, in the order of `sbmlinfo.py`.
    pub list_of_constraints: Vec<ReportConstraint>,
    /// The `list_of_reactions`, in the order of `sbmlinfo.py`.
    pub list_of_reactions: Vec<ReportReaction>,
    /// The `list_of_events`, in the order of `sbmlinfo.py`.
    pub list_of_events: Vec<ReportEvent>,
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
    /// The `ode_system`, `null` when the equations of the model are not computed, which this slice never does.
    pub ode_system: Option<serde_json::Value>,
    /// The `ode_error`, the message of a failure to compute the equations, `null` when none failed.
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
