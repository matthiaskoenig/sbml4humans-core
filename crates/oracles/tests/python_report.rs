//! The gate: the Rust report of a document equals the python report, field for field.

use sbml4humans_oracles::{compare, names_a_pointer};

// What this slice does not build yet, and the values it fabricates, each line saying which slice
// removes it. Every later slice removes lines from this list, and the list is empty when the port
// is complete.
//
// The rule of the list, so that the next reader applies it rather than deriving it again: an entry
// belongs here when the port writes a value which **differs** from the one python writes on a
// document the gate already has, which is what would make the gate red today. The fabricated `-` of
// `derivedUnits`, the missing `unitsLatex`, the empty `linkGraph` and the lists of a package are
// those. A value the port does not build which **agrees** with python on every document of the gate
// is declared below instead and stays compared: silencing an agreement buys nothing and costs the
// comparison. No field this slice does build belongs here either, for the same reason: such a field
// is compared, so a regression in the code the gate guards shows up.
//
// Every entry of this list silences the same pointer of every document of the gate, so a pointer
// which names an element by its place in a list belongs to the one document it is owed on and not
// here: `/models/0/listOfReactions/1` is a different reaction in every document. Those entries are
// the `only` of a line of `GATED` below. Both lists are measured by
// `every_entry_of_the_exclusion_lists_silences_a_difference_of_a_document_it_covers` and by
// `every_entry_of_the_exclusion_lists_names_a_pointer_of_a_document_it_covers`.
//
// What the port does not build and no entry silences. Every value named below is `null`, the empty
// list or absent on both sides of every document of the gate, so each of them agrees today and none
// of them is silenced. The day a document states one, the gate fails at its pointer with
// `ours null but python "mole"`, which names the value which is owed and asks for it to be built;
// an entry here would take that failure away and leave the liveness test saying that a line of the
// list became live, which is less. So this is the declaration, and no list to widen:
//
//   - The six unit attributes of a model, `substanceUnits`, `timeUnits`, `volumeUnits`, `areaUnits`,
//     `lengthUnits` and `extentUnits` (core §4.2.4). A plain read the filler skips, owed by the
//     slice which finishes the attributes of a model. Setting `substanceUnits` and `timeUnits` is
//     near universal in a Level 3 model, so this is the class the next document of the gate is
//     likeliest to make red.
//   - The six latex renderings of those attributes, `substanceUnitsLatex` to `extentUnitsLatex`.
//     They need the renderer `**/unitsLatex` below waits for, owed by the slice which derives
//     units. `**/unitsLatex` covers none of the six: the segment match of an entry is exact, and
//     the segment of such a field is `substanceUnitsLatex` and never `unitsLatex`.
//   - The `conversionFactor` of a model and of a species (core §4.2.5, §4.6.7). Python resolves the
//     parameter the attribute names in the model and writes its `sid`, its `value` and its `units`
//     (`sbmlinfo.conversion_factor`), so a species with a `conversionFactor="cf"` is
//     `{"sid": "cf", "units": null, "value": 2.0}` there and `null` here. Owed by the slice which
//     resolves the parameter an element names.
//   - The `lists` of every element: the `listOf` containers which state something of their own, a
//     `listOfReactants` with an id, a `listOfSpecies` with a notes, which every `ListOf` class of
//     SBML may carry because it derives from `SBase` (core §4.2.7, `sbmlinfo.lists`). Owed by the
//     slice which reads the lists of an element.
//   - The `fbc` of a species and of a reaction (fbc §3.4, §3.5), owed by the slice which reads fbc.
//     The `fbc` of a model is the one of the three which differs on a document of the gate today,
//     the `{"activeObjective": null, "strict": false}` of `minimal_model.xml`, and is the entry
//     `/models/0/fbc` below.
//   - The `history`, the `comp`, the `uncertainties` and the `keyValuePairs` of every element, which
//     `build::sbase` leaves at the default of `SBase`, and the `externalModelDefinitions` of the
//     report. Owed by the slices which read the annotation RDF (the history of an element is parsed
//     out of it, as the cvterms are), the comp extension of an element, the distrib and the fbc
//     annotations of one, and the model definitions of a document. The `cvterms` of an element is of
//     that same kind and is the one with an entry, the live `MINIMAL_MODEL_ONLY` below, because a
//     document of the gate reaches it and the port differs there.
//
// This list and that declaration together are the whole account of what the port does not build, as
// of the last reading of the builder against `model.py` of the python implementation. The values of
// the declaration were written down at the review of this slice, found by that reading and by
// measuring an ordinary Level 3 Version 2 document against python, and not by the gate, which is
// green over every one of them and stays green over them.
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

// The annotation of an element is not read into cvterms yet, and `minimal_model.xml` is the one
// document of the gate which carries one: python reads the bqbiol:is of its parameter `k1` into a
// cvterm, and the port writes the empty list. The container itself stays compared, so a second
// term would differ. The pointer names the parameter by its place, so it is scoped to this
// document: the parameter at that place in another document carries no annotation today and would
// lose the comparison of a cvterm it grew tomorrow. Removed by the slice which reads the
// annotation RDF.
const MINIMAL_MODEL_ONLY: &[&str] = &["/models/0/listOfParameters/0/cvterms/0"];

// Python keys an element which carries no id and no metaId by the sha1 digest of its xml (`_key`,
// `sbmlinfo.py:463`); this port keys it by the index of the element in the list of its parent
// (`pk::key`), the rule every element type of slice 1 and of this slice follows, and the digest is
// named there as what the port still owes. `core_elements.xml` holds the one such element of the
// gate, the reaction which states nothing at all, and these are the two pointers its key reaches:
// the pk of the reaction, which sits at the index 1 of the reactions of this document, and the pk
// of the reference below it, whose key its parent derives from the key of the reaction. Every
// other value of both elements stays compared. They are scoped to this document because they name
// their element by its place: in `level2_reactions.xml` the same two pointers are the pk of the
// reaction `J1` and of its reference, both of which the port builds as python does and which are
// compared. Removed by the slice which writes the digest, which the other element types keyed by
// it need as well.
const CORE_ELEMENTS_ONLY: &[&str] = &[
    "/models/0/listOfReactions/1/pk",
    "/models/0/listOfReactions/1/listOfReactants/0/pk",
];

/// One document of the gate: the stem of `testdata/<name>.xml` and of its fixture, and the entries
/// which silence a part of the report of that document and of no other.
struct Gated {
    /// The stem of the document and of the fixture of the python report of it.
    name: &'static str,
    /// What is excluded on this document alone, next to the shared `EXCLUDED`.
    only: &'static [&'static str],
}

/// Writes `GATED` and the gate test of every document from one list.
///
/// The documents of the gate are named once, in the invocation below, so that a document cannot be
/// gated without being measured by the liveness test of the exclusion lists, nor be listed there
/// without being gated. A line is the name of its test, the stem of the document and the entries
/// excluded on it alone.
macro_rules! gated {
    ($($test:ident = $name:literal, $only:expr;)*) => {
        const GATED: &[Gated] = &[$(Gated { name: $name, only: $only }),*];
        $(
            #[test]
            fn $test() {
                gate($name);
            }
        )*
    };
}

gated! {
    // The document of the first slice: the compartments, the species and the parameters, with the
    // comp port and the fbc plugin libsbml gives it and the one annotation of the gate.
    the_report_of_minimal_model_equals_the_python_report = "minimal_model", MINIMAL_MODEL_ONLY;
    // Level 2 is where libsbml and the python report agree on values no document states: the
    // defaults of the specification as set attributes, and the layout and the render plugin as
    // declared packages. The fixture pins both rather than a hand comparison.
    the_report_of_a_level_2_document_equals_the_python_report = "level2_model", &[];
    // The document of this slice, one element of every type of SBML core: the shapes whose value
    // no other document of the gate pins, among them a reaction and an event which state nothing
    // at all, a reference keyed by an id of its own next to one keyed by its place, the three
    // stoichiometries the half equation of python formats by a branch of its own, a local
    // parameter which shadows a global one and a unit of a fractional exponent.
    the_report_of_core_elements_equals_the_python_report = "core_elements", CORE_ELEMENTS_ONLY;
    // What only an older level has: the defaults Level 2 gives `reversible`, `fast` and the
    // stoichiometry of a reference which states none, a kinetic law whose parameters the document
    // writes as `<parameter>` in a `<listOfParameters>`, and a reference with a `stoichiometryMath`,
    // which is a stoichiometry of its own and is reported as none.
    the_report_of_level2_reactions_equals_the_python_report = "level2_reactions", &[];
}

// The entries of `EXCLUDED` which no document of the gate reaches, and why: each is the list of the
// elements of a package, which the port writes as the empty list python writes for a document
// without that package, so only a document which carries the package differs in it. The two
// documents this slice adds hold no package at all, by the ruling of the slice, and of the two
// before them `minimal_model.xml` is the one with a comp port and an fbc plugin, which is why
// `/models/0/listOfPorts` and `/models/0/fbc` are live and are not listed here. The slice which
// reads the packages brings the documents which make these seven live, and removes them from
// `EXCLUDED` together with the two.
//
// An entry belongs here only where the port writes something a document of the gate could differ
// in, which a list of a package is: the port writes the empty list of a document without the
// package, so a document with it differs. A value which agrees with python on every document, like
// the ones declared at the head of `EXCLUDED`, is in neither list.
const UNREACHED: &[&str] = &[
    "/models/0/listOfSubmodels",
    "/models/0/listOfGeneProducts",
    "/models/0/listOfObjectives",
    "/models/0/listOfFluxBounds",
    "/models/0/listOfUserDefinedConstraints",
    "/models/0/listOfQualitativeSpecies",
    "/models/0/listOfTransitions",
];

/// The document of `GATED` named `name`, which the gate test of every one of them passes.
fn of(name: &str) -> &'static Gated {
    GATED
        .iter()
        .find(|document| document.name == name)
        .unwrap_or_else(|| panic!("{name} is no document of GATED"))
}

/// What is silenced of the report of one document: the shared entries and the entries of its own.
fn excluded_for(document: &Gated) -> Vec<&'static str> {
    EXCLUDED.iter().chain(document.only).copied().collect()
}

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
    let document = of(name);
    let excluded = excluded_for(document);
    let (ours, theirs) = reports(name);
    let differences = compare(&ours, &theirs, &excluded);
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    // anti-vacuity: the walk reaches the fixture, so an empty report is not equal to it
    assert!(!compare(&ours, &serde_json::json!({}), &excluded).is_empty());
}

/// The value with every leaf of it replaced, so that a comparison with it answers one difference
/// per leaf the walk reaches: the objects and the arrays keep their shape and every leaf becomes a
/// string no leaf of a report equals, whatever its type.
fn every_leaf_replaced(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(fields) => serde_json::Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), every_leaf_replaced(value)))
                .collect(),
        ),
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(every_leaf_replaced).collect())
        }
        _ => serde_json::Value::String("\u{0}no leaf of a report".to_owned()),
    }
}

/// How many leaves of the report of a document the gate compares with `excluded` silenced.
fn compared_leaves(
    ours: &serde_json::Value,
    theirs: &serde_json::Value,
    excluded: &[&str],
) -> usize {
    compare(ours, &every_leaf_replaced(theirs), excluded).len()
}

// That an entry which is owed on one document silences no other, which is what scoping it to its
// line of `GATED` is for. The two pointers of `CORE_ELEMENTS_ONLY` name a value of
// `level2_reactions.xml` as well: the pk of its reaction `J1`, which sits at the index 1 of its
// reactions, and the pk of the reference below it, which its parent derived from the key of that
// reaction. Both are keys the port builds as python does, so with those two entries shared by every
// document the gate of this one would pass over two of the pointers at which it reads an id keyed
// reaction and a parent derived reference key. Not over all of them: this document holds two
// reactions, `J0` and `J1`, and four references below them, so the gate would still read the key of
// `J0` and of the three other references. What the two entries would cost is measured below and is
// exactly two compared leaves.
#[test]
fn an_entry_which_is_owed_on_one_document_silences_no_pointer_of_another() {
    let document = of("level2_reactions");
    let (ours, theirs) = reports(document.name);
    let keys = [
        "level2_reactions/Reaction:J1",
        "level2_reactions/SpeciesReference:J1.reactant.S2",
    ];
    let excluded = excluded_for(document);
    for (entry, key) in CORE_ELEMENTS_ONLY.iter().zip(keys) {
        // the pointer names a key of this document, and a key both sides agree on
        assert_eq!(ours.pointer(entry).and_then(|v| v.as_str()), Some(key));
        assert_eq!(theirs.pointer(entry), ours.pointer(entry));
        assert!(!excluded.contains(entry), "{entry}");
    }
    // and the two are leaves the gate of this document reaches: silencing them takes exactly two
    // compared leaves out of the walk, which is what sharing them would cost
    let shared: Vec<&str> = excluded
        .iter()
        .copied()
        .chain(CORE_ELEMENTS_ONLY.iter().copied())
        .collect();
    assert_eq!(
        compared_leaves(&ours, &theirs, &excluded) - compared_leaves(&ours, &theirs, &shared),
        2
    );
}

/// Whether `entry`, taken alone, silences a difference of this document.
///
/// `baseline` is the number of differences of the document with nothing silenced at all. Taking the
/// entry alone isolates it, so an entry whose pointer lies below another one is measured for itself
/// and the order of a list says nothing; and silencing a pointer prunes the walk below it, so the
/// count of the differences drops exactly when the entry names one of them.
fn silences(
    entry: &str,
    ours: &serde_json::Value,
    theirs: &serde_json::Value,
    baseline: usize,
) -> bool {
    compare(ours, theirs, &[entry]).len() < baseline
}

// The liveness of the exclusion lists, the measure of what the port does not build yet. A shared
// entry of `EXCLUDED` is measured against every document, since one list is shared by all of them:
// an entry which is dead on one document is normal and an entry which is dead on all of them names
// nothing any more. An entry which is scoped to one document is measured against that document
// alone, which is the document it is owed on and the only one it silences. Either way a dead entry
// is one of two things: the value it hid is a value the port now builds, and the gate would no
// longer guard that value, or no document of the gate reaches it, and the line reads as a gap of the
// port which is none. The second is the honest state of the seven package lists of `UNREACHED`,
// which name what they wait for; every other entry has to be live.
#[test]
fn every_entry_of_the_exclusion_lists_silences_a_difference_of_a_document_it_covers() {
    // the stem of a document appears once in `GATED`: `of` answers the first line which carries it,
    // so a stem written twice would measure that one document twice and leave the other unmeasured
    let mut names: Vec<&str> = GATED.iter().map(|document| document.name).collect();
    names.sort_unstable();
    let written = names.len();
    names.dedup();
    assert_eq!(names.len(), written, "a stem of GATED is written twice");
    let gated: Vec<(serde_json::Value, serde_json::Value)> = GATED
        .iter()
        .map(|document| reports(document.name))
        .collect();
    // the differences of a document with nothing silenced, one walk per document rather than one
    // per entry, and the measure of every entry below
    let baselines: Vec<usize> = gated
        .iter()
        .map(|(ours, theirs)| compare(ours, theirs, &[]).len())
        .collect();
    for (baseline, document) in baselines.iter().zip(GATED) {
        // every document differs in something, or the measure would be vacuous for every entry
        assert!(*baseline > 0, "{}", document.name);
    }
    let mut dead: Vec<String> = Vec::new();
    let mut reached: Vec<&str> = Vec::new();
    for entry in EXCLUDED {
        let live = gated
            .iter()
            .zip(&baselines)
            .filter(|((ours, theirs), baseline)| silences(entry, ours, theirs, **baseline))
            .count();
        if live == 0 && !UNREACHED.contains(entry) {
            dead.push(format!("{entry}, of EXCLUDED"));
        }
        if live > 0 && UNREACHED.contains(entry) {
            reached.push(entry);
        }
    }
    for (((ours, theirs), baseline), document) in gated.iter().zip(&baselines).zip(GATED) {
        for entry in document.only {
            if !silences(entry, ours, theirs, *baseline) {
                dead.push(format!("{entry}, of {}", document.name));
            }
            // and an entry is owed on one document or on all of them, never in both lists
            assert!(
                !EXCLUDED.contains(entry),
                "{entry} is an entry of {} and of EXCLUDED",
                document.name,
            );
        }
    }
    assert!(
        dead.is_empty(),
        "{} entry or entries of the exclusion lists silence no difference of a document they \
         cover, so they name nothing the port still owes: {dead:?}",
        dead.len(),
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

// That every entry of the exclusion lists is spelled to match the report, which its liveness does
// not say: an entry of `UNREACHED` silences no difference by definition, and neither does an entry
// which names nothing at all. The two are told apart here, by asking whether the entry matches a
// pointer of the report of a document it covers, the pointer of an object or of an empty list
// included. The form of an entry is easy to get wrong in a way nothing else catches: the segment
// match is exact, so `**/unitsLatex` names `unitsLatex` and never `substanceUnitsLatex`, and an
// entry written for the wrong segment silences nothing while reading like a line which does.
#[test]
fn every_entry_of_the_exclusion_lists_names_a_pointer_of_a_document_it_covers() {
    let gated: Vec<(serde_json::Value, serde_json::Value)> = GATED
        .iter()
        .map(|document| reports(document.name))
        .collect();
    let mut nothing: Vec<String> = Vec::new();
    for entry in EXCLUDED {
        // the pointer may be one python writes and the port does not, or the other way round, so
        // both sides of every document are asked
        let named = gated
            .iter()
            .any(|(ours, theirs)| names_a_pointer(entry, ours) || names_a_pointer(entry, theirs));
        if !named {
            nothing.push(format!("{entry}, of EXCLUDED"));
        }
    }
    for ((ours, theirs), document) in gated.iter().zip(GATED) {
        for entry in document.only {
            if !names_a_pointer(entry, ours) && !names_a_pointer(entry, theirs) {
                nothing.push(format!("{entry}, of {}", document.name));
            }
        }
    }
    assert!(
        nothing.is_empty(),
        "{} entry or entries of the exclusion lists match no pointer of the report of a document \
         they cover, so they are misspelled and declare nothing: {nothing:?}",
        nothing.len(),
    );
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
