//! The owned data model of an SBML document.
//!
//! Plain structs which a reader fills and the report builder reads. Every list
//! keeps the order of the document, so the position of an element is its index
//! in its list.

mod document;
mod element;

pub use document::{Compartment, Document, Model, Package, Parameter, Species};
pub use element::Element;
