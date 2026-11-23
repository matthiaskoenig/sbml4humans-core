//! The gate: the Rust report of a document equals the python report, field for field.

use sbml4humans_oracles::compare;

// What this slice does not build yet, and the values it fabricates, each line saying which slice
// removes it. Nothing else belongs here: a field this slice does build is compared, so a
// regression in the code the gate guards shows up. Every later slice removes lines from this
// list, and the list is empty when the port is complete. The list itself is measured by
// `every_entry_of_the_exclusion_list_silences_a_difference_of_one_document_at_least` below.
const EXCLUDED: &[&str] = &[
    "/models/0/listOfPorts",
    "/models/0/fbc",
    "/models/0/listOfSubmodels",
    "/models/0/listOfGeneProducts",
    "/models/0/listOfObjectives",
    "/models/0/listOfFluxBounds",
    "/models/0/listOfUserDefinedConstraints",
    "/models/0/listOfQualitativeSpecies",
    "/models/0/listOfTransitions",
    // The annotation of an element is not read into cvterms yet, python reads the bqbiol:is of the
    // parameter k1 into one. The container itself stays compared, so a second term would differ.
    // Removed by the slice which reads the annotation RDF.
    "/models/0/listOfParameters/0/cvterms/0",
    // Python keys an element which carries no id and no metaId by the sha1 digest of its xml
    // (`_key`, `sbmlinfo.py:463`); this port keys it by the index of the element in the list of its
    // parent (`pk::key`), the rule every element type of slice 1 and of this slice follows, and
    // the digest is named there as what the port still owes. `core_elements.xml` holds one such
    // element, the reaction which states nothing at all, and these are the two pointers its key
    // reaches: the pk of the reaction and the pk of the reference below it, whose key its parent
    // derives from the key of the reaction. Every other value of both elements stays compared.
    // Removed by the slice which writes the digest, which the other element types keyed by it need
    // as well.
    "/models/0/listOfReactions/1/pk",
    "/models/0/listOfReactions/1/listOfReactants/0/pk",
    // The one field the builder knowingly fabricates: it writes the `-` python writes for an
    // element whose unit does not derive, for every compartment, species, parameter, initial
    // assignment, rule, kinetic law and local parameter, the seven types of the report which carry
    // it. Python derives the unit of an element, `l` for a compartment with a size,
    // `\frac{mol}{l}` for a species with an amount, so the value is wrong for a document which
    // sets units at all. Removed by the slice which derives units.
    "**/derivedUnits",
    // The latex of the units of an element, whose input the model of this port already holds:
    // `units.udef_to_string` renders it from the units the element names, of a unit definition of
    // the model or of a base unit of SBML, and of the five types which carry it the unit
    // definition is the one whose input is the definition itself. What is missing here is that
    // renderer and not a reader, which sets this line apart from `**/derivedUnits` above, where
    // the port cannot ask libsbml what it derives. Removed by the slice which derives units.
    "**/unitsLatex",
    "/linkGraph",
    "/odeSystem",
    "/odeError",
];

// The documents of the gate. Each `the_report_of_..._equals_the_python_report` test below names
// one of them, and the liveness test of the exclusion list walks all of them: one list is shared by
// every document, so an entry which is dead on one of them is normal and an entry which is dead on
// all of them is a line which outlived its slice. A document added to the gate belongs here too.
const GATED: &[&str] = &[
    "minimal_model",
    "level2_model",
    "core_elements",
    "level2_reactions",
];

// The entries of `EXCLUDED` which no document of the gate reaches, and why: each is the list of the
// elements of a package, which the port writes as the empty list python writes for a document
// without that package, so only a document which carries the package differs in it. The two
// documents this slice adds hold no package at all, by the ruling of the slice, and of the two
// before them `minimal_model.xml` is the one with a comp port and an fbc plugin, which is why
// `/models/0/listOfPorts` and `/models/0/fbc` are live and are not listed here. The slice which
// reads the packages brings the documents which make these seven live, and removes them from
// `EXCLUDED` together with the two.
const UNREACHED: &[&str] = &[
    "/models/0/listOfSubmodels",
    "/models/0/listOfGeneProducts",
    "/models/0/listOfObjectives",
    "/models/0/listOfFluxBounds",
    "/models/0/listOfUserDefinedConstraints",
    "/models/0/listOfQualitativeSpecies",
    "/models/0/listOfTransitions",
];

/// The Rust report of `testdata/<name>.xml` and the python fixture of the same name, as JSON.
fn reports(name: &str) -> (serde_json::Value, serde_json::Value) {
    let source = std::fs::read(format!("../../testdata/{name}.xml")).unwrap();
    let ours = serde_json::to_value(
        sbml4humans_report::build(&sbml4humans_libsbml::read(&source).unwrap()).unwrap(),
    )
    .unwrap();
    let theirs: serde_json::Value = serde_json::from_slice(
        &std::fs::read(format!("../../testdata/oracles/{name}.json")).unwrap(),
    )
    .unwrap();
    (ours, theirs)
}

/// Compares the Rust report of `testdata/<name>.xml` with the python fixture of the same name.
fn gate(name: &str) {
    let (ours, theirs) = reports(name);
    let differences = compare(&ours, &theirs, EXCLUDED);
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    // anti-vacuity: the walk reaches the fixture, so an empty report is not equal to it
    assert!(!compare(&ours, &serde_json::json!({}), EXCLUDED).is_empty());
}

#[test]
fn the_report_of_minimal_model_equals_the_python_report() {
    gate("minimal_model");
}

#[test]
fn the_report_of_a_level_2_document_equals_the_python_report() {
    // Level 2 is where libsbml and the python report agree on values no document states: the
    // defaults of the specification as set attributes, and the layout and the render plugin as
    // declared packages. The fixture pins both rather than a hand comparison.
    gate("level2_model");
}

#[test]
fn the_report_of_core_elements_equals_the_python_report() {
    // The document of this slice, one element of every type of SBML core: the shapes whose value
    // no other document of the gate pins, among them a reaction and an event which state nothing
    // at all, a reference keyed by an id of its own next to one keyed by its place, the three
    // stoichiometries the half equation of python formats by a branch of its own, a local
    // parameter which shadows a global one and a unit of a fractional exponent.
    gate("core_elements");
}

#[test]
fn the_report_of_level2_reactions_equals_the_python_report() {
    // What only an older level has: the defaults Level 2 gives `reversible`, `fast` and the
    // stoichiometry of a reference which states none, a kinetic law whose parameters the document
    // writes as `<parameter>` in a `<listOfParameters>`, and a reference with a `stoichiometryMath`,
    // which is a stoichiometry of its own and is reported as none.
    gate("level2_reactions");
}

/// On how many documents of the gate `entry` silences a difference of its own.
///
/// An entry is live on a document when taking that entry alone silences at least one difference of
/// that document, measured against the comparison with nothing silenced at all. Taking it alone
/// isolates it, so an entry whose pointer lies below another one is counted for itself and the
/// order of the list says nothing; and silencing a pointer prunes the walk below it, so the count
/// of the differences drops exactly when the entry names one of them.
fn live_on(entry: &str, gated: &[(serde_json::Value, serde_json::Value)]) -> usize {
    gated
        .iter()
        .filter(|(ours, theirs)| {
            compare(ours, theirs, &[entry]).len() < compare(ours, theirs, &[]).len()
        })
        .count()
}

// The liveness of the exclusion list, the measure of what the port does not build yet. One list is
// shared by every document, so an entry which is dead on one of them is normal and an entry which
// is dead on all of them names nothing any more: either the value it hid is a value the port now
// builds, and the gate would no longer guard that value, or no document of the gate reaches it, and
// the line reads as a gap of the port which is none. The second is the honest state of the seven
// package lists of `UNREACHED`, which name what they wait for; every other entry has to be live.
#[test]
fn every_entry_of_the_exclusion_list_silences_a_difference_of_one_document_at_least() {
    let gated: Vec<(serde_json::Value, serde_json::Value)> =
        GATED.iter().map(|name| reports(name)).collect();
    // every document differs in something, or the measure would be vacuous for every entry
    for ((ours, theirs), name) in gated.iter().zip(GATED) {
        assert!(!compare(ours, theirs, &[]).is_empty(), "{name}");
    }
    let mut dead = Vec::new();
    let mut reached = Vec::new();
    for entry in EXCLUDED {
        let live = live_on(entry, &gated);
        if live == 0 && !UNREACHED.contains(entry) {
            dead.push(*entry);
        }
        if live > 0 && UNREACHED.contains(entry) {
            reached.push(*entry);
        }
    }
    assert!(
        dead.is_empty(),
        "{} of the {} entries of the exclusion list silence no difference of any of the {} \
         documents of the gate, so they name nothing the port still owes: {dead:?}",
        dead.len(),
        EXCLUDED.len(),
        GATED.len(),
    );
    assert!(
        reached.is_empty(),
        "an entry which `UNREACHED` says no document of the gate reaches does silence a \
         difference now, so it belongs to the list of the live entries: {reached:?}",
    );
    // and every entry of `UNREACHED` is an entry of the list it exempts
    for entry in UNREACHED {
        assert!(EXCLUDED.contains(entry), "{entry} is no entry of EXCLUDED");
    }
}

#[test]
fn the_comparison_is_structural_and_not_textual() {
    let a = serde_json::json!({"b": 1, "a": [1.0, 2.0]});
    let b = serde_json::json!({"a": [1.0, 2.0], "b": 1});
    assert!(compare(&a, &b, &[]).is_empty());
    let c = serde_json::json!({"a": [2.0, 1.0], "b": 1});
    assert_eq!(compare(&a, &c, &[]).len(), 2);
    let d = serde_json::json!({"a": [1.0, 2.0], "b": 2});
    assert_eq!(compare(&a, &d, &["/b"]).len(), 0);
}

#[test]
fn a_key_present_on_one_side_only_is_a_difference_in_both_directions() {
    let with = serde_json::json!({"a": 1, "b": 2});
    let without = serde_json::json!({"a": 1});
    assert_eq!(compare(&with, &without, &[]).len(), 1);
    assert_eq!(compare(&without, &with, &[]).len(), 1);
    assert_eq!(compare(&with, &without, &["/b"]).len(), 0);
    assert_eq!(compare(&without, &with, &["/b"]).len(), 0);
}

#[test]
fn arrays_of_a_different_length_differ_once_per_surplus_item() {
    let short = serde_json::json!([1, 2]);
    let long = serde_json::json!([1, 2, 3, 4]);
    assert_eq!(compare(&short, &long, &[]).len(), 2);
    assert_eq!(compare(&long, &short, &[]).len(), 2);
    assert_eq!(compare(&long, &short, &["/2"]).len(), 1);
}

#[test]
fn a_type_mismatch_is_a_difference() {
    use serde_json::json;
    assert_eq!(compare(&json!({}), &json!([]), &[]).len(), 1);
    assert_eq!(compare(&json!(0), &json!("0"), &[]).len(), 1);
    assert_eq!(compare(&json!(false), &json!(null), &[]).len(), 1);
}

#[test]
fn an_integer_and_a_float_of_the_same_value_agree() {
    use serde_json::json;
    assert_eq!(compare(&json!({"b": 1}), &json!({"b": 1.0}), &[]).len(), 0);
    assert_eq!(compare(&json!({"b": 1.0}), &json!({"b": 1}), &[]).len(), 0);
    assert_eq!(compare(&json!({"b": 1}), &json!({"b": 1.5}), &[]).len(), 1);
}

#[test]
fn an_exclusion_hides_a_pointer_and_its_descendants_and_not_a_sibling_with_the_same_prefix() {
    use serde_json::json;
    let ours = json!({"kind": {"x": 1}, "kindOf": 1});
    let theirs = json!({"kind": {"x": 2}, "kindOf": 2});
    assert_eq!(compare(&ours, &theirs, &[]).len(), 2);
    assert_eq!(compare(&ours, &theirs, &["/kind"]).len(), 1);
    assert_eq!(compare(&ours, &theirs, &["/kind/x"]).len(), 1);
    assert_eq!(compare(&ours, &theirs, &["/kind", "/kindOf"]).len(), 0);
}

#[test]
fn an_exact_exclusion_carries_the_escaping_of_rfc_6901() {
    use serde_json::json;
    let ours = json!({"a/b": 1, "c~d": 1});
    let theirs = json!({"a/b": 2, "c~d": 2});
    assert_eq!(compare(&ours, &theirs, &["/a~1b", "/c~0d"]).len(), 0);
    assert_eq!(compare(&ours, &theirs, &["/a/b", "/c~d"]).len(), 2);
}

#[test]
fn a_trailing_segment_exclusion_hides_that_segment_wherever_it_sits() {
    use serde_json::json;
    let ours = json!({"a": {"x": 1}, "b": [{"x": {"deep": 1}}], "x": 1});
    let theirs = json!({"a": {"x": 2}, "b": [{"x": {"deep": 2}}], "x": 2});
    assert_eq!(compare(&ours, &theirs, &[]).len(), 3);
    assert_eq!(compare(&ours, &theirs, &["**/x"]).len(), 0);
}

#[test]
fn a_trailing_segment_exclusion_matches_the_whole_segment_and_not_its_end() {
    use serde_json::json;
    let ours = json!({"a": {"yx": 1}, "ax": 1});
    let theirs = json!({"a": {"yx": 2}, "ax": 2});
    assert_eq!(compare(&ours, &theirs, &["**/x"]).len(), 2);
    assert_eq!(compare(&ours, &theirs, &["**/yx", "**/ax"]).len(), 0);
}

#[test]
fn an_empty_exclusion_hides_nothing_rather_than_the_whole_report() {
    use serde_json::json;
    let ours = json!({"a": 1});
    let theirs = json!({"a": 2});
    assert_eq!(compare(&ours, &theirs, &[""]).len(), 1);
    assert_eq!(compare(&ours, &theirs, &["**/"]).len(), 1);
}
