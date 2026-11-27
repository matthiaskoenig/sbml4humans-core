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

#[test]
fn nested_gzip_models_share_the_total_decoded_source_budget() {
    use std::io::Write;
    let mut writer = pymetadata_core::zip::Writer::new(Vec::new());
    let manifest=br#"<omexManifest><content location="a.xml.gz" format="http://identifiers.org/combine.specifications/sbml"/><content location="b.xml.gz" format="http://identifiers.org/combine.specifications/sbml"/></omexManifest>"#;
    writer.add_stored("manifest.xml", manifest).unwrap();
    let xml = format!(
        r#"<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2"><model id="inner"><notes><p xmlns="http://www.w3.org/1999/xhtml">{}</p></notes></model></sbml>"#,
        "x".repeat(900)
    );
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let compressed = gzip.finish().unwrap();
    writer.add_stored("a.xml.gz", &compressed).unwrap();
    writer.add_stored("b.xml.gz", &compressed).unwrap();
    let bytes = writer.into_bytes().unwrap();
    let mut options = SourceOptions::default();
    options.limits.max_content_size = 1600;
    assert!(bytes.len() < 1600 && xml.len() < 1600 && xml.len() * 2 > 1600);
    let error = source(&bytes, &options).unwrap_err();
    assert_eq!(error.kind, "refused");
    assert!(error.message.contains("total source size"));
    assert_eq!(
        source(&bytes, &SourceOptions::default())
            .unwrap()
            .reports
            .len(),
        2
    );
}

#[test]
fn inferred_sedml_is_master_and_unknown_entries_are_not() {
    let mut writer = pymetadata_core::zip::Writer::new(Vec::new());
    writer
        .add_stored("manifest.xml", b"<omexManifest/>")
        .unwrap();
    writer.add_stored("simulation.sedml", b"<sedML/>").unwrap();
    writer.add_stored("unknown.dat", b"unknown").unwrap();
    let result = source(&writer.into_bytes().unwrap(), &SourceOptions::default()).unwrap();
    assert!(
        result
            .manifest
            .entries
            .iter()
            .find(|e| e.location == "./simulation.sedml")
            .unwrap()
            .master
    );
    assert!(
        !result
            .manifest
            .entries
            .iter()
            .find(|e| e.location == "./unknown.dat")
            .unwrap()
            .master
    );
}
#[test]
fn standalone_models_cannot_replace_derived_archive_locations() {
    for location in [".", "./manifest.xml"] {
        let options = SourceOptions {
            location: location.into(),
            ..SourceOptions::default()
        };
        assert_eq!(source(XML, &options).unwrap_err().kind, "refused");
    }
}
