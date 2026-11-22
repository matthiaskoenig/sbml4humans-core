//! The command line of the report, run as a person runs it.

use std::process::{Command, Output, Stdio};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sbml4humans-core"))
        .args(args)
        .output()
        .unwrap()
}

/// Asserts a failure of the command line: exit code 1, a message and nothing on stdout.
fn assert_fails(out: &Output) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.starts_with("error:"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert!(
        out.stdout.is_empty(),
        "stdout holds {} bytes",
        out.stdout.len()
    );
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
    assert_fails(&out);
}

#[test]
fn pretty_writes_the_same_report_over_many_lines() {
    let path = "../../testdata/minimal_model.xml";
    let compact = run(&["report", path]);
    let pretty = run(&["report", "--pretty", path]);
    assert!(pretty.status.success());
    assert!(String::from_utf8_lossy(&pretty.stdout).lines().count() > 10);
    let a: serde_json::Value = serde_json::from_slice(&compact.stdout).unwrap();
    let b: serde_json::Value = serde_json::from_slice(&pretty.stdout).unwrap();
    assert_eq!(a, b);
}

#[test]
fn an_option_after_the_path_is_read_as_an_option() {
    let out = run(&["report", "../../testdata/minimal_model.xml", "--pretty"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).lines().count() > 10);
}

#[test]
fn help_prints_the_usage_and_exits_0() {
    let out = run(&["--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("Usage:"));
    assert!(out.stderr.is_empty());
}

#[test]
fn a_missing_file_fails() {
    assert_fails(&run(&["report", "../../testdata/no_such_file.xml"]));
}

#[test]
fn a_directory_fails() {
    assert_fails(&run(&["report", "../../testdata"]));
}

#[test]
fn an_empty_file_fails() {
    let path = std::env::temp_dir().join(format!("s4h-cli-empty-{}", std::process::id()));
    std::fs::write(&path, b"").unwrap();
    let out = run(&["report", path.to_str().unwrap()]);
    std::fs::remove_file(&path).unwrap();
    assert_fails(&out);
}

#[test]
fn an_unknown_option_fails() {
    assert_fails(&run(&[
        "report",
        "--bogus",
        "../../testdata/minimal_model.xml",
    ]));
}

#[test]
fn an_unknown_command_fails() {
    assert_fails(&run(&["bogus"]));
}

#[test]
fn no_argument_fails() {
    assert_fails(&run(&[]));
}

#[test]
fn a_report_without_a_path_fails() {
    assert_fails(&run(&["report"]));
}

#[test]
fn a_second_path_fails() {
    assert_fails(&run(&["report", "a.xml", "b.xml"]));
}

#[cfg(unix)]
#[test]
fn a_path_which_is_no_utf8_fails() {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::OsStr::from_bytes(b"no_such_\xff.xml");
    let out = Command::new(env!("CARGO_BIN_EXE_sbml4humans-core"))
        .arg("report")
        .arg(path)
        .output()
        .unwrap();
    assert_fails(&out);
}

#[cfg(unix)]
#[test]
fn a_closed_stdout_is_a_failure_with_no_panic() {
    // the reader of the pipe is gone before the binary writes, as in `--help | head -c0`
    let mut child = Command::new(env!("CARGO_BIN_EXE_sbml4humans-core"))
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
    // the write races the close, so the exit is 0 when it came first and 1 when it lost
    assert!(
        matches!(out.status.code(), Some(0 | 1)),
        "{:?} {stderr}",
        out.status
    );
}
