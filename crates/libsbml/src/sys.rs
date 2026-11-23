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
/// The root of a tree of `s4h_read_math_from_string` is owned by whoever read
/// it, every other node by its parent.
#[repr(C)]
pub(crate) struct ASTNode {
    _opaque: [u8; 0],
}

/// The package of an SBML Level 3 extension a document declares.
#[repr(C)]
pub(crate) struct Package {
    /// The prefix of the namespace of the package, null when there is none.
    pub(crate) prefix: *const c_char,
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

    pub(crate) fn s4h_sbase_id(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_sbase_meta_id(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_sbase_name(sbase: *mut SBase) -> *const c_char;
    pub(crate) fn s4h_sbase_sbo_term_id(sbase: *mut SBase) -> *mut c_char;
    pub(crate) fn s4h_sbase_notes(sbase: *mut SBase) -> *mut c_char;
    pub(crate) fn s4h_sbase_annotation(sbase: *mut SBase) -> *mut c_char;
    pub(crate) fn s4h_sbase_xml(sbase: *mut SBase) -> *mut c_char;
    pub(crate) fn s4h_sbase_line(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_sbase_column(sbase: *mut SBase) -> c_uint;

    pub(crate) fn s4h_model_num_compartments(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_compartment(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_model_num_species(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_species(sbase: *mut SBase, n: c_uint) -> *mut SBase;
    pub(crate) fn s4h_model_num_parameters(sbase: *mut SBase) -> c_uint;
    pub(crate) fn s4h_model_parameter(sbase: *mut SBase, n: c_uint) -> *mut SBase;

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

    pub(crate) fn s4h_read_math_from_string(xml: *const c_char) -> *mut ASTNode;
    pub(crate) fn s4h_ast_free(node: *mut ASTNode);
    pub(crate) fn s4h_ast_kind(node: *mut ASTNode) -> c_int;
    pub(crate) fn s4h_ast_num_children(node: *mut ASTNode) -> c_uint;
    pub(crate) fn s4h_ast_child(node: *mut ASTNode, n: c_uint) -> *mut ASTNode;
    pub(crate) fn s4h_ast_name(node: *mut ASTNode) -> *const c_char;
    pub(crate) fn s4h_ast_number(
        node: *mut ASTNode,
        integer: *mut c_longlong,
        real: *mut c_double,
        exponent: *mut c_longlong,
        denominator: *mut c_longlong,
    ) -> c_int;
    pub(crate) fn s4h_ast_units(node: *mut ASTNode) -> *mut c_char;
}
