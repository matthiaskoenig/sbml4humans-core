//! Reading an SBML document with libsbml into the owned model of the report.
//!
//! The crate is temporary. It fills the model of [`sbml4humans_model`] through
//! libsbml, so that the reader of our own has a report to be held against, and
//! it is deleted once the two agree on every example. It is therefore a
//! correct, explicit and small layer over libsbml and no general binding of
//! the library: it reaches libsbml through `shim.cpp`, a C layer over the C++
//! API which exposes what the filler reads and nothing else.
//!
//! [`read`] answers a [`ReadError`] for every source which is no document, and
//! holds one lock of the process while it reads, since libsbml keeps global
//! state. It never panics, whatever the source holds.

mod fill;
mod strings;
mod sys;

pub use fill::{ReadError, read};
