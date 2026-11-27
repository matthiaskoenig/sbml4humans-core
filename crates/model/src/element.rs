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
    /// The controlled annotations parsed from the RDF of this element.
    pub cvterms: Vec<CvTerm>,
    /// The creators and dates parsed from the RDF of this element.
    pub history: Option<History>,
    /// The element serialized back to XML.
    pub xml: Option<String>,
    /// The line where the element starts in the source.
    pub line: u32,
    /// The column where the element starts in the source.
    pub column: u32,
}

/// One controlled annotation, including annotations of that relation itself.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CvTerm {
    /// The wire name of the model or biological qualifier.
    pub qualifier: String,
    /// Resource URIs in their authored order.
    pub resources: Vec<String>,
    /// Qualifiers of this relation, in their authored order.
    pub nested: Vec<CvTerm>,
}

/// A creator named in the history of an element.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Creator {
    /// The given name, absent when it is not stated.
    pub given_name: Option<String>,
    /// The family name, absent when it is not stated.
    pub family_name: Option<String>,
    /// The organization, absent when it is not stated.
    pub organization: Option<String>,
    /// The email address, absent when it is not stated.
    pub email: Option<String>,
}

/// The history of an element, with dates normalized by the reader as SBML defines them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct History {
    /// The creators, in their authored order.
    pub creators: Vec<Creator>,
    /// The creation date, absent when none is stated.
    pub created_date: Option<String>,
    /// The modification dates, in their authored order.
    pub modified_dates: Vec<String>,
}
