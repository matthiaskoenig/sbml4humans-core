//! The command line of the report, run as a person runs it.

use std::process::{Command, Output, Stdio};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sbml4humans-core"))
        .args(args)
        .output()
        .unwrap()
}

/// A temporary directory of one test, removed whichever way the test leaves its scope.
///
/// The removal is a `Drop` and no statement of the test, so a panic between the creation and the
/// end of the test, of an assertion or of an `unwrap` of the setup, leaves nothing behind; and it
/// ignores a failure to remove, which would otherwise mask the failure the test is about.
struct TempDir(std::path::PathBuf);

impl TempDir {
    /// A new directory named after `what` and the process, created empty.
    fn new(what: &str) -> Self {
        let path = std::env::temp_dir().join(format!("s4h-cli-{what}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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
    let dir = TempDir::new("empty");
    let path = dir.0.join("empty.xml");
    std::fs::write(&path, b"").unwrap();
    assert_fails(&run(&["report", path.to_str().unwrap()]));
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
fn a_path_which_is_no_utf8_is_read_like_any_other() {
    use std::os::unix::ffi::OsStrExt;
    // the path reaches the file system as the bytes it is, never as a String, so a name which is
    // no UTF-8 names a readable file like any other and the report of that file is written
    let dir = TempDir::new("no-utf8");
    let path = dir.0.join(std::ffi::OsStr::from_bytes(b"model_\xff.xml"));
    std::fs::copy("../../testdata/minimal_model.xml", &path).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_sbml4humans-core"))
        .arg("report")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["document"]["pk"], "document/SBMLDocument:document");
}

#[cfg(unix)]
#[test]
fn a_closed_stdout_is_a_failure_with_no_panic() {
    // The read end of the pipe is closed before the binary starts, so its first write to stdout
    // gets EPIPE and nothing of it can land in the buffer of the pipe first: no race to lose.
    // Closing the descriptor outright instead would prove nothing, because the standard library
    // reopens a closed descriptor 0, 1 or 2 onto /dev/null before main runs and the write
    // succeeds. The exit code is the whole point of the test, so it is asserted exactly.
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let child = Command::new(env!("CARGO_BIN_EXE_sbml4humans-core"))
        .arg("--help")
        .stdout(writer)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    assert_fails(&child.wait_with_output().unwrap());
}
