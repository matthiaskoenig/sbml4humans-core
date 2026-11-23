//! Every math of the corpus fixture parses to a tree and writes the formula python writes.

use sbml4humans_libsbml::read_math;
use sbml4humans_model::AstKind;
use sbml4humans_oracles::{MathFixture, load_maths};
use sbml4humans_report::{formula, math};

/// The differences a run of the gate prints before it names their number.
const PRINTED: usize = 20;

/// How many entries the fixture holds: the 1,832 maths of the corpus and the 2 of the bound.
const ENTRIES: usize = 1834;

// The fixture is the math of the whole corpus, enumerated document wide: one of its 1,832
// corpus entries comes from a comp model definition (a kinetic law in comp_deletion.xml), which
// the python report renders too. A change which narrows the enumeration to the lists of the main
// model gives fewer and fails this count with that reason. The other two entries are the pair the
// generator builds at the bound of the latex pipeline, which no model of the corpus comes near.
#[test]
fn every_math_of_the_fixture_parses_to_a_tree() {
    let maths = load_maths();
    assert_eq!(maths.len(), ENTRIES);
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
    assert_eq!(maths.len(), ENTRIES);
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

// The latex of the fixture is what `sbml4humans.mathml.astnode_to_latex` writes of the same
// MathML: the output of the two stylesheets of the python implementation, after its ten
// replacements and its two heuristics. This compares the port of that pipeline against it over
// every math of the corpus, in the shape of the gate above.
//
// It compares what `math` answers and not what `latex` writes, so that the two rules of
// `math_info` are gated as well: the latex of an entry is the `Option` python answered for it, so
// a rule of ours which fires where python writes a latex, and one which does not fire where python
// writes none, are both differences like any other.
#[test]
fn the_latex_of_every_math_of_the_fixture_equals_the_python_one() {
    let maths = load_maths();
    assert_eq!(maths.len(), ENTRIES);
    let mut differences = Vec::new();
    for entry in &maths {
        let ast = read_math(&entry.mathml).unwrap_or_else(|e| panic!("{e:?}: {}", entry.mathml));
        match math(&ast).map(|written| written.latex) {
            Ok(ours) if ours == entry.latex => {}
            Ok(ours) => differences.push(format!(
                "ours {ours:?} but python {:?}, of {}",
                entry.latex, entry.mathml
            )),
            Err(error) => differences.push(format!(
                "ours {error} but python {:?}, of {}",
                entry.latex, entry.mathml
            )),
        }
    }
    assert!(
        differences.is_empty(),
        "{} of the {} maths of the fixture write another latex than python:\n{}",
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

/// How many divisions the Level 3 formula of an entry writes.
///
/// It names the two chains of nested divides the generator of the fixture adds, which are the only
/// entries with more than a handful: the one at the bound writes 254 and the one past it 255.
fn divisions(entry: &MathFixture) -> usize {
    entry.formula.matches(" / ").count()
}

// The bound of the latex pipeline, pinned by the two entries the generator builds for it. Python
// typesets a math whose mathml is 256 elements deep and reports one of 257 as its formula alone,
// since lxml parses no deeper document (`mathml.py`), and `math_info` of this port answers `None`
// for the second by the mapping of `mathml_depth`, which was measured shape by shape against
// libsbml and has no entry of the corpus anywhere near the bound to answer for it. These two are
// that answer, and they come from python like every other entry: a port which measured the depth
// one element short, or one too far, fails here and in the gate of the latex above.
//
// Reading the mathml of the entry at the bound needs a libsbml built with expat, which
// `scripts/build-libsbml.sh` and the CI do: libxml2 enforces the same 256 element limit lxml does,
// so a libsbml built against it would answer no latex one level earlier than python.
#[test]
fn the_fixture_holds_the_two_maths_at_the_bound_of_the_latex_pipeline() {
    let maths = load_maths();
    let chains: Vec<&MathFixture> = maths.iter().filter(|m| divisions(m) >= 254).collect();
    assert_eq!(chains.len(), 2, "the two chains of divides of the fixture");
    let at = chains.iter().find(|m| divisions(m) == 254).expect("254");
    let past = chains.iter().find(|m| divisions(m) == 255).expect("255");
    // the shallower of the two is typeset by python and by this port, the deeper by neither
    assert!(at.latex.is_some());
    assert_eq!(past.latex, None);
    // and it is the one math of the whole fixture python wrote no latex for
    assert_eq!(maths.iter().filter(|m| m.latex.is_none()).count(), 1);
    for entry in [at, past] {
        let ast = read_math(&entry.mathml).expect("the mathml of a chain of divides reads");
        assert_eq!(
            math(&ast).expect("a chain of divides is written").latex,
            entry.latex
        );
    }
}
