//! Trusted local models and uploads share admission without sharing filesystem authority.
#![cfg(not(target_arch = "wasm32"))]
use sbml4humans_core::{SourceOptions, prepare_path};
use std::fs;

fn xml(definitions: &str, model: &str) -> String {
    format!(
        r#"<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" xmlns:comp="http://www.sbml.org/sbml/level3/version1/comp/version1" level="3" version="2" comp:required="true">{definitions}<model id="main">{model}</model></sbml>"#
    )
}
fn external(source: &str) -> String {
    format!(
        r#"<comp:listOfExternalModelDefinitions><comp:externalModelDefinition comp:id="external" comp:source="{source}"/></comp:listOfExternalModelDefinitions>"#
    )
}
const INSTANCE: &str = r#"<comp:listOfSubmodels><comp:submodel comp:id="child" comp:modelRef="external"/></comp:listOfSubmodels>"#;

#[test]
fn trusted_companions_are_breadth_first_and_untrusted_paths_never_admit_them() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("models")).unwrap();
    let main = directory.path().join("main.xml");
    fs::write(&main, xml(&external("models/child.xml"), INSTANCE)).unwrap();
    fs::write(
        directory.path().join("models/child.xml"),
        xml(&external("../leaf.xml"), INSTANCE),
    )
    .unwrap();
    fs::write(directory.path().join("leaf.xml"), xml("", "")).unwrap();
    let options = SourceOptions::default();
    assert_eq!(
        prepare_path(&main, false, &options).unwrap().entries.len(),
        1
    );
    let prepared = prepare_path(&main, true, &options).unwrap();
    assert_eq!(
        prepared
            .entries
            .iter()
            .map(|entry| entry.location.as_str())
            .collect::<Vec<_>>(),
        ["./main.xml", "./models/child.xml", "./leaf.xml"]
    );
    assert!(
        prepared.export("python", None, false, 10000).unwrap()["content"]
            .as_str()
            .unwrap()
            .contains("def f_dxdt")
    );
}

#[test]
fn archives_never_gain_directory_companions_and_unreadable_companions_remain_known() {
    let directory = tempfile::tempdir().unwrap();
    let main = directory.path().join("main.xml");
    fs::write(&main, xml(&external("bad.xml"), INSTANCE)).unwrap();
    fs::write(directory.path().join("bad.xml"), "<not-sbml/>").unwrap();
    let prepared = prepare_path(&main, true, &SourceOptions::default()).unwrap();
    assert_eq!(prepared.unavailable, ["./bad.xml"]);
    let mut archive = pymetadata_core::zip::Writer::new(Vec::new());
    archive.add_stored("manifest.xml", br#"<omexManifest><content location="main.xml" format="http://identifiers.org/combine.specifications/sbml" master="true"/></omexManifest>"#).unwrap();
    archive
        .add_stored("main.xml", xml(&external("bad.xml"), INSTANCE).as_bytes())
        .unwrap();
    let archive_path = directory.path().join("main.omex");
    fs::write(&archive_path, archive.into_bytes().unwrap()).unwrap();
    let prepared = prepare_path(&archive_path, true, &SourceOptions::default()).unwrap();
    assert!(prepared.unavailable.is_empty());
    assert_eq!(prepared.entries.len(), 1);
}

#[cfg(unix)]
#[test]
fn local_symlinks_cannot_admit_a_companion_outside_the_selected_directory() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("selected")).unwrap();
    fs::write(directory.path().join("outside.xml"), xml("", "")).unwrap();
    std::os::unix::fs::symlink(
        directory.path().join("outside.xml"),
        directory.path().join("selected/link.xml"),
    )
    .unwrap();
    let main = directory.path().join("selected/main.xml");
    fs::write(&main, xml(&external("link.xml"), INSTANCE)).unwrap();
    assert_eq!(
        prepare_path(&main, true, &SourceOptions::default())
            .unwrap()
            .entries
            .len(),
        1
    );
}

#[test]
fn path_reads_obey_request_limits_and_trusted_gzip_retains_its_model_filename() {
    use std::io::Write;
    let directory = tempfile::tempdir().unwrap();
    let main = directory.path().join("named.xml.gz");
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(xml("", "").as_bytes()).unwrap();
    fs::write(&main, encoder.finish().unwrap()).unwrap();
    let mut options = SourceOptions::default();
    options.limits.max_content_size = 1;
    assert_eq!(
        prepare_path(&main, false, &options).err().unwrap().kind,
        "refused"
    );
    assert_eq!(
        prepare_path(&main, true, &options).unwrap().entries[0].location,
        "./named.xml"
    );
}

#[test]
fn selected_example_entries_reuse_confined_companion_admission_without_extraction() {
    use sbml4humans_core::prepare_host_path;
    let directory = tempfile::tempdir().unwrap();
    let mut archive = pymetadata_core::zip::Writer::new(Vec::new());
    archive.add_stored("manifest.xml", br#"<omexManifest><content location="models/main.xml" format="http://identifiers.org/combine.specifications/sbml" master="true"/></omexManifest>"#).unwrap();
    archive
        .add_stored(
            "models/main.xml",
            xml(
                &(external("sub/child.xml") + &external("../outside.xml")),
                INSTANCE,
            )
            .as_bytes(),
        )
        .unwrap();
    archive
        .add_stored(
            "models/sub/child.xml",
            xml(&external("../leaf.xml"), INSTANCE).as_bytes(),
        )
        .unwrap();
    archive
        .add_stored("models/leaf.xml", xml("", "").as_bytes())
        .unwrap();
    archive
        .add_stored("outside.xml", xml("", "").as_bytes())
        .unwrap();
    let path = directory.path().join("example.omex");
    fs::write(&path, archive.into_bytes().unwrap()).unwrap();
    let options = SourceOptions::default();
    let prepared = prepare_host_path(&path, true, &options, Some("./models/main.xml")).unwrap();
    assert_eq!(
        prepared
            .entries
            .iter()
            .map(|entry| entry.location.as_str())
            .collect::<Vec<_>>(),
        ["./main.xml", "./sub/child.xml", "./leaf.xml"]
    );
    assert!(prepare_host_path(&path, false, &options, Some("./models/main.xml")).is_err());
    assert!(prepare_host_path(&path, true, &options, Some("./missing.xml")).is_err());
}
#[test]
fn native_io_memory_errors_keep_their_resource_category() {
    let memory =
        sbml4humans_core::ReportError::from(std::io::Error::from(std::io::ErrorKind::OutOfMemory));
    assert_eq!(memory.kind, "memory");
    let missing =
        sbml4humans_core::ReportError::from(std::io::Error::from(std::io::ErrorKind::NotFound));
    assert_eq!(missing.kind, "malformed");
}
