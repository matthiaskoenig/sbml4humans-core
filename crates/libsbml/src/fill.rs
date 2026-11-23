//! Filling the document model with the getters of libsbml.
//!
//! Nothing here may end a read of a source a request brought: nothing indexes
//! with a number of the document, nothing unwraps what the document holds, and
//! an element libsbml does not answer is left out rather than assumed.

use std::ffi::{CString, c_double, c_int, c_uint};
use std::sync::{Mutex, MutexGuard};

use sbml4humans_model::{Compartment, Document, Element, Model, Package, Parameter, Species};

use crate::strings;
use crate::sys;

/// Why a source is no document the report can be built of.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    /// A document which libsbml read without errors holds no model, with the log of the read.
    #[error("the document holds no model: {0}")]
    NoModel(String),
    /// libsbml could not read the source.
    #[error("libsbml could not read the source: {0}")]
    Libsbml(String),
}

/// The lock every read of libsbml holds.
///
/// libsbml keeps global state, the registry of its packages and the resolver of
/// the external model definitions a later slice installs among it, so one read
/// of the process runs at a time.
static LIBSBML: Mutex<()> = Mutex::new(());

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
            model: Some(model(sbase)),
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
/// # Safety
///
/// `sbase` is the model of a document of libsbml which is alive.
unsafe fn model(sbase: *mut sys::SBase) -> Model {
    unsafe {
        Model {
            element: element(sbase),
            compartments: elements(
                sbase,
                sys::s4h_model_num_compartments,
                sys::s4h_model_compartment,
                compartment,
            ),
            species: elements(
                sbase,
                sys::s4h_model_num_species,
                sys::s4h_model_species,
                species,
            ),
            parameters: elements(
                sbase,
                sys::s4h_model_num_parameters,
                sys::s4h_model_parameter,
                parameter,
            ),
        }
    }
}

/// The elements of one list of the model, in the order of the document.
///
/// An element libsbml does not answer is left out, so that the number it
/// states of its list cannot make the filler ask for one which is not there.
///
/// # Safety
///
/// `model` is the model of a document of libsbml which is alive, and `count`,
/// `at` and `fill` belong to one list of the model.
unsafe fn elements<T>(
    model: *mut sys::SBase,
    count: unsafe extern "C" fn(*mut sys::SBase) -> c_uint,
    at: unsafe extern "C" fn(*mut sys::SBase, c_uint) -> *mut sys::SBase,
    fill: unsafe fn(*mut sys::SBase) -> T,
) -> Vec<T> {
    let mut elements = Vec::new();
    for n in 0..unsafe { count(model) } {
        let sbase = unsafe { at(model, n) };
        if !sbase.is_null() {
            elements.push(unsafe { fill(sbase) });
        }
    }
    elements
}

/// A compartment of the model.
///
/// # Safety
///
/// `sbase` is a compartment of a document of libsbml which is alive.
unsafe fn compartment(sbase: *mut sys::SBase) -> Compartment {
    unsafe {
        Compartment {
            element: element_with_xml(sbase),
            spatial_dimensions: number(sbase, sys::s4h_compartment_spatial_dimensions),
            size: number(sbase, sys::s4h_compartment_size),
            constant: flag(sbase, sys::s4h_compartment_constant),
            units: strings::borrowed(sys::s4h_compartment_units(sbase)),
        }
    }
}

/// A species of the model.
///
/// # Safety
///
/// `sbase` is a species of a document of libsbml which is alive.
unsafe fn species(sbase: *mut sys::SBase) -> Species {
    unsafe {
        Species {
            element: element_with_xml(sbase),
            compartment: strings::borrowed(sys::s4h_species_compartment(sbase)).unwrap_or_default(),
            initial_amount: number(sbase, sys::s4h_species_initial_amount),
            initial_concentration: number(sbase, sys::s4h_species_initial_concentration),
            substance_units: strings::borrowed(sys::s4h_species_substance_units(sbase)),
            has_only_substance_units: flag(sbase, sys::s4h_species_has_only_substance_units),
            boundary_condition: flag(sbase, sys::s4h_species_boundary_condition),
            constant: flag(sbase, sys::s4h_species_constant),
        }
    }
}

/// A parameter of the model.
///
/// # Safety
///
/// `sbase` is a parameter of a document of libsbml which is alive.
unsafe fn parameter(sbase: *mut sys::SBase) -> Parameter {
    unsafe {
        Parameter {
            element: element_with_xml(sbase),
            value: number(sbase, sys::s4h_parameter_value),
            constant: flag(sbase, sys::s4h_parameter_constant),
            units: strings::borrowed(sys::s4h_parameter_units(sbase)),
        }
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
