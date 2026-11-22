//! The fields every SBML object shares.

/// The fields every SBML object shares.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Element {
    /// The `id` attribute.
    pub id: Option<String>,
    /// The `metaid` attribute.
    pub meta_id: Option<String>,
    /// The `name` attribute.
    pub name: Option<String>,
    /// The `sboTerm` attribute, as the id of the term.
    pub sbo: Option<String>,
    /// The text of the `notes` element.
    pub notes: Option<String>,
    /// The text of the `annotation` element.
    pub annotation: Option<String>,
    /// The element serialized back to XML.
    pub xml: Option<String>,
    /// The line where the element starts in the source.
    pub line: u32,
    /// The column where the element starts in the source.
    pub column: u32,
}
