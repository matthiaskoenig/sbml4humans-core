//! The readable equation of a reaction.

use sbml4humans_model::{Reaction, SpeciesReference};

use crate::number::python_float;

/// The arrow of a reversible reaction (U+21C6).
const REVERSIBLE: char = '⇆';

/// The arrow of an irreversible reaction (U+279E).
const IRREVERSIBLE: char = '➞';

/// How far a stoichiometry may lie from 1 or -1 and still be written as none at all.
///
/// It is the tolerance of `_half_equation` (`sbmlinfo.py:1244-1262`) and not a comparison of the
/// doubles: a document which writes `0.999999999` reads as `S1` in both implementations.
const TOLERANCE: f64 = 1e-8;

/// The readable equation of a reaction: its two half equations and a plain unicode arrow.
///
/// A port of `_equation` and `_half_equation` (`sbmlinfo.py:1236-1262`). The halves and the arrow
/// are joined by single spaces, so a half with no participant leaves the space of its own: a
/// reaction with neither a reactant nor a product is the three characters `" ⇆ "`, as python
/// writes it. The modifiers of a reaction take no part in its equation.
///
/// The arrow is the raw `reversible` of libsbml, which answers the default `true` of a reaction of
/// any level which sets nothing, and not the `reversible` the report shows, which is `null` for
/// such a reaction of Level 3.
pub(crate) fn equation(reaction: &Reaction) -> String {
    let left = half(&reaction.reactants);
    let right = half(&reaction.products);
    let arrow = if reaction.reversible.unwrap_or(true) {
        REVERSIBLE
    } else {
        IRREVERSIBLE
    };
    format!("{left} {arrow} {right}")
}

/// One half of an equation: the references of one side of the reaction, in the order of the list.
fn half(references: &[SpeciesReference]) -> String {
    references
        .iter()
        .map(term)
        .collect::<Vec<String>>()
        .join(" + ")
}

/// One term of a half equation: a species with the coefficient its stoichiometry reads as.
///
/// The number is the raw answer of libsbml and no value the report derives, so a reference of
/// Level 1 or Level 2 which writes no stoichiometry carries the 1 of its level and one of Level 3
/// carries a `NaN`, which stands for a stoichiometry an initial assignment or a rule sets: python
/// names it by the id of the reference, which such a document has to carry, and by a `?` where the
/// reference carries none.
fn term(reference: &SpeciesReference) -> String {
    let species = &reference.species;
    let stoichiometry = reference.stoichiometry;
    if stoichiometry.is_nan() {
        let coefficient = reference.element.id.as_deref().unwrap_or("?");
        format!("{coefficient} {species}")
    } else if (stoichiometry - 1.0).abs() < TOLERANCE {
        species.clone()
    } else if (stoichiometry + 1.0).abs() < TOLERANCE {
        format!("-{species}")
    } else {
        format!("{} {species}", python_float(stoichiometry))
    }
}

#[cfg(test)]
mod tests {
    use sbml4humans_model::{Element, Reaction, SpeciesReference};

    use super::equation;

    /// The equation of a reaction of the shorthand of [`reference`].
    fn equation_of(reactants: &[&str], products: &[&str], reversible: bool) -> String {
        equation(&Reaction {
            reversible: Some(reversible),
            reactants: reactants.iter().copied().map(reference).collect(),
            products: products.iter().copied().map(reference).collect(),
            ..Reaction::default()
        })
    }

    /// A reference of the shorthand `"<stoichiometry> <species>"`, or `"<species>"` for one of 1.
    ///
    /// The stoichiometry counts as set, as a document which writes the attribute sets it: the
    /// equation reads the raw number of libsbml and not the value the report shows, so `"nan S"`
    /// is the reference of a document of Level 3 which writes no stoichiometry at all.
    fn reference(term: &str) -> SpeciesReference {
        let (stoichiometry, species) = match term.split_once(' ') {
            Some((coefficient, species)) => (coefficient.parse().unwrap(), species),
            None => (1.0, term),
        };
        SpeciesReference {
            species: species.to_owned(),
            stoichiometry,
            stoichiometry_set: true,
            ..SpeciesReference::default()
        }
    }

    #[test]
    fn an_equation_is_written_as_python_writes_it() {
        assert_eq!(
            equation_of(&["2 S1", "S1"], &["S2"], true),
            "2.0 S1 + S1 ⇆ S2"
        );
        assert_eq!(equation_of(&["S1"], &[], false), "S1 ➞ ");
        assert_eq!(equation_of(&[], &[], true), " ⇆ ");
        assert_eq!(equation_of(&["-1 S1"], &["0.5 S2"], false), "-S1 ➞ 0.5 S2");
        assert_eq!(
            equation_of(&["1e-05 S1"], &["nan S2"], false),
            "1e-05 S1 ➞ ? S2"
        );
    }

    #[test]
    fn a_reaction_which_sets_no_reversible_is_written_with_the_reversible_arrow() {
        // python reads the raw `getReversible()` of libsbml, which answers `true` for a reaction
        // of any level which sets nothing, so the arrow is the reversible one at every level and
        // it is no default of a level the builder writes
        let reaction = Reaction {
            reversible: None,
            reactants: vec![reference("S1")],
            products: vec![reference("S2")],
            ..Reaction::default()
        };
        assert_eq!(equation(&reaction), "S1 ⇆ S2");
    }

    #[test]
    fn a_reference_whose_stoichiometry_is_no_number_is_written_with_its_id() {
        // a reference of Level 3 which writes no stoichiometry is set by an initial assignment or
        // a rule which names the id of the reference, and the equation names that id
        let mut named = reference("nan S1");
        named.element = Element {
            id: Some("sr_S1".to_owned()),
            ..Element::default()
        };
        let reaction = Reaction {
            reversible: Some(false),
            reactants: vec![named],
            products: vec![reference("S2")],
            ..Reaction::default()
        };
        assert_eq!(equation(&reaction), "sr_S1 S1 ➞ S2");
    }
}
