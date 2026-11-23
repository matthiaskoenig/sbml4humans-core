//! Filling the document model with the getters of libsbml.
//!
//! Nothing here may end a read of a source a request brought: nothing indexes
//! with a number of the document, nothing unwraps what the document holds, and
//! an element libsbml does not answer is left out rather than assumed.

use std::ffi::{CString, c_double, c_int, c_uint};
use std::sync::{Mutex, MutexGuard};

use sbml4humans_model::{
    Ast, Compartment, Constraint, Document, Element, Event, EventAssignment, FunctionDefinition,
    InitialAssignment, KineticLaw, LocalParameter, MathElement, Model, ModifierSpeciesReference,
    Package, Parameter, Reaction, Rule, RuleKind, Species, SpeciesReference, Trigger, Unit,
    UnitDefinition,
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
    /// An element of the document is of a class the filler has no kind of.
    ///
    /// libsbml read the source, and the filler refuses to guess the kind of a
    /// class it does not know, the way the python implementation refuses the
    /// report of one (`sbmlinfo.rule` raises `TypeError` for a rule which is
    /// neither an assignment, a rate nor an algebraic rule). The text names
    /// the element and what is wrong with it.
    #[error("the document holds {0}")]
    ElementUnknownKind(String),
    /// The syntax tree of a math holds a node of a type the report has no kind of.
    ///
    /// No document of libsbml 5.21.2 reaches this: the shim names every type of
    /// `ASTNodeType_t` a document can carry, the `AST_UNKNOWN` of a math it
    /// could not read included. It is the guard against a libsbml which
    /// declares a type the shim does not name, whose node is refused here rather
    /// than handed to the writers of the report, which would have to guess the
    /// kind of a node they do not know.
    #[error("the math holds a node of a type the report has no kind of")]
    MathUnknownKind,
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
            reactions: elements(
                sbase,
                sys::s4h_model_num_reactions,
                sys::s4h_model_reaction,
                reaction,
            )?,
            events: elements(
                sbase,
                sys::s4h_model_num_events,
                sys::s4h_model_event,
                event,
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
/// The `compartment` is a required attribute of a species, so it is a plain string and the empty
/// one where libsbml answers nothing, which is the value python reports for such a species as
/// well; the field of `ReportSpecies` says so with the line of `model.py` which requires it.
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
                return Err(ReadError::ElementUnknownKind(
                    "a rule which is neither an assignment, a rate nor an algebraic rule"
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

/// A reaction of the model with its participants and its kinetic law.
///
/// `reversible` and `fast` are what the document sets and not what libsbml
/// answers of one which sets neither: libsbml answers `true` of the
/// `reversible` of a reaction of every level, which is the default of Level 1
/// and Level 2 and no value at all in Level 3, and the report writes the
/// default of the level itself (`sbmlinfo._reversible`, `sbmlinfo._fast`).
///
/// # Safety
///
/// `sbase` is a reaction of a document of libsbml which is alive, and the
/// caller holds the lock of libsbml.
unsafe fn reaction(sbase: *mut sys::SBase) -> Result<Reaction, ReadError> {
    unsafe {
        Ok(Reaction {
            element: element_with_xml(sbase),
            reversible: flag(sbase, sys::s4h_reaction_reversible),
            fast: flag(sbase, sys::s4h_reaction_fast),
            compartment: strings::borrowed(sys::s4h_reaction_compartment(sbase)),
            reactants: elements(
                sbase,
                sys::s4h_reaction_num_reactants,
                sys::s4h_reaction_reactant,
                species_reference,
            )?,
            products: elements(
                sbase,
                sys::s4h_reaction_num_products,
                sys::s4h_reaction_product,
                species_reference,
            )?,
            modifiers: elements(
                sbase,
                sys::s4h_reaction_num_modifiers,
                sys::s4h_reaction_modifier,
                modifier_species_reference,
            )?,
            kinetic_law: child(sbase, sys::s4h_reaction_kinetic_law, kinetic_law)?,
        })
    }
}

/// A reactant or a product of a reaction.
///
/// The stoichiometry is the raw answer of libsbml with the two predicates of
/// it, from which the report derives the value it shows: libsbml answers the
/// default of the level of a reference which sets nothing, so nothing else may
/// write a stoichiometry of its own (`sbmlinfo._stoichiometry`). The species is
/// read unguarded, as the python implementation reads it: it is the empty
/// string where the document names nothing, which the model keeps as one.
///
/// # Safety
///
/// `sbase` is a reactant or a product of a document of libsbml which is alive.
unsafe fn species_reference(sbase: *mut sys::SBase) -> Result<SpeciesReference, ReadError> {
    unsafe {
        Ok(SpeciesReference {
            element: element_with_xml(sbase),
            species: strings::borrowed(sys::s4h_species_reference_species(sbase))
                .unwrap_or_default(),
            // the shim writes the stoichiometry of every reference, so the
            // NaN stands for an element which is no reference at all
            stoichiometry: number(sbase, sys::s4h_species_reference_stoichiometry)
                .unwrap_or(f64::NAN),
            stoichiometry_set: sys::s4h_species_reference_stoichiometry_set(sbase) != 0,
            stoichiometry_math: sys::s4h_species_reference_stoichiometry_math(sbase) != 0,
            constant: flag(sbase, sys::s4h_species_reference_constant),
        })
    }
}

/// A modifier of a reaction, which names a species and nothing else.
///
/// # Safety
///
/// `sbase` is a modifier of a document of libsbml which is alive.
unsafe fn modifier_species_reference(
    sbase: *mut sys::SBase,
) -> Result<ModifierSpeciesReference, ReadError> {
    unsafe {
        Ok(ModifierSpeciesReference {
            element: element_with_xml(sbase),
            species: strings::borrowed(sys::s4h_species_reference_species(sbase))
                .unwrap_or_default(),
        })
    }
}

/// The kinetic law of a reaction with its local parameters.
///
/// # Safety
///
/// `sbase` is a kinetic law of a document of libsbml which is alive, and the
/// caller holds the lock of libsbml.
unsafe fn kinetic_law(sbase: *mut sys::SBase) -> Result<KineticLaw, ReadError> {
    unsafe {
        Ok(KineticLaw {
            element: element_with_xml(sbase),
            math: math(sbase, sys::s4h_kinetic_law_math)?,
            local_parameters: elements(
                sbase,
                sys::s4h_kinetic_law_num_parameters,
                sys::s4h_kinetic_law_parameter,
                local_parameter,
            )?,
        })
    }
}

/// A parameter of a kinetic law, local to its reaction.
///
/// The shim answers the class of the level, a `LocalParameter` of Level 3 and a
/// `Parameter` of Level 1 and Level 2, and the getters read either, so the
/// model has one type for both (`sbmlinfo._kinetic_law_parameters`).
///
/// # Safety
///
/// `sbase` is a parameter of a kinetic law of a document of libsbml which is
/// alive.
unsafe fn local_parameter(sbase: *mut sys::SBase) -> Result<LocalParameter, ReadError> {
    unsafe {
        Ok(LocalParameter {
            element: element_with_xml(sbase),
            value: number(sbase, sys::s4h_local_parameter_value),
            units: strings::borrowed(sys::s4h_local_parameter_units(sbase)),
        })
    }
}

/// An event of the model with its trigger, priority, delay and assignments.
///
/// # Safety
///
/// `sbase` is an event of a document of libsbml which is alive, and the caller
/// holds the lock of libsbml.
unsafe fn event(sbase: *mut sys::SBase) -> Result<Event, ReadError> {
    unsafe {
        Ok(Event {
            element: element_with_xml(sbase),
            use_values_from_trigger_time: flag(sbase, sys::s4h_event_use_values_from_trigger_time),
            trigger: child(sbase, sys::s4h_event_trigger, trigger)?,
            priority: child(sbase, sys::s4h_event_priority, math_element)?,
            delay: child(sbase, sys::s4h_event_delay, math_element)?,
            event_assignments: elements(
                sbase,
                sys::s4h_event_num_event_assignments,
                sys::s4h_event_event_assignment,
                event_assignment,
            )?,
        })
    }
}

/// The trigger of an event.
///
/// # Safety
///
/// `sbase` is a trigger of a document of libsbml which is alive, and the caller
/// holds the lock of libsbml.
unsafe fn trigger(sbase: *mut sys::SBase) -> Result<Trigger, ReadError> {
    unsafe {
        Ok(Trigger {
            element: element_with_xml(sbase),
            math: math(sbase, sys::s4h_trigger_math)?,
            initial_value: flag(sbase, sys::s4h_trigger_initial_value),
            persistent: flag(sbase, sys::s4h_trigger_persistent),
        })
    }
}

/// The priority or the delay of an event, whose one field is a math.
///
/// # Safety
///
/// `sbase` is a priority or a delay of a document of libsbml which is alive,
/// and the caller holds the lock of libsbml.
unsafe fn math_element(sbase: *mut sys::SBase) -> Result<MathElement, ReadError> {
    unsafe {
        Ok(MathElement {
            element: element_with_xml(sbase),
            math: math(sbase, sys::s4h_math_element_math)?,
        })
    }
}

/// One assignment of an event.
///
/// The variable is read unguarded, as the python implementation reads it: it is
/// the empty string where the document names nothing.
///
/// # Safety
///
/// `sbase` is an event assignment of a document of libsbml which is alive, and
/// the caller holds the lock of libsbml.
unsafe fn event_assignment(sbase: *mut sys::SBase) -> Result<EventAssignment, ReadError> {
    unsafe {
        Ok(EventAssignment {
            element: element_with_id_attribute(sbase),
            variable: strings::borrowed(sys::s4h_event_assignment_variable(sbase))
                .unwrap_or_default(),
            math: math(sbase, sys::s4h_event_assignment_math)?,
        })
    }
}

/// The one child of an element of a class, `None` when it has none.
///
/// # Safety
///
/// `parent` is an element of a document of libsbml which is alive, the caller
/// holds the lock of libsbml, and `of` and `fill` belong to one child of the
/// class of that element.
unsafe fn child<T>(
    parent: *mut sys::SBase,
    of: unsafe extern "C" fn(*mut sys::SBase) -> *mut sys::SBase,
    fill: unsafe fn(*mut sys::SBase) -> Result<T, ReadError>,
) -> Result<Option<T>, ReadError> {
    let sbase = unsafe { of(parent) };
    if sbase.is_null() {
        return Ok(None);
    }
    unsafe { fill(sbase) }.map(Some)
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
    of: unsafe extern "C" fn(*mut sys::SBase) -> *const sys::ASTNode,
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
