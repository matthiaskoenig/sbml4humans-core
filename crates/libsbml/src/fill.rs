//! Filling the document model with the getters of libsbml.
//!
//! Nothing here may end a read of a source a request brought: nothing indexes
//! with a number of the document, nothing unwraps what the document holds, and
//! an element libsbml does not answer is left out rather than assumed.

use std::ffi::{CString, c_double, c_int, c_uint};
use std::sync::{Mutex, MutexGuard};

use sbml4humans_model::{
    Ast, Compartment, Constraint, Document, Element, FunctionDefinition, InitialAssignment, Model,
    Package, Parameter, Rule, RuleKind, Species, Unit, UnitDefinition,
};

use crate::math::ast_of;
use crate::strings;
use crate::sys;

/// Why a source is no document, or no math, the report can be built of.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    /// A document which libsbml read without errors holds no model, with the log of the read.
    #[error("the document holds no model: {0}")]
    NoModel(String),
    /// libsbml could not read the source.
    #[error("libsbml could not read the source: {0}")]
    Libsbml(String),
    /// The syntax tree of a math is nested deeper than [`sbml4humans_model::MAX_AST_DEPTH`].
    #[error(
        "the math is nested deeper than {} levels",
        sbml4humans_model::MAX_AST_DEPTH
    )]
    MathTooDeep,
}

/// The lock every read of libsbml holds.
///
/// libsbml keeps global state, the registry of its packages and the resolver of
/// the external model definitions a later slice installs among it, so one read
/// of the process runs at a time.
static LIBSBML: Mutex<()> = Mutex::new(());

/// The code `s4h_rule_kind` answers of an assignment rule.
const RULE_ASSIGNMENT: c_int = 0;
/// The code `s4h_rule_kind` answers of a rate rule.
const RULE_RATE: c_int = 1;
/// The code `s4h_rule_kind` answers of an algebraic rule.
const RULE_ALGEBRAIC: c_int = 2;

/// Holds the lock of libsbml, also when a reader before it panicked.
///
/// A poisoned lock says that a read ended in a panic, which says nothing about
/// the document of the next one, so the lock is taken over rather than an error.
pub(crate) fn locked() -> MutexGuard<'static, ()> {
    LIBSBML
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A document of libsbml, freed when the read of it ends.
struct Reading(*mut sys::SBMLDocument);

impl Drop for Reading {
    fn drop(&mut self) {
        unsafe { sys::s4h_document_free(self.0) };
    }
}

/// The document of a source of SBML, as libsbml reads it.
///
/// The source is read as one XML document; neither a gzipped document nor an
/// archive is one, and neither are the bytes of anything else.
pub fn read(source: &[u8]) -> Result<Document, ReadError> {
    let xml = CString::new(source).map_err(|_| {
        ReadError::Libsbml("the source holds a nul byte, which no XML document holds".to_owned())
    })?;
    let _lock = locked();
    let reading = Reading(unsafe { sys::s4h_read(xml.as_ptr()) });
    if reading.0.is_null() {
        return Err(ReadError::Libsbml(
            "libsbml read no document of the source".to_owned(),
        ));
    }
    unsafe { document(reading.0) }
}

/// The document and its model.
///
/// # Safety
///
/// `doc` is a document of libsbml which is alive.
unsafe fn document(doc: *mut sys::SBMLDocument) -> Result<Document, ReadError> {
    unsafe {
        if sys::s4h_document_out_of_memory(doc) != 0 {
            return Err(ReadError::Libsbml(error_log(doc)));
        }
        let sbase = sys::s4h_document_model(doc);
        if sbase.is_null() {
            let log = error_log(doc);
            // a source libsbml logged errors of is no document, whatever it holds
            return Err(if sys::s4h_document_num_errors(doc) > 0 {
                ReadError::Libsbml(log)
            } else {
                ReadError::NoModel(log)
            });
        }
        Ok(Document {
            level: sys::s4h_document_level(doc),
            version: sys::s4h_document_version(doc),
            packages: packages(doc),
            // the serialized XML of the document is the whole source and the
            // one of the model nearly all of it, so neither carries it
            element: element(sys::s4h_document_sbase(doc)),
            model: Some(model(sbase)?),
        })
    }
}

/// The errors libsbml logged of a read, in its own words.
///
/// # Safety
///
/// `doc` is a document of libsbml which is alive.
unsafe fn error_log(doc: *mut sys::SBMLDocument) -> String {
    let log = unsafe { strings::handed_over(sys::s4h_document_error_log(doc)) }.unwrap_or_default();
    let log = log.trim_end();
    if log.is_empty() {
        "libsbml logged no error of the read".to_owned()
    } else {
        log.to_owned()
    }
}

/// The packages a document declares, in the order of the document.
///
/// # Safety
///
/// `doc` is a document of libsbml which is alive.
unsafe fn packages(doc: *mut sys::SBMLDocument) -> Vec<Package> {
    let mut packages = Vec::new();
    for n in 0..unsafe { sys::s4h_document_num_plugins(doc) } {
        let package = unsafe { sys::s4h_document_package(doc, n) };
        if let Some(prefix) = unsafe { strings::borrowed(package.prefix) } {
            packages.push(Package {
                prefix,
                version: package.version,
                required: package.required != 0,
            });
        }
    }
    packages
}

/// The model of a document with its elements.
///
/// The lists are filled in the order the python implementation builds them, so
/// that the two can be read side by side.
///
/// # Safety
///
/// `sbase` is the model of a document of libsbml which is alive, and the
/// caller holds the lock of libsbml.
unsafe fn model(sbase: *mut sys::SBase) -> Result<Model, ReadError> {
    unsafe {
        Ok(Model {
            element: element(sbase),
            function_definitions: elements(
                sbase,
                sys::s4h_model_num_function_definitions,
                sys::s4h_model_function_definition,
                function_definition,
            )?,
            unit_definitions: elements(
                sbase,
                sys::s4h_model_num_unit_definitions,
                sys::s4h_model_unit_definition,
                unit_definition,
            )?,
            compartments: elements(
                sbase,
                sys::s4h_model_num_compartments,
                sys::s4h_model_compartment,
                compartment,
            )?,
            species: elements(
                sbase,
                sys::s4h_model_num_species,
                sys::s4h_model_species,
                species,
            )?,
            parameters: elements(
                sbase,
                sys::s4h_model_num_parameters,
                sys::s4h_model_parameter,
                parameter,
            )?,
            initial_assignments: elements(
                sbase,
                sys::s4h_model_num_initial_assignments,
                sys::s4h_model_initial_assignment,
                initial_assignment,
            )?,
            rules: elements(sbase, sys::s4h_model_num_rules, sys::s4h_model_rule, rule)?,
            constraints: elements(
                sbase,
                sys::s4h_model_num_constraints,
                sys::s4h_model_constraint,
                constraint,
            )?,
        })
    }
}

/// The elements of one list of an element, in the order of the document.
///
/// An element libsbml does not answer is left out, so that the number it
/// states of its list cannot make the filler ask for one which is not there.
/// A fill which refuses the document, the one of a math which is nested too
/// deep, ends the read of the whole document.
///
/// # Safety
///
/// `parent` is an element of a document of libsbml which is alive, the caller
/// holds the lock of libsbml, and `count`, `at` and `fill` belong to one list
/// of the class of that element.
unsafe fn elements<T>(
    parent: *mut sys::SBase,
    count: unsafe extern "C" fn(*mut sys::SBase) -> c_uint,
    at: unsafe extern "C" fn(*mut sys::SBase, c_uint) -> *mut sys::SBase,
    fill: unsafe fn(*mut sys::SBase) -> Result<T, ReadError>,
) -> Result<Vec<T>, ReadError> {
    let mut elements = Vec::new();
    for n in 0..unsafe { count(parent) } {
        let sbase = unsafe { at(parent, n) };
        if !sbase.is_null() {
            elements.push(unsafe { fill(sbase) }?);
        }
    }
    Ok(elements)
}

/// A function definition of the model.
///
/// # Safety
///
/// `sbase` is a function definition of a document of libsbml which is alive,
/// and the caller holds the lock of libsbml.
unsafe fn function_definition(sbase: *mut sys::SBase) -> Result<FunctionDefinition, ReadError> {
    unsafe {
        Ok(FunctionDefinition {
            element: element_with_xml(sbase),
            math: math(sbase, sys::s4h_function_definition_math)?,
        })
    }
}

/// A unit definition of the model with its units.
///
/// # Safety
///
/// `sbase` is a unit definition of a document of libsbml which is alive, and
/// the caller holds the lock of libsbml.
unsafe fn unit_definition(sbase: *mut sys::SBase) -> Result<UnitDefinition, ReadError> {
    unsafe {
        Ok(UnitDefinition {
            element: element_with_xml(sbase),
            units: elements(
                sbase,
                sys::s4h_unit_definition_num_units,
                sys::s4h_unit_definition_unit,
                unit,
            )?,
        })
    }
}

/// One unit of a unit definition.
///
/// A unit carries no identifier and nothing refers to it, so it carries none
/// of the shared fields of an element.
///
/// # Safety
///
/// `sbase` is a unit of a document of libsbml which is alive.
unsafe fn unit(sbase: *mut sys::SBase) -> Result<Unit, ReadError> {
    unsafe {
        Ok(Unit {
            kind: strings::borrowed(sys::s4h_unit_kind(sbase)),
            exponent: number(sbase, sys::s4h_unit_exponent),
            scale: integer(sbase, sys::s4h_unit_scale),
            multiplier: number(sbase, sys::s4h_unit_multiplier),
        })
    }
}

/// A compartment of the model.
///
/// # Safety
///
/// `sbase` is a compartment of a document of libsbml which is alive.
unsafe fn compartment(sbase: *mut sys::SBase) -> Result<Compartment, ReadError> {
    unsafe {
        Ok(Compartment {
            element: element_with_xml(sbase),
            spatial_dimensions: number(sbase, sys::s4h_compartment_spatial_dimensions),
            size: number(sbase, sys::s4h_compartment_size),
            constant: flag(sbase, sys::s4h_compartment_constant),
            units: strings::borrowed(sys::s4h_compartment_units(sbase)),
        })
    }
}

/// A species of the model.
///
/// # Safety
///
/// `sbase` is a species of a document of libsbml which is alive.
unsafe fn species(sbase: *mut sys::SBase) -> Result<Species, ReadError> {
    unsafe {
        Ok(Species {
            element: element_with_xml(sbase),
            compartment: strings::borrowed(sys::s4h_species_compartment(sbase)).unwrap_or_default(),
            initial_amount: number(sbase, sys::s4h_species_initial_amount),
            initial_concentration: number(sbase, sys::s4h_species_initial_concentration),
            substance_units: strings::borrowed(sys::s4h_species_substance_units(sbase)),
            has_only_substance_units: flag(sbase, sys::s4h_species_has_only_substance_units),
            boundary_condition: flag(sbase, sys::s4h_species_boundary_condition),
            constant: flag(sbase, sys::s4h_species_constant),
        })
    }
}

/// A parameter of the model.
///
/// # Safety
///
/// `sbase` is a parameter of a document of libsbml which is alive.
unsafe fn parameter(sbase: *mut sys::SBase) -> Result<Parameter, ReadError> {
    unsafe {
        Ok(Parameter {
            element: element_with_xml(sbase),
            value: number(sbase, sys::s4h_parameter_value),
            constant: flag(sbase, sys::s4h_parameter_constant),
            units: strings::borrowed(sys::s4h_parameter_units(sbase)),
        })
    }
}

/// An initial assignment of the model.
///
/// The symbol is read unguarded, as the python implementation reads it: it is
/// the empty string where the document names nothing.
///
/// # Safety
///
/// `sbase` is an initial assignment of a document of libsbml which is alive,
/// and the caller holds the lock of libsbml.
unsafe fn initial_assignment(sbase: *mut sys::SBase) -> Result<InitialAssignment, ReadError> {
    unsafe {
        Ok(InitialAssignment {
            element: element_with_id_attribute(sbase),
            symbol: strings::borrowed(sys::s4h_initial_assignment_symbol(sbase))
                .unwrap_or_default(),
            math: math(sbase, sys::s4h_initial_assignment_math)?,
        })
    }
}

/// A rule of the model.
///
/// A rule of a class which is neither an assignment, a rate nor an algebraic
/// rule ends the read, as it ends the build of the report of the python
/// implementation (`sbmlinfo.rule` raises `TypeError`): the three are the
/// rules of SBML, and a fourth is nothing the filler may guess the kind of.
///
/// # Safety
///
/// `sbase` is a rule of a document of libsbml which is alive, and the caller
/// holds the lock of libsbml.
unsafe fn rule(sbase: *mut sys::SBase) -> Result<Rule, ReadError> {
    unsafe {
        let kind = match sys::s4h_rule_kind(sbase) {
            RULE_ASSIGNMENT => RuleKind::Assignment,
            RULE_RATE => RuleKind::Rate,
            RULE_ALGEBRAIC => RuleKind::Algebraic,
            _ => {
                return Err(ReadError::Libsbml(
                    "the model holds a rule which is neither an assignment, a rate nor an \
                     algebraic rule"
                        .to_owned(),
                ));
            }
        };
        Ok(Rule {
            element: element_with_id_attribute(sbase),
            kind,
            // an algebraic rule sets no variable, and libsbml answers the
            // empty string of one, which says the same
            variable: match kind {
                RuleKind::Algebraic => None,
                _ => strings::borrowed(sys::s4h_rule_variable(sbase)),
            },
            math: math(sbase, sys::s4h_rule_math)?,
        })
    }
}

/// A constraint of the model.
///
/// # Safety
///
/// `sbase` is a constraint of a document of libsbml which is alive, and the
/// caller holds the lock of libsbml.
unsafe fn constraint(sbase: *mut sys::SBase) -> Result<Constraint, ReadError> {
    unsafe {
        Ok(Constraint {
            element: element_with_xml(sbase),
            math: math(sbase, sys::s4h_constraint_math)?,
            message: strings::handed_over(sys::s4h_constraint_message(sbase)),
        })
    }
}

/// The fields every element shares, without the element serialized back to XML.
///
/// # Safety
///
/// `sbase` is an element of a document of libsbml which is alive.
unsafe fn element(sbase: *mut sys::SBase) -> Element {
    unsafe {
        Element {
            id: strings::borrowed(sys::s4h_sbase_id(sbase)),
            meta_id: strings::borrowed(sys::s4h_sbase_meta_id(sbase)),
            name: strings::borrowed(sys::s4h_sbase_name(sbase)),
            sbo: strings::handed_over(sys::s4h_sbase_sbo_term_id(sbase)),
            notes: strings::handed_over(sys::s4h_sbase_notes(sbase)),
            annotation: strings::handed_over(sys::s4h_sbase_annotation(sbase)),
            xml: None,
            line: sys::s4h_sbase_line(sbase),
            column: sys::s4h_sbase_column(sbase),
        }
    }
}

/// The fields every element shares and the element serialized back to XML.
///
/// # Safety
///
/// `sbase` is an element of a document of libsbml which is alive.
unsafe fn element_with_xml(sbase: *mut sys::SBase) -> Element {
    let mut filled = unsafe { element(sbase) };
    filled.xml = unsafe { strings::handed_over(sys::s4h_sbase_xml(sbase)) };
    filled
}

/// The shared fields of an element whose id libsbml aliases to its target.
///
/// libsbml answers `getId()` of an initial assignment, of a rule and of an
/// event assignment as the attribute which names what they set, so the `id` of
/// those three is the `id` attribute of the document, which is the id the
/// python implementation carries of them as well (`sbmlinfo._identifier`).
///
/// # Safety
///
/// `sbase` is an initial assignment, a rule or an event assignment of a
/// document of libsbml which is alive.
unsafe fn element_with_id_attribute(sbase: *mut sys::SBase) -> Element {
    let mut filled = unsafe { element_with_xml(sbase) };
    filled.id = unsafe { strings::borrowed(sys::s4h_sbase_id_attribute(sbase)) };
    filled
}

/// The syntax tree of the math of an element, `None` when it has none.
///
/// The tree belongs to the element and is read, never freed here. A tree which
/// is nested deeper than [`sbml4humans_model::MAX_AST_DEPTH`] ends the read of
/// the document, so that the reader of a math stays the one place which
/// refuses one.
///
/// # Safety
///
/// `sbase` is an element of a document of libsbml which is alive, the caller
/// holds the lock of libsbml, and `of` is a function of the shim of the math
/// of its class.
unsafe fn math(
    sbase: *mut sys::SBase,
    of: unsafe extern "C" fn(*mut sys::SBase) -> *mut sys::ASTNode,
) -> Result<Option<Ast>, ReadError> {
    let node = unsafe { of(sbase) };
    if node.is_null() {
        return Ok(None);
    }
    unsafe { ast_of(node, 1) }.map(Some)
}

/// A number attribute, `None` when the document does not set it.
///
/// # Safety
///
/// `sbase` is an element of a document of libsbml which is alive, and `of` is
/// a function of the shim of a number attribute of its class.
unsafe fn number(
    sbase: *mut sys::SBase,
    of: unsafe extern "C" fn(*mut sys::SBase, *mut c_double) -> c_int,
) -> Option<f64> {
    let mut value = 0.0;
    (unsafe { of(sbase, &mut value) } != 0).then_some(value)
}

/// An integer attribute, `None` when the document does not set it.
///
/// # Safety
///
/// `sbase` is an element of a document of libsbml which is alive, and `of` is
/// a function of the shim of an integer attribute of its class.
unsafe fn integer(
    sbase: *mut sys::SBase,
    of: unsafe extern "C" fn(*mut sys::SBase, *mut c_int) -> c_int,
) -> Option<i32> {
    let mut value = 0;
    (unsafe { of(sbase, &mut value) } != 0).then_some(value)
}

/// A boolean attribute, `None` when the document does not set it.
///
/// # Safety
///
/// `sbase` is an element of a document of libsbml which is alive, and `of` is
/// a function of the shim of a boolean attribute of its class.
unsafe fn flag(
    sbase: *mut sys::SBase,
    of: unsafe extern "C" fn(*mut sys::SBase, *mut c_int) -> c_int,
) -> Option<bool> {
    let mut value = 0;
    (unsafe { of(sbase, &mut value) } != 0).then_some(value != 0)
}
