//! User-visible source limits and manifest compatibility through the complete Rust pipeline.
use sbml4humans_core::{SourceOptions, source};
const XML:&[u8]=br#"<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2"><model id="source"/></sbml>"#;
fn archive(manifest: &[u8]) -> Vec<u8> {
    let mut writer = pymetadata_core::zip::Writer::new(Vec::new());
    writer.add_stored("manifest.xml", manifest).unwrap();
    writer.add_deflated("a.xml", XML, 6).unwrap();
    writer.into_bytes().unwrap()
}
#[test]
fn duplicate_manifest_declarations_retain_metadata_but_produce_one_report() {
    let bytes=archive(br#"<omexManifest><content location="a.xml" format="http://identifiers.org/combine.specifications/sbml" master="false"/><content location="./a.xml" format="http://identifiers.org/combine.specifications/sbml.level-3.version-2" master="true"/><content location="missing.xml" format="http://identifiers.org/combine.specifications/sbml"/></omexManifest>"#);
    let result = source(&bytes, &SourceOptions::default()).unwrap();
    assert_eq!(result.manifest.entries.len(), 4);
    assert!(!result.manifest.entries[2].master);
    assert!(result.manifest.entries[3].master);
    assert_eq!(result.reports.len(), 1);
    assert_eq!(result.reports.keys().next().unwrap(), "./a.xml");
}
#[test]
fn configured_limits_refuse_input_before_inflation_and_do_not_leak_between_calls() {
    let bytes=archive(br#"<omexManifest><content location="a.xml" format="http://identifiers.org/combine.specifications/sbml"/></omexManifest>"#);
    let mut options = SourceOptions::default();
    options.limits.max_archive_entries = 1;
    let error = source(&bytes, &options).unwrap_err();
    assert_eq!(error.kind, "refused");
    assert!(error.message.contains("more than 1 entries"));
    assert_eq!(
        source(&bytes, &SourceOptions::default())
            .unwrap()
            .reports
            .len(),
        1
    );
    options = SourceOptions::default();
    options.limits.max_content_size = 1;
    assert_eq!(source(XML, &options).unwrap_err().kind, "refused");
}
#[test]
fn corrupt_member_crc_is_reported_and_does_not_poison_the_next_source() {
    let mut bytes=archive(br#"<omexManifest><content location="a.xml" format="http://identifiers.org/combine.specifications/sbml"/></omexManifest>"#);
    // Corrupt the model's authoritative central CRC while retaining a readable directory and manifest.
    let central = bytes
        .windows(4)
        .enumerate()
        .filter(|(_, b)| *b == b"PK\x01\x02")
        .map(|(i, _)| i)
        .nth(1)
        .unwrap();
    bytes[central + 16] ^= 1;
    assert_eq!(
        source(&bytes, &SourceOptions::default()).unwrap_err().kind,
        "malformed"
    );
    assert_eq!(
        source(XML, &SourceOptions::default())
            .unwrap()
            .reports
            .len(),
        1
    );
}
