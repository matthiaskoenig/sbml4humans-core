//! Reading an SBML document with libsbml into the owned model of the report.
//!
//! The crate is temporary. It fills the model of [`sbml4humans_model`] through
//! libsbml, so that the reader of our own has a report to be held against, and
//! it is deleted once the two agree on every example. It is therefore a
//! correct, explicit and small layer over libsbml and no general binding of
//! the library: it reaches libsbml through `shim.cpp`, a C layer over the C++
//! API which exposes what the filler reads and nothing else.
//!
//! [`read`] answers a [`ReadError`] for every source which is no document and
//! [`read_math`] one for every string which is no content MathML, and both
//! hold one lock of the process while they read, since libsbml keeps global
//! state. Neither panics, whatever the source holds.

mod fill;
mod math;
mod strings;
mod sys;

pub use fill::{ReadError, read};
pub use math::read_math;

/// The names of the packages the linked libsbml registers, sorted.
///
/// This is the package set of the library the crate was built against, which
/// the oracle manifest records so that a prefix with another set is noticed.
pub fn registered_packages() -> Vec<String> {
    let _lock = fill::locked();
    let count = unsafe { sys::s4h_registry_num_packages() };
    let mut names: Vec<String> = (0..count)
        .filter_map(|n| unsafe { strings::handed_over(sys::s4h_registry_package_name(n)) })
        .collect();
    names.sort();
    names
}
