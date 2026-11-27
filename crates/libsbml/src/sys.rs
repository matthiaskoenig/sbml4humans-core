//! The functions of `shim.cpp`, hand written against it.
//!
//! The declarations mirror the shim one for one, which is why they are written
//! rather than generated: the C API of libsbml is declared per class and
//! misses what the filler needs, so there is nothing to generate from.
//!
//! Every pointer of libsbml is an opaque type. It is handed back to libsbml as
//! it came and is never dereferenced, cast to another type of libsbml, counted
//! on or built in Rust.

use std::ffi::{c_char, c_double, c_int, c_longlong, c_uint};

/// An SBML document of libsbml, owned by whoever read it.
#[repr(C)]
pub(crate) struct SBMLDocument {
    _opaque: [u8; 0],
}

/// An element of a document of libsbml, owned by its document.
#[repr(C)]
pub(crate) struct SBase {
    _opaque: [u8; 0],
}

/// A node of a syntax tree of libsbml.
///
/// The root of a tree of `s4h_read_math_from_string` and of `s4h_read_formula`
/// is owned by whoever read it, every other node by its parent or by the
/// document of its element. A node which is owned is a `*mut ASTNode`, which
/// `s4h_ast_free` takes and nothing else does; a node which is borrowed, the
/// math of an element among them, is a `*const ASTNode`. Which of the two a
/// pointer is is therefore a question of its type.
#[repr(C)]
pub(crate) struct ASTNode {
    _opaque: [u8; 0],
}

/// A controlled annotation owned by its SBML document.
#[repr(C)]
pub(crate) struct CVTerm {
    _opaque: [u8; 0],
}
/// A model history owned by its SBML document.
#[repr(C)]
pub(crate) struct ModelHistory {
    _opaque: [u8; 0],
}
/// A creator owned by its model history.
#[repr(C)]
pub(crate) struct ModelCreator {
    _opaque: [u8; 0],
}

/// The package of an SBML Level 3 extension a document declares.
#[repr(C)]
pub(crate) struct Package {
    /// The prefix of the namespace of the package, null when there is none.
    pub(crate) prefix: *const c_char,
    /// The canonical package name.
    pub(crate) name: *const c_char,
    /// The version of the package.
    pub(crate) version: c_uint,
    /// Whether the document declares the package as required.
    pub(crate) required: c_int,
}

unsafe extern "C" {
    pub(crate) fn s4h_string_free(text: *mut c_char);

    pub(crate) fn s4h_read(xml: *const c_char) -> *mut SBMLDocument;
    pub(crate) fn s4h_registry_num_packages() -> c_uint;
    pub(crate) fn s4h_registry_package_name(n: c_uint) -> *mut c_char;

    pub(crate) fn s4h_document_free(doc: *mut SBMLDocument);
    pub(crate) fn s4h_document_level(doc: *mut SBMLDocument) -> c_uint;
    pub(crate) fn s4h_document_version(doc: *mut SBMLDocument) -> c_uint;
    pub(crate) fn s4h_document_sbase(doc: *mut SBMLDocument) -> *mut SBase;
    pub(crate) fn s4h_document_model(doc: *mut SBMLDocument) -> *mut SBase;
    pub(crate) fn s4h_document_error_log(doc: *mut SBMLDocument) -> *mut c_char;
    pub(crate) fn s4h_document_out_of_memory(doc: *mut SBMLDocument) -> c_int;
    pub(crate) fn s4h_document_num_errors(doc: *mut SBMLDocument) -> c_uint;
    pub(crate) fn s4h_document_num_plugins(doc: *mut SBMLDocument) -> c_uint;
    pub(crate) fn s4h_document_package(doc: *mut SBMLDocument, n: c_uint) -> Package;

    pub(crate) fn s4h_sbase_num_cvterms(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_sbase_cvterm(sbase: *mut SBase, n: c_uint) -> *mut CVTerm;
    pub(crate) fn s4h_cvterm_qualifier_type(term: *mut CVTerm) -> c_int;
    pub(crate) fn s4h_cvterm_qualifier(term: *mut CVTerm) -> *const c_char;
    pub(crate) fn s4h_cvterm_num_resources(term: *mut CVTerm) -> c_uint;
    pub(crate) fn s4h_cvterm_resource(term: *mut CVTerm, n: c_uint) -> *mut c_char;
    pub(crate) fn s4h_cvterm_num_nested(term: *mut CVTerm) -> c_uint;
    pub(crate) fn s4h_cvterm_nested(term: *mut CVTerm, n: c_uint) -> *mut CVTerm;
    pub(crate) fn s4h_sbase_history(sbase: *mut SBase) -> *mut ModelHistory;
    pub(crate) fn s4h_history_num_creators(history: *mut ModelHistory) -> c_uint;
    pub(crate) fn s4h_history_creator(history: *mut ModelHistory, n: c_uint) -> *mut ModelCreator;
    pub(crate) fn s4h_creator_given_name(creator: *mut ModelCreator) -> *const c_char;
    pub(crate) fn s4h_creator_family_name(creator: *mut ModelCreator) -> *const c_char;
    pub(crate) fn s4h_creator_organization(creator: *mut ModelCreator) -> *const c_char;
    pub(crate) fn s4h_creator_email(creator: *mut ModelCreator) -> *const c_char;
    pub(crate) fn s4h_history_created(history: *mut ModelHistory) -> *const c_char;
    pub(crate) fn s4h_history_num_modified(history: *mut ModelHistory) -> c_uint;
    pub(crate) fn s4h_history_modified(history: *mut ModelHistory, n: c_uint) -> *const c_char;

    pub(crate) fn s4h_sbase_id(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_sbase_id_attribute(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_sbase_meta_id(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_sbase_name(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_sbase_sbo_term_id(sbase: *mut SBase) -> *mut c_char;
    pub(crate) fn s4h_sbase_notes(sbase: *mut SBase) -> *mut c_char;
    pub(crate) fn s4h_sbase_annotation(sbase: *mut SBase) -> *mut c_char;
    pub(crate) fn s4h_sbase_xml(sbase: *mut SBase) -> *mut c_char;
    pub(crate) fn s4h_sbase_line(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_sbase_column(sbase: *mut SBase) -> c_uint;

    pub(crate) fn s4h_model_substance_units(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_model_time_units(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_model_volume_units(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_model_area_units(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_model_length_units(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_model_extent_units(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_conversion_factor(sbase: *mut SBase) -> *const c_char;

    pub(crate) fn s4h_model_num_function_definitions(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_function_definition(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_model_num_unit_definitions(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_unit_definition(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_model_num_compartments(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_compartment(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_model_num_species(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_species(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_model_num_parameters(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_parameter(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_model_num_initial_assignments(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_initial_assignment(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_model_num_rules(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_rule(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_model_num_constraints(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_constraint(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_model_num_reactions(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_reaction(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_model_num_events(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_event(sbase: *mut SBase, n: c_uint) -> *mut SBase;

    pub(crate) fn s4h_function_definition_math(sbase: *mut SBase) -> *const ASTNode;

    pub(crate) fn s4h_unit_definition_num_units(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_unit_definition_unit(sbase: *mut SBase, n: c_uint) -> *mut SBase;

    pub(crate) fn s4h_unit_kind(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_unit_exponent(sbase: *mut SBase, value: *mut c_double) -> c_int;
    pub(crate) fn s4h_unit_scale(sbase: *mut SBase, value: *mut c_int) -> c_int;
    pub(crate) fn s4h_unit_multiplier(sbase: *mut SBase, value: *mut c_double) -> c_int;

    pub(crate) fn s4h_compartment_spatial_dimensions(
        sbase: *mut SBase,
        value: *mut c_double,
    ) -> c_int;
    pub(crate) fn s4h_compartment_size(sbase: *mut SBase, value: *mut c_double) -> c_int;
    pub(crate) fn s4h_compartment_constant(sbase: *mut SBase, value: *mut c_int) -> c_int;
    pub(crate) fn s4h_compartment_units(sbase: *mut SBase) -> *const c_char;

    pub(crate) fn s4h_species_compartment(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_species_initial_amount(sbase: *mut SBase, value: *mut c_double) -> c_int;
    pub(crate) fn s4h_species_initial_concentration(
        sbase: *mut SBase,
        value: *mut c_double,
    ) -> c_int;
    pub(crate) fn s4h_species_spatial_size_units(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_species_substance_units(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_species_has_only_substance_units(
        sbase: *mut SBase,
        value: *mut c_int,
    ) -> c_int;
    pub(crate) fn s4h_species_boundary_condition(sbase: *mut SBase, value: *mut c_int) -> c_int;
    pub(crate) fn s4h_species_constant(sbase: *mut SBase, value: *mut c_int) -> c_int;

    pub(crate) fn s4h_parameter_value(sbase: *mut SBase, value: *mut c_double) -> c_int;
    pub(crate) fn s4h_parameter_constant(sbase: *mut SBase, value: *mut c_int) -> c_int;
    pub(crate) fn s4h_parameter_units(sbase: *mut SBase) -> *const c_char;

    pub(crate) fn s4h_initial_assignment_symbol(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_initial_assignment_math(sbase: *mut SBase) -> *const ASTNode;

    pub(crate) fn s4h_rule_kind(sbase: *mut SBase) -> c_int;
    pub(crate) fn s4h_rule_variable(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_rule_math(sbase: *mut SBase) -> *const ASTNode;

    pub(crate) fn s4h_constraint_math(sbase: *mut SBase) -> *const ASTNode;
    pub(crate) fn s4h_constraint_message(sbase: *mut SBase) -> *mut c_char;

    pub(crate) fn s4h_reaction_reversible(sbase: *mut SBase, value: *mut c_int) -> c_int;
    pub(crate) fn s4h_reaction_fast(sbase: *mut SBase, value: *mut c_int) -> c_int;
    pub(crate) fn s4h_reaction_compartment(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_reaction_num_reactants(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_reaction_reactant(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_reaction_num_products(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_reaction_product(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_reaction_num_modifiers(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_reaction_modifier(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_reaction_kinetic_law(sbase: *mut SBase) -> *mut SBase;

    pub(crate) fn s4h_species_reference_species(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_species_reference_stoichiometry(
        sbase: *mut SBase,
        value: *mut c_double,
    ) -> c_int;
    pub(crate) fn s4h_species_reference_stoichiometry_set(sbase: *mut SBase) -> c_int;
    pub(crate) fn s4h_species_reference_math(sbase: *mut SBase) -> *const ASTNode;
    pub(crate) fn s4h_species_reference_stoichiometry_math(sbase: *mut SBase) -> c_int;
    pub(crate) fn s4h_species_reference_constant(sbase: *mut SBase, value: *mut c_int) -> c_int;

    pub(crate) fn s4h_kinetic_law_math(sbase: *mut SBase) -> *const ASTNode;
    pub(crate) fn s4h_kinetic_law_num_parameters(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_kinetic_law_parameter(sbase: *mut SBase, n: c_uint) -> *mut SBase;

    pub(crate) fn s4h_local_parameter_value(sbase: *mut SBase, value: *mut c_double) -> c_int;
    pub(crate) fn s4h_local_parameter_units(sbase: *mut SBase) -> *const c_char;

    pub(crate) fn s4h_event_use_values_from_trigger_time(
        sbase: *mut SBase,
        value: *mut c_int,
    ) -> c_int;
    pub(crate) fn s4h_event_trigger(sbase: *mut SBase) -> *mut SBase;
    pub(crate) fn s4h_event_priority(sbase: *mut SBase) -> *mut SBase;
    pub(crate) fn s4h_event_delay(sbase: *mut SBase) -> *mut SBase;
    pub(crate) fn s4h_event_num_event_assignments(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_event_event_assignment(sbase: *mut SBase, n: c_uint) -> *mut SBase;

    pub(crate) fn s4h_trigger_math(sbase: *mut SBase) -> *const ASTNode;
    pub(crate) fn s4h_trigger_initial_value(sbase: *mut SBase, value: *mut c_int) -> c_int;
    pub(crate) fn s4h_trigger_persistent(sbase: *mut SBase, value: *mut c_int) -> c_int;

    pub(crate) fn s4h_math_element_math(sbase: *mut SBase) -> *const ASTNode;

    pub(crate) fn s4h_event_assignment_variable(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_event_assignment_math(sbase: *mut SBase) -> *const ASTNode;

    pub(crate) fn s4h_read_math_from_string(xml: *const c_char) -> *mut ASTNode;
    pub(crate) fn s4h_read_formula(formula: *const c_char) -> *mut ASTNode;
    pub(crate) fn s4h_ast_free(node: *mut ASTNode);
    pub(crate) fn s4h_ast_kind(node: *const ASTNode) -> c_int;
    pub(crate) fn s4h_ast_num_children(node: *const ASTNode) -> c_uint;
    pub(crate) fn s4h_ast_child(node: *const ASTNode, n: c_uint) -> *const ASTNode;
    pub(crate) fn s4h_ast_name(node: *const ASTNode) -> *const c_char;
    pub(crate) fn s4h_ast_number(
        node: *const ASTNode,
        integer: *mut c_longlong,
        real: *mut c_double,
        exponent: *mut c_longlong,
        denominator: *mut c_longlong,
    ) -> c_int;
    pub(crate) fn s4h_ast_units(node: *const ASTNode) -> *mut c_char;
}
