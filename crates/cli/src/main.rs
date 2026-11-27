//! The command line of the report of sbml4humans: `sbml4humans-core report <path>`.

use std::ffi::OsString;
use std::io::{BufWriter, Read, Write};
use std::process::ExitCode;

const USAGE: &str = "\
Usage: sbml4humans-core <report|source> [--pretty] <path>

Reads SBML or an OMEX archive. report writes one document; source writes the complete envelope.

Options:
  --pretty    indent the JSON
  -h, --help  print this help
";

/// What the arguments ask for.
enum Command {
    /// Print the help to stdout.
    Help,
    /// Write the report of the file at `path`.
    Report {
        path: OsString,
        pretty: bool,
        source: bool,
    },
}

/// Parses the arguments after the program name, or returns the message of the first problem.
fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut command: Option<String> = None;
    let mut path: Option<OsString> = None;
    let mut pretty = false;
    for arg in args {
        match arg.to_str() {
            Some("-h" | "--help") => return Ok(Command::Help),
            Some("--pretty") => pretty = true,
            Some(flag) if flag.starts_with('-') && flag != "-" => {
                return Err(format!("unknown option {flag}"));
            }
            Some(word) if command.is_none() => command = Some(word.to_owned()),
            _ if command.is_none() => return Err("the command is not valid UTF-8".to_owned()),
            _ if path.is_none() => path = Some(arg),
            _ => return Err(format!("unexpected argument {}", arg.to_string_lossy())),
        }
    }
    match command.as_deref() {
        Some("report" | "source") => match path {
            Some(path) => Ok(Command::Report {
                path,
                pretty,
                source: command.as_deref() == Some("source"),
            }),
            None => Err("report needs the path of a file".to_owned()),
        },
        Some(other) => Err(format!("unknown command {other}")),
        None => Err("no command given".to_owned()),
    }
}

/// Writes to stdout and returns the message of a failure, which a closed pipe is.
fn write_stdout(bytes: &[u8]) -> Result<(), String> {
    let mut out = std::io::stdout().lock();
    out.write_all(bytes)
        .and_then(|()| out.flush())
        .map_err(|e| format!("cannot write to stdout: {e}"))
}

/// Reads the file, builds the report and writes it to stdout.
fn report(path: &OsString, pretty: bool, source_envelope: bool) -> Result<(), String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("cannot read {}: {e}", path.to_string_lossy()))?;
    let mut source = Vec::new();
    file.take(sbml4humans_core::MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut source)
        .map_err(|e| format!("cannot read {}: {e}", path.to_string_lossy()))?;
    if source.len() > sbml4humans_core::MAX_INPUT_BYTES {
        return Err("The source exceeds the input size limit.".into());
    }
    if source_envelope {
        let report = sbml4humans_core::source(&source, &sbml4humans_core::SourceOptions::default())
            .map_err(|e| e.to_string())?;
        return write_json(&report, pretty);
    }
    let report = sbml4humans_core::report(&source).map_err(|e| e.to_string())?;
    write_json(&report, pretty)
}
fn write_json(report: &impl serde::Serialize, pretty: bool) -> Result<(), String> {
    let mut out = BufWriter::new(std::io::stdout().lock());
    let written = if pretty {
        serde_json::to_writer_pretty(&mut out, &report)
    } else {
        serde_json::to_writer(&mut out, &report)
    };
    written.map_err(|e| format!("cannot write the report: {e}"))?;
    writeln!(out)
        .and_then(|()| out.flush())
        .map_err(|e| format!("cannot write the report: {e}"))
}

fn main() -> ExitCode {
    let result = match parse(std::env::args_os().skip(1)) {
        Ok(Command::Help) => write_stdout(USAGE.as_bytes()),
        Ok(Command::Report {
            path,
            pretty,
            source,
        }) => report(&path, pretty, source),
        Err(message) => Err(format!("{message}\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            // a closed stderr leaves no way to say so, and the exit code still does
            let _ = writeln!(std::io::stderr(), "error: {message}");
            ExitCode::FAILURE
        }
    }
}
