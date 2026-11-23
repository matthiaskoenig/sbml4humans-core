//! The primary keys of the elements of a report.

use std::collections::HashSet;

use sbml4humans_model::Element;

/// The primary key of an element: `<scope>/<sbml_type>:<key>`.
pub fn of(scope: &str, sbml_type: &str, key: &str) -> String {
    format!("{scope}/{sbml_type}:{key}")
}

/// The key an element carries of its own: its id, else its metaId, `None` when it carries neither.
///
/// It is the rule `_has_own_key` reads (`sbmlinfo.py:190`): the id branch and the metaId branch of
/// `_key` (`sbmlinfo.py:455-460`) without the target branch which stands between them, which the
/// builder answers for itself. Both [`key`] and [`derived`] below ask for it: `key` answers a
/// fallback where this is `None` and `derived` numbers the children where it is.
fn own_key(element: &Element) -> Option<&str> {
    element.id.as_deref().or(element.meta_id.as_deref())
}

/// The key of an element within its scope: its id, else its metaId, else what `fallback` answers.
///
/// The builder answers the index of the element in the list of its parent, the name a parent gives the one
/// child it has of a kind (`<reaction>.kineticLaw`), and the literal `model` for the model of a document,
/// which is what python keys a model without either by (`MAIN_MODEL_KEY`) and not by a position; the key of
/// the model is the scope of every element below it. The fallback is a closure so that an element which
/// carries a key does not pay for building one it does not use.
///
/// The index is the rule of the elements of the first slice and is not the rule of the python
/// implementation. Python takes the id, else the id of a class keyed by its target, else the metaId, else a
/// key which the parent derived for the child (the species of a species reference, with a `.1`, `.2` suffix
/// for a repetition), else a digest of the xml of the element. SBML requires an id on every element of that
/// slice, so the index is reachable only on an invalid document. The parent derived key is `derived` below,
/// which a parent calls for a list of nested children instead of keying them one by one through this
/// fallback. What is still owed is the digest, and a digest for a model named `model` when a comp model
/// definition of the same document carries that id.
pub fn key(element: &Element, fallback: impl FnOnce() -> String) -> String {
    own_key(element).map_or_else(fallback, str::to_owned)
}

/// The key of every child of one list, in the order of the list.
///
/// A port of `_keyed` (`sbmlinfo.py:233-252`), together with the preference `_key` gives an identifier of
/// the file over the key its parent derived (`sbmlinfo.py:452-462`): python hands the derived key to `_key`
/// for every child and `_key` answers the id, else the metaId, of a child which carries one, so the key of
/// such a child is its identifier and not what `about` says. `element_of` reaches the element of a child,
/// which is a struct of the model carrying it, and `about` names what one of the others is about, the
/// species of a species reference, which the file does not change when it reorders the list.
///
/// A child which carries an id or a metaId (`_has_own_key`, `sbmlinfo.py:190`) takes no part in the
/// numbering, which therefore knows nothing of the identifiers of the siblings: a derived `a.1` collides
/// with a sibling whose id is `a.1`. That is what python does, and the report of this implementation is the
/// report of python.
pub fn derived<'a, T>(
    children: &'a [T],
    element_of: impl Fn(&'a T) -> &'a Element,
    about: impl Fn(&'a T) -> String,
) -> Vec<String> {
    let own: Vec<Option<String>> = children
        .iter()
        .map(|child| own_key(element_of(child)).map(str::to_owned))
        .collect();
    let candidates: Vec<String> = children
        .iter()
        .zip(&own)
        .filter(|(_, own)| own.is_none())
        .map(|(child, _)| about(child))
        .collect();
    // `unique` answers one key per candidate and a candidate was taken per child without an identifier,
    // so the keys never run out; the empty key is there only so that a miss cannot panic.
    let mut keys = unique(&candidates).into_iter();
    own.into_iter()
        .map(|own| own.unwrap_or_else(|| keys.next().unwrap_or_default()))
        .collect()
}

/// The keys of one list, a key which repeats an earlier one told apart by its occurrence.
///
/// A port of `_unique_keys` (`sbmlinfo.py:212-230`). The first occurrence of a key keeps it and a repetition
/// becomes `<key>.1`, `<key>.2` and so on, passing over a candidate which another key of the list carries
/// already, so that a suffix never names the child of that other key. A key keeps itself where it is the
/// first occurrence although it is reserved, which is why the reservation is read from the first suffix on
/// only. The loop ends because a higher occurrence is a key which neither set holds, and both sets are
/// bounded by the length of the list.
///
/// It is public for its own sake: the numbering is the part of the rule which says nothing about elements,
/// and `derived` is the one caller.
pub fn unique(keys: &[String]) -> Vec<String> {
    let reserved: HashSet<&str> = keys.iter().map(String::as_str).collect();
    let mut used: HashSet<String> = HashSet::with_capacity(keys.len());
    let mut numbered: Vec<String> = Vec::with_capacity(keys.len());
    for key in keys {
        let mut candidate = key.clone();
        let mut occurrence: usize = 0;
        while used.contains(&candidate) || (occurrence > 0 && reserved.contains(candidate.as_str()))
        {
            occurrence += 1;
            candidate = format!("{key}.{occurrence}");
        }
        used.insert(candidate.clone());
        numbered.push(candidate);
    }
    numbered
}

#[cfg(test)]
mod tests {
    use sbml4humans_model::Element;

    use super::{derived, unique};

    /// An element which carries nothing but the two identifiers the keying rule reads.
    fn element(id: Option<&str>, meta_id: Option<&str>) -> Element {
        Element {
            id: id.map(str::to_owned),
            meta_id: meta_id.map(str::to_owned),
            ..Element::default()
        }
    }

    #[test]
    fn a_key_which_repeats_is_numbered_by_its_occurrence() {
        let keys = ["J0.reactant.S1", "J0.reactant.S2", "J0.reactant.S1"].map(str::to_owned);
        assert_eq!(
            unique(&keys),
            ["J0.reactant.S1", "J0.reactant.S2", "J0.reactant.S1.1"]
        );
    }

    #[test]
    fn a_number_another_child_carries_is_passed_over() {
        let keys = ["a", "a.1", "a", "a"].map(str::to_owned);
        assert_eq!(unique(&keys), ["a", "a.1", "a.2", "a.3"]);
    }

    #[test]
    fn a_child_with_an_identifier_is_keyed_by_it_and_takes_no_part() {
        // three children about the same species, the middle one with an id of its own
        let children = [
            element(None, None),
            element(Some("sr"), None),
            element(None, Some("meta")),
        ];
        let keys = derived(&children, |e| e, |_| "R.reactant.S1".to_owned());
        assert_eq!(keys, ["R.reactant.S1", "sr", "meta"]);
    }

    #[test]
    fn the_identifier_of_a_sibling_does_not_number_a_derived_key() {
        // the id `a.1` belongs to a child which takes no part, so the repetition is still `a.1`
        let children = [
            element(None, None),
            element(Some("a.1"), None),
            element(None, None),
        ];
        let keys = derived(&children, |e| e, |_| "a".to_owned());
        assert_eq!(keys, ["a", "a.1", "a.1"]);
    }
}
