//! Every math of the corpus fixture parses to a tree and writes the formula python writes.

use sbml4humans_libsbml::read_math;
use sbml4humans_model::AstKind;
use sbml4humans_oracles::load_maths;
use sbml4humans_report::formula;

/// The differences a run of the gate prints before it names their number.
const PRINTED: usize = 20;

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

// The formula of the fixture is `libsbml.formulaToL3String` of the syntax tree python builds of
// the same MathML, so this compares the port of that writer against the writer itself over every
// math of the corpus. One run reports every difference, so that a red gate can be read by the
// shape of its failures rather than one entry at a time; it prints the first `PRINTED` of them
// and then their number, so a writer which is wrong everywhere does not flood the terminal.
#[test]
fn the_formula_of_every_math_of_the_fixture_equals_the_python_one() {
    let maths = load_maths();
    // the count is asserted here as well as in the test above, so that a
    // fixture which lost its entries fails this gate rather than passing it
    // over nothing
    assert_eq!(maths.len(), 1832);
    let mut differences = Vec::new();
    for entry in &maths {
        let ast = read_math(&entry.mathml).unwrap_or_else(|e| panic!("{e:?}: {}", entry.mathml));
        match formula(&ast) {
            Ok(ours) if ours == entry.formula => {}
            Ok(ours) => differences.push(format!(
                "ours {ours:?} but python {:?}, of {}",
                entry.formula, entry.mathml
            )),
            Err(error) => differences.push(format!(
                "ours {error} but python {:?}, of {}",
                entry.formula, entry.mathml
            )),
        }
    }
    assert!(
        differences.is_empty(),
        "{} of the {} maths of the fixture write another formula than python:\n{}",
        differences.len(),
        maths.len(),
        differences
            .iter()
            .take(PRINTED)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
