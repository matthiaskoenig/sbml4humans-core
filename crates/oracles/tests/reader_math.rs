//! The pure Rust mathematical reader feeds the unchanged report math writers.

#[test]
fn pure_rust_math_matches_the_entire_python_math_corpus() {
    let corpus: serde_json::Value =
        serde_json::from_slice(include_bytes!("../../../testdata/oracles/maths.json")).unwrap();
    let mut differences = Vec::new();
    for (index, case) in corpus["maths"].as_array().unwrap().iter().enumerate() {
        let ast = match sbml4humans_reader::read_math(case["mathml"].as_str().unwrap()) {
            Ok(Some(ast)) => ast,
            other => {
                differences.push(format!("{index}: reader {other:?}"));
                continue;
            }
        };
        let actual = sbml4humans_report::math(&ast).unwrap();
        if Some(actual.formula.as_str()) != case["formula"].as_str()
            || actual.latex.as_deref() != case["latex"].as_str()
        {
            differences.push(format!(
                "{index}: formula {:?} != {}; latex {:?} != {}",
                actual.formula, case["formula"], actual.latex, case["latex"]
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "{} differences:\n{}",
        differences.len(),
        differences
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
