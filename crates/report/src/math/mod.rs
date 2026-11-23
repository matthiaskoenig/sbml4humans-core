//! The writers of the math of an element.
//!
//! [`formula`] writes the Level 3 formula of a syntax tree, the `formula` field
//! of the math of the report. Both fields of a math are what the python
//! implementation writes for the same document, which the gate of
//! `crates/oracles` compares entry by entry against the whole corpus.
//!
//! A writer reads the `name` of a node for a symbol and for a call of a
//! function of the model only, and keys everything else off the kind of the
//! node: libsbml answers its own built-in name for every other node, so a node
//! of the kind [`AstKind::FunctionAbs`] carries the name `abs` and one of the
//! kind [`AstKind::ConstantPi`] carries `pi`, and a writer which fell back to
//! the name would be right on those and wrong wherever libsbml names a node
//! differently from the way a formula or a latex writes it.
//!
//! No writer panics on any tree, whatever a document holds: this crate is built
//! for WebAssembly, where a panic traps the instance and takes the report of
//! every other document of the process with it.

mod formula;

pub use formula::formula;

use sbml4humans_model::{AstKind, MAX_AST_DEPTH};

/// Why the math of an element could not be written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MathError {
    /// The syntax tree is nested deeper than [`MAX_AST_DEPTH`].
    #[error("the math is nested deeper than {MAX_AST_DEPTH} levels")]
    TooDeep,
    /// The tree holds a node of a kind no writer knows, which only
    /// [`AstKind::Other`] is: the kind the libsbml shim answers for a type it
    /// does not map, a node of a package this crate does not write.
    #[error("the math holds a node of the kind {0:?}, which no writer writes")]
    UnknownKind(AstKind),
    /// A node lacks a child a writer of its kind reads.
    ///
    /// Every writer asks for a child of a node whose shape it has established,
    /// so a tree the libsbml shim filled never answers this; it is here so that
    /// no writer indexes into the children of a node and panics instead.
    #[error("a node of the math lacks a child its writer reads")]
    MissingChild,
}
