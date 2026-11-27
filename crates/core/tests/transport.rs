//! Streaming transport retains the complete wire payload across buffer boundaries.
use sbml4humans_core::{SourceOptions, encode_report, source};
use std::io::Read;

#[test]
fn buffered_gzip_preserves_large_complete_json_and_its_final_bytes() {
    let parameters = (0..800)
        .map(|i| format!(r#"<parameter id="p{i}" value="{i}" constant="true"/>"#))
        .collect::<String>();
    let xml = format!(
        r#"<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2"><model id="large" name="µ model"><listOfParameters>{parameters}</listOfParameters></model></sbml>"#
    );
    let response = source(xml.as_bytes(), &SourceOptions::default()).unwrap();
    let plain = encode_report(&response, false).unwrap();
    assert!(plain.content.len() > 128 * 1024);
    let packed = encode_report(&response, true).unwrap();
    let mut decoded = Vec::new();
    flate2::read::GzDecoder::new(packed.content.as_slice())
        .read_to_end(&mut decoded)
        .unwrap();
    assert_eq!(decoded, plain.content);
    assert_eq!(packed.diagnostics, plain.diagnostics);
}
