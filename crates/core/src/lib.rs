//! One report pipeline for the CLI, native Python module and browser worker.

use serde::Serialize;
use std::borrow::Cow;
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
mod filesystem;
#[cfg(not(target_arch = "wasm32"))]
pub use filesystem::prepare_path;
pub use sbml4humans_report::normalize_location;
#[cfg(not(target_arch = "wasm32"))]
mod host;
#[cfg(not(target_arch = "wasm32"))]
pub use host::{
    ExampleSummary, ReportTransport, encode_report, example_summary, prepare_host_path, report_path,
};
mod source;
pub use source::{
    PreparedEntry, PreparedSource, SourceLimits, SourceManifest, SourceOptions, SourceResponse,
    prepare, source, source_json,
};

/// The maximum uncompressed XML input accepted by the default host API.
pub const MAX_INPUT_BYTES: usize = 100 * 1024 * 1024;

/// A report failure with an explicit category shared by every host.
#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct ReportError {
    /// `malformed`, `refused`, `internal`, or native `memory`, for host routing.
    pub kind: &'static str,
    /// The human-readable cause without an operating-system traceback.
    pub message: String,
}

impl From<std::io::Error> for ReportError {
    fn from(error: std::io::Error) -> Self {
        Self {
            kind: if error.kind() == std::io::ErrorKind::OutOfMemory {
                "memory"
            } else {
                "malformed"
            },
            message: error.to_string(),
        }
    }
}

impl ReportError {
    /// Refuse input before allocating or parsing it.
    pub fn refused(message: impl Into<String>) -> Self {
        Self {
            kind: "refused",
            message: message.into(),
        }
    }
}

/// Build a typed report from plain or gzipped SBML bytes.
pub fn report(source: &[u8]) -> Result<sbml4humans_report::Report, ReportError> {
    let decoded = decode(source, MAX_INPUT_BYTES)?;
    let source = decoded.as_ref();
    let document = sbml4humans_reader::read(source).map_err(|error| ReportError {
        kind: "malformed",
        message: error.to_string(),
    })?;
    sbml4humans_report::build(&document).map_err(|error| ReportError {
        kind: "malformed",
        message: error.to_string(),
    })
}

/// Build JSON bytes without reconstructing per-element host objects.
pub fn report_json(source: &[u8]) -> Result<Vec<u8>, ReportError> {
    serde_json::to_vec(&report(source)?).map_err(|error| ReportError {
        kind: "internal",
        message: error.to_string(),
    })
}

/// The shared wire contract, independent of host and reader implementation.
pub fn contract() -> &'static str {
    sbml4humans_abi::REPORT_CONTRACT
}

pub(crate) fn decode(source: &[u8], limit: usize) -> Result<Cow<'_, [u8]>, ReportError> {
    if source.len() > limit {
        return Err(ReportError::refused(format!(
            "The SBML source exceeds {limit} bytes."
        )));
    }
    if !source.starts_with(&[31, 139]) {
        return Ok(Cow::Borrowed(source));
    }
    let mut output = Vec::new();
    flate2::read::MultiGzDecoder::new(source)
        .take(limit.saturating_add(1) as u64)
        .read_to_end(&mut output)
        .map_err(|e| ReportError {
            kind: "malformed",
            message: format!("Invalid gzip SBML: {e}"),
        })?;
    if output.len() > limit {
        return Err(ReportError::refused(format!(
            "The decompressed SBML source exceeds {limit} bytes."
        )));
    }
    Ok(Cow::Owned(output))
}

#[cfg(test)]
mod tests {
    #[test]
    fn reports_keep_errors_out_of_the_success_payload() {
        let error = super::report_json(b"not XML").unwrap_err();
        assert_eq!(error.kind, "malformed");
        assert!(!error.message.contains("Traceback"));
    }
    #[test]
    fn the_pure_pipeline_includes_real_graph_and_unit_data() {
        let source = include_bytes!("../../../testdata/level2_graph_units.xml");
        let report: serde_json::Value =
            serde_json::from_slice(&super::report_json(source).unwrap()).unwrap();
        assert_eq!(
            report["models"][0]["listOfUnitDefinitions"][0]["unitsLatex"],
            "min"
        );
        assert!(!report["linkGraph"]["edges"].as_array().unwrap().is_empty());
        assert_eq!(super::contract(), "report/1");
    }
}
