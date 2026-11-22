//! The primary keys of the elements of a report.

use sbml4humans_model::Element;

/// The primary key of an element: `<scope>/<sbml_type>:<key>`.
pub fn of(scope: &str, sbml_type: &str, key: &str) -> String {
    format!("{scope}/{sbml_type}:{key}")
}

/// The key of an element within its scope: its id, else its metaId, else its index in the list of its parent.
///
/// The index is the rule of this slice and is not the rule of the python implementation. Python takes the id,
/// else the id of a class keyed by its target, else the metaId, else a key which the parent derived for the
/// child (the species of a species reference, with a `.1`, `.2` suffix for a repetition), else a digest of the
/// xml of the element. SBML requires an id on every element this slice builds, so the fallback is reachable
/// only on an invalid document. A later slice which adds the nested elements must implement the parent
/// derived key and the digest instead of the index.
pub fn key(element: &Element, index: usize) -> String {
    element
        .id
        .as_deref()
        .or(element.meta_id.as_deref())
        .map_or_else(|| index.to_string(), str::to_owned)
}
