//! The entire application response of complete archives, with no excluded report fields.
use std::io::Read;
#[test]
fn complete_source_responses_match_python() {
    let root = std::path::Path::new("../../testdata/sources");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("MANIFEST.json")).unwrap()).unwrap();
    let mut failures = Vec::new();
    for case in manifest["files"].as_array().unwrap() {
        let input = std::fs::read(root.join(case["source"].as_str().unwrap())).unwrap();
        let compressed = std::fs::read(root.join(case["report"].as_str().unwrap())).unwrap();
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(compressed.as_slice())
            .read_to_end(&mut bytes)
            .unwrap();
        let expected = serde_json::from_slice(&bytes).unwrap();
        let mut options = sbml4humans_core::SourceOptions::default();
        if let Some(budget) = case["budget"].as_u64() {
            options.max_expanded_elements = budget as usize;
        }
        let actual = match sbml4humans_core::source_json(&input, &options) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap(),
            Err(error) => {
                failures.push(format!("{}: {error}", case["origin"]));
                continue;
            }
        };
        let differences = sbml4humans_oracles::compare(&actual, &expected, &[]);
        if !differences.is_empty() {
            eprintln!("{}: {} differences", case["origin"], differences.len());
        }
        failures.extend(differences.into_iter().take(8).map(|d| {
            format!(
                "{}: {}",
                case["origin"],
                d.to_string().chars().take(450).collect::<String>()
            )
        }));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
