//! The primary keys of the elements of a report.

use std::collections::{HashMap, HashSet};

use sbml4humans_model::Element;

/// The primary key of an element: `<scope>/<sbml_type>:<key>`.
pub fn of(scope: &str, sbml_type: &str, key: &str) -> String {
    format!("{scope}/{sbml_type}:{key}")
}

/// The key an element carries of its own: its id, else its metaId, `None` when it carries neither.
///
/// It is the rule `_own_key` reads (`sbmlinfo.py:190-199`), whose presence `_has_own_key` answers:
/// the id branch and the metaId branch of `_key` (`sbmlinfo.py:572-579`) without the target branch
/// which stands between them, which the builder answers for itself. Every function below asks for
/// it: [`key`] answers a fallback where this is `None`, and [`keyed_by`] both numbers the children
/// where it is `None` and reserves it where it is not.
fn own_key(element: &Element) -> Option<&str> {
    element.id.as_deref().or(element.meta_id.as_deref())
}

/// The name of an element in SBML: the name of its class with a lower first letter.
///
/// A port of `_element_name` (`sbmlinfo.py:207-219`), which reads the name of the libsbml class of
/// an element and never `getElementName()`, since the latter answers the name the level of the
/// document writes, `speciesConcentrationRule` for a rate rule of Level 1. The class of the
/// specification is the `sbmlType` of the report object of every element this port builds, so the
/// type the report states is what this is given. The one element for which the two differ is a comp
/// model definition, whose type is `Model` while the name of its element is `modelDefinition`, and
/// which the slice that reads comp brings. It is private: [`indexed`] is the one caller, and a
/// caller outside this module would be one which keys an element without going through it.
fn element_name(sbml_type: &str) -> String {
    let mut characters = sbml_type.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_lowercase().chain(characters).collect()
    })
}

/// The key of an element within its scope: its id, else its metaId, else what `fallback` answers.
///
/// The builder answers the name a parent gives the one child it has of a kind
/// (`<reaction>.kineticLaw`) and the literal `model` for the model of a document, which is what
/// python keys a model without either by (`MAIN_MODEL_KEY`) and which is the scope of every element
/// below it. The fallback is a closure so that an element which carries a key does not pay for
/// building one it does not use. A child of a list is keyed by [`derived`] or by [`indexed`]
/// instead, which number the whole list at once.
///
/// What is still owed of `_key` (`sbmlinfo.py:543-582`) is its last resort, the sha1 digest of the
/// xml of an element. Since sbml4humans 0.13.0 that digest keys the one element of a document no
/// list holds, its model, and only where a comp model definition takes the word `model` from it
/// (`_main_model_key`, `sbmlinfo.py:1169-1183`); every element of a list is given a key by its
/// parent and reaches no digest at all. It is therefore owed by the slice which reads comp.
pub fn key(element: &Element, fallback: impl FnOnce() -> String) -> String {
    own_key(element).map_or_else(fallback, str::to_owned)
}

/// The key of every child of one list, in the order of the list, `candidates` naming one each.
///
/// A port of `_keyed_by` (`sbmlinfo.py:265-295`), together with the preference `_key` gives an
/// identifier of the file over the key its parent derived (`sbmlinfo.py:572-581`): python hands the
/// derived key to `_key` for every child and `_key` answers the id, else the metaId, of a child
/// which carries one, so the key of such a child is its identifier and not its candidate.
///
/// A child which carries an id or a metaId (`_own_key`) takes no part in the numbering and its
/// candidate is dropped, while that identifier is reserved against the candidates of the others, so
/// that a key this function writes never names another child of the list. A candidate is built for
/// every child all the same, which is what makes the place of [`indexed`] count over the whole list.
///
/// `_keyed_by` takes a further `reserved`, the keys of elements which are no children of the list
/// and whose key its keys have to stay clear of nonetheless. The model of a document is the one
/// such element, reserved against the list of the comp model definitions, and it is owed by the
/// slice which reads comp.
pub(crate) fn keyed_by<'a, T>(
    children: &'a [T],
    element_of: impl Fn(&'a T) -> &'a Element,
    candidates: &[String],
) -> Vec<String> {
    keyed_by_reserved(children, element_of, candidates, &[])
}

pub(crate) fn keyed_by_reserved<'a, T>(
    children: &'a [T],
    element_of: impl Fn(&'a T) -> &'a Element,
    candidates: &[String],
    reserved: &[&str],
) -> Vec<String> {
    debug_assert_eq!(children.len(), candidates.len());
    let own: Vec<Option<&str>> = children
        .iter()
        .map(|child| own_key(element_of(child)))
        .collect();
    let reserved: Vec<&str> = own
        .iter()
        .copied()
        .flatten()
        .chain(reserved.iter().copied())
        .collect();
    let numbered: Vec<String> = candidates
        .iter()
        .zip(&own)
        .filter(|(_, own)| own.is_none())
        .map(|(candidate, _)| candidate.clone())
        .collect();
    // `unique` answers one key per candidate and a candidate was kept per child without an
    // identifier, so the keys never run out; the empty key is there only so that a miss cannot panic
    let mut keys = unique(&numbered, &reserved).into_iter();
    own.into_iter()
        .map(|own| own.map_or_else(|| keys.next().unwrap_or_default(), str::to_owned))
        .collect()
}

/// The key of every child of one list, each named by what it is about within its parent.
///
/// A port of `_keyed` (`sbmlinfo.py:298-310`). `element_of` reaches the element of a child, which is
/// a struct of the model carrying it, and `about` names what one of them is about, the species of a
/// species reference or the variable of an event assignment, which the file does not change when it
/// reorders the list. [`keyed_by`] turns those candidates into the keys.
pub fn derived<'a, T>(
    children: &'a [T],
    element_of: impl Fn(&'a T) -> &'a Element,
    about: impl Fn(&'a T) -> String,
) -> Vec<String> {
    let candidates: Vec<String> = children.iter().map(about).collect();
    keyed_by(children, element_of, &candidates)
}

/// The key of every child of one list, each named by its place in that list.
///
/// A port of `_indexed` (`sbmlinfo.py:313-334`). The candidate of a child is the name of its element
/// in SBML and its place among the children of that name, `reaction.3`, which no edit of the child
/// moves and which no id of the file is, since an SId carries no dot. The names are counted apart,
/// so the algebraic rules of a `listOfRules` are `algebraicRule.0` and `algebraicRule.1` whatever
/// the rules between them, and a place counts over the whole list: a child which carries an
/// identifier of its own takes its place in the count although its candidate is dropped, so that
/// giving one child an id moves the key of no other. `sbml_type` answers the type of the report
/// object of a child, which [`element_name`] turns into the name of its element.
pub fn indexed<'a, T>(
    children: &'a [T],
    element_of: impl Fn(&'a T) -> &'a Element,
    sbml_type: impl Fn(&'a T) -> &'static str,
) -> Vec<String> {
    let mut places: HashMap<&'static str, usize> = HashMap::new();
    let candidates: Vec<String> = children
        .iter()
        .map(|child| {
            let name = sbml_type(child);
            let place = places.entry(name).or_default();
            let candidate = format!("{}.{place}", element_name(name));
            *place += 1;
            candidate
        })
        .collect();
    keyed_by(children, element_of, &candidates)
}

/// The keys of one list, a key which repeats or is reserved told apart by its occurrence.
///
/// A port of `_unique_keys` (`sbmlinfo.py:239-262`). The first occurrence of a key keeps it and a
/// repetition becomes `<key>.1`, `<key>.2` and so on, passing over a key which another key of the
/// list carries already, so that a suffix never names the child of that other key. `reserved` are
/// the keys of the list which are no candidate of its own, the identifiers of the siblings the file
/// keys, and they block a candidate from the bare key on, so that a key this function writes never
/// names an element the file keys. A key of the list itself blocks from the first suffix on only,
/// which is why a key keeps itself where it is the first occurrence. The loop ends because a higher
/// occurrence is a key which neither set holds, and both sets are bounded by the length of the list.
///
/// It is public for its own sake: the numbering is the part of the rule which says nothing about
/// elements, and the kinetic law of the builder keys its local parameters through it, as python
/// does (`sbmlinfo.py:1351-1359`).
pub fn unique(keys: &[String], reserved: &[&str]) -> Vec<String> {
    let blocked: HashSet<&str> = reserved.iter().copied().collect();
    let taken: HashSet<&str> = keys
        .iter()
        .map(String::as_str)
        .chain(reserved.iter().copied())
        .collect();
    let mut used: HashSet<String> = HashSet::with_capacity(keys.len());
    let mut numbered: Vec<String> = Vec::with_capacity(keys.len());
    for key in keys {
        let mut candidate = key.clone();
        let mut occurrence: usize = 0;
        while used.contains(&candidate)
            || blocked.contains(candidate.as_str())
            || (occurrence > 0 && taken.contains(candidate.as_str()))
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

    use super::{derived, element_name, indexed, unique};

    /// An element which carries nothing but the two identifiers the keying rule reads.
    fn element(id: Option<&str>, meta_id: Option<&str>) -> Element {
        Element {
            id: id.map(str::to_owned),
            meta_id: meta_id.map(str::to_owned),
            ..Element::default()
        }
    }

    #[test]
    fn the_name_of_an_element_is_its_type_with_a_lower_first_letter() {
        assert_eq!(element_name("Reaction"), "reaction");
        assert_eq!(element_name("AlgebraicRule"), "algebraicRule");
        assert_eq!(element_name("SBMLDocument"), "sBMLDocument");
        assert_eq!(element_name(""), "");
    }

    #[test]
    fn a_key_which_repeats_is_numbered_by_its_occurrence() {
        let keys = ["J0.reactant.S1", "J0.reactant.S2", "J0.reactant.S1"].map(str::to_owned);
        assert_eq!(
            unique(&keys, &[]),
            ["J0.reactant.S1", "J0.reactant.S2", "J0.reactant.S1.1"]
        );
    }

    #[test]
    fn a_number_another_child_carries_is_passed_over() {
        let keys = ["a", "a.1", "a", "a"].map(str::to_owned);
        assert_eq!(unique(&keys, &[]), ["a", "a.1", "a.2", "a.3"]);
    }

    #[test]
    fn a_reserved_key_blocks_a_candidate_from_the_bare_key_on() {
        // a reserved key blocks a candidate at every occurrence, while a key of the list blocks
        // from the first suffix on only, which is why `a` keeps itself in the second line
        let keys = ["a", "a"].map(str::to_owned);
        assert_eq!(unique(&keys, &["a"]), ["a.1", "a.2"]);
        assert_eq!(unique(&keys, &["a.1"]), ["a", "a.2"]);
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
    fn the_identifier_of_a_sibling_is_reserved_against_a_derived_key() {
        // the id `a.1` belongs to a child which takes no part, and the repetition passes over it
        let children = [
            element(None, None),
            element(Some("a.1"), None),
            element(None, None),
        ];
        let keys = derived(&children, |e| e, |_| "a".to_owned());
        assert_eq!(keys, ["a", "a.1", "a.2"]);
    }

    #[test]
    fn a_metaid_which_names_a_place_sends_the_child_of_that_place_one_further() {
        // an SId holds no dot and can never be the place of a child, a metaId is an XML `ID` and
        // can: the child of the place `reaction.1` passes over the sibling which carries it
        let children = [element(None, Some("reaction.1")), element(None, None)];
        let keys = indexed(&children, |e| e, |_| "Reaction");
        assert_eq!(keys, ["reaction.1", "reaction.1.1"]);
    }

    #[test]
    fn the_place_of_a_child_counts_over_the_whole_list_and_per_name() {
        // the second rule is keyed by its metaId and the fourth is `algebraicRule.2` all the same,
        // and the assignment rule between them is counted under a name of its own
        let children = [
            (element(None, None), "AlgebraicRule"),
            (element(None, Some("meta")), "AlgebraicRule"),
            (element(None, None), "AssignmentRule"),
            (element(None, None), "AlgebraicRule"),
        ];
        let keys = indexed(&children, |(e, _)| e, |(_, kind)| *kind);
        assert_eq!(
            keys,
            [
                "algebraicRule.0",
                "meta",
                "assignmentRule.0",
                "algebraicRule.2"
            ]
        );
    }
}
