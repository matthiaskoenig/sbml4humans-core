//! The primary keys of the elements of a report.

use sbml4humans_model::Element;

/// The primary key of an element: `<scope>/<sbml_type>:<key>`.
pub fn of(scope: &str, sbml_type: &str, key: &str) -> String {
    format!("{scope}/{sbml_type}:{key}")
}

/// The key of an element within its scope: its id, else its metaId, else what `fallback` answers.
///
/// The builder answers the index of the element in the list of its parent, and the literal `model` for
/// the model of a document, which is what python keys a model without either by (`MAIN_MODEL_KEY`) and
/// not by a position; the key of the model is the scope of every element below it. The fallback is a
/// closure so that an element which carries a key does not pay for building one it does not use.
///
/// The index is the rule of this slice and is not the rule of the python implementation. Python takes the id,
/// else the id of a class keyed by its target, else the metaId, else a key which the parent derived for the
/// child (the species of a species reference, with a `.1`, `.2` suffix for a repetition), else a digest of the
/// xml of the element. SBML requires an id on every element this slice builds, so the fallback is reachable
/// only on an invalid document. A later slice which adds the nested elements must implement the parent
/// derived key and the digest instead of the index, and answer a digest for a model named `model` when a
/// comp model definition of the same document carries that id.
pub fn key(element: &Element, fallback: impl FnOnce() -> String) -> String {
    element
        .id
        .as_deref()
        .or(element.meta_id.as_deref())
        .map_or_else(fallback, str::to_owned)
}
