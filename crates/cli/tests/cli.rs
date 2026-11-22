//! The command line of the report, run as a person runs it.

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sbml4humans-core"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn report_writes_the_json_of_the_report_to_stdout() {
    let out = run(&["report", "../../testdata/minimal_model.xml"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["document"]["pk"], "document/SBMLDocument:document");
    assert_eq!(
        v["models"][0]["listOfSpecies"][0]["pk"],
        "minimal_model/Species:S1"
    );
}

#[test]
fn a_file_which_is_no_document_fails_with_a_message_and_no_panic() {
    let out = run(&["report", "../../Cargo.toml"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
}
