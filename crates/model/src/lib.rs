//! The owned data model of an SBML document.
//!
//! Plain structs which a reader fills and the report builder reads. Every list
//! keeps the order of the document, so the position of an element is its index
//! in its list.

mod document;
mod element;
mod math;

pub use document::{
    Compartment, Constraint, Document, FunctionDefinition, InitialAssignment, Model, Package,
    Parameter, Rule, RuleKind, Species, Unit, UnitDefinition,
};
pub use element::Element;
pub use math::{Ast, AstKind, AstNumber, MAX_AST_DEPTH};
