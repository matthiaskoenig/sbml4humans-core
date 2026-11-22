//! A double of SBML, which may be infinite or not a number.

use serde::{Serialize, Serializer};

/// A double which serializes a finite value as a number and the three other values as strings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Double(
    /// The value.
    pub f64,
);

impl Serialize for Double {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let v = self.0;
        if v.is_nan() {
            serializer.serialize_str("NaN")
        } else if v == f64::INFINITY {
            serializer.serialize_str("Infinity")
        } else if v == f64::NEG_INFINITY {
            serializer.serialize_str("-Infinity")
        } else {
            serializer.serialize_f64(v)
        }
    }
}
