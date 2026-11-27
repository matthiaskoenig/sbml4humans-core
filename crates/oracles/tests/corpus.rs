//! Every field of the bundled BioModels and application models is compared.
use sbml4humans_oracles::compare;
use std::io::Read;

fn decoded(path: &std::path::Path) -> Vec<u8> {
    let compressed = std::fs::read(path).unwrap();
    let mut bytes = Vec::new();
    flate2::read::MultiGzDecoder::new(compressed.as_slice())
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}
#[test]
fn complete_reports_match_the_full_model_corpus() {
    let root = std::path::Path::new("../../testdata/corpus");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("MANIFEST.json")).unwrap()).unwrap();
    let mut failures = Vec::new();
    for case in manifest["files"].as_array().unwrap() {
        let source = root.join(case["source"].as_str().unwrap());
        let expected =
            serde_json::from_slice(&decoded(&root.join(case["report"].as_str().unwrap()))).unwrap();
        for reader in ["pure Rust pipeline", "native oracle"] {
            let actual = if reader == "pure Rust pipeline" {
                serde_json::from_slice(
                    &sbml4humans_core::report_json(&std::fs::read(&source).unwrap()).unwrap(),
                )
                .unwrap()
            } else {
                let input = sbml4humans_libsbml::read(&decoded(&source)).unwrap();
                serde_json::to_value(sbml4humans_report::build(&input).unwrap()).unwrap()
            };
            let differences = compare(&actual, &expected, &[]);
            if !differences.is_empty() {
                eprintln!(
                    "{} ({reader}): {} differences",
                    case["origin"],
                    differences.len()
                );
            }
            failures.extend(differences.into_iter().take(4).map(|d| {
                format!(
                    "{} ({reader}): {}",
                    case["origin"],
                    d.to_string().chars().take(420).collect::<String>()
                )
            }));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
