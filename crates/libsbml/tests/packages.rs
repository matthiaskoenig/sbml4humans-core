//! The package set of the linked libsbml, against the one the manifest records.

/// The registry of the linked libsbml lists the packages of `rust_libsbml` in
/// `testdata/oracles/MANIFEST.json`.
///
/// The manifest records what the Rust side links, and the prefix built by
/// `scripts/build-libsbml.sh` and a prefix somebody else built link different
/// package sets, so this fails against a prefix other than the one the
/// manifest describes, by design: a prefix built elsewhere, such as one with
/// arrays and without multi and spatial, fails it.
#[test]
fn the_linked_libsbml_registers_the_package_set_the_manifest_records() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../testdata/oracles/MANIFEST.json"))
            .expect("the manifest parses");
    let recorded: Vec<String> = manifest["rust_libsbml"]["packages"]
        .as_array()
        .expect("the manifest lists packages")
        .iter()
        .map(|name| name.as_str().expect("a package is a name").to_owned())
        .collect();
    assert_eq!(sbml4humans_libsbml::registered_packages(), recorded);
}
