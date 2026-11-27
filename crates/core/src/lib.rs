//! One report pipeline for the CLI, native Python module and browser worker.

use serde::Serialize;
use std::io::Read;

/// The maximum uncompressed XML input accepted by the default host API.
pub const MAX_INPUT_BYTES: usize = 100 * 1024 * 1024;

/// A report failure with an explicit category shared by every host.
#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct ReportError {
    /// `malformed`, `refused` or `internal`, suitable for host-side routing.
    pub kind: &'static str,
    /// The human-readable cause without an operating-system traceback.
    pub message: String,
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
    if source.len() > MAX_INPUT_BYTES {
        return Err(ReportError::refused(format!(
            "The SBML source exceeds {MAX_INPUT_BYTES} bytes."
        )));
    }
    let decoded;
    let source = if source.starts_with(&[0x1f, 0x8b]) {
        let mut output = Vec::new();
        flate2::read::MultiGzDecoder::new(source)
            .take(MAX_INPUT_BYTES as u64 + 1)
            .read_to_end(&mut output)
            .map_err(|e| ReportError {
                kind: "malformed",
                message: format!("Invalid gzip SBML: {e}"),
            })?;
        if output.len() > MAX_INPUT_BYTES {
            return Err(ReportError::refused(format!(
                "The decompressed SBML source exceeds {MAX_INPUT_BYTES} bytes."
            )));
        }
        decoded = output;
        decoded.as_slice()
    } else {
        source
    };
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
