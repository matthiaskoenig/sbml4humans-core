//! The serde types of the report of sbml4humans.
//!
//! The JSON these types produce is the JSON of the python backend, field for field.

mod double;
mod types;

pub use double::Double;
pub use types::*;
