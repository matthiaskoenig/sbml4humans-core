//! Native libSBML validation and the reference reader of the report tests.
//!
//! [`ValidationSession`] owns the admitted documents, preserves diagnostics and
//! source keys, and installs a closed comp resolver only while validating.
//! Native hosts enable it explicitly; portable report and WASM builds do not
//! link libSBML. The reference reader fills [`sbml4humans_model`] for parity
//! tests and is not used by the production Rust report pipeline.
//!
//! [`read`] answers a [`ReadError`] for every source which is no document and
//! for every document which holds a math nested deeper than
//! [`sbml4humans_model::MAX_AST_DEPTH`], [`read_math`] one for every string
//! which is no content MathML and
//! [`read_formula`] one for every string which is no formula of a Level 1 math
//! (Level 2 dropped the `formula` attribute), and all three hold one lock of
//! the process while they read,
//! since libsbml keeps global state. None of them panics, whatever the source
//! holds.

mod annotations;
mod fill;
mod math;
mod strings;
mod sys;
mod validation;

pub use fill::{ReadError, read};
pub use math::{read_formula, read_math};
pub use validation::{
    ElementPositions, ValidationContent, ValidationError, ValidationIssue, ValidationRule,
    ValidationSession, ValidationSource, rule_package, validation_rule,
};

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
