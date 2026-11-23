//! Every math of the corpus fixture parses to a tree.

use sbml4humans_libsbml::read_math;
use sbml4humans_model::AstKind;
use sbml4humans_oracles::load_maths;

// The fixture is the math of the whole corpus, enumerated document wide: one of its 1,832
// entries comes from a comp model definition (a kinetic law in comp_deletion.xml), which the
// python report renders too. A change which narrows the enumeration to the lists of the main
// model gives fewer and fails this count with that reason.
#[test]
fn every_math_of_the_fixture_parses_to_a_tree() {
    let maths = load_maths();
    assert_eq!(maths.len(), 1832);
    for entry in &maths {
        let ast = read_math(&entry.mathml).unwrap_or_else(|e| panic!("{e:?}: {}", entry.mathml));
        assert_ne!(
            ast.kind,
            AstKind::Other,
            "an unmapped kind in {}",
            entry.mathml
        );
    }
}
