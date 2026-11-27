//! Thin Python ownership of a native libSBML validation session.
use std::sync::Mutex;

use pyo3::exceptions::{PyMemoryError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sbml4humans_libsbml::{ElementPositions, ValidationContent, ValidationError, ValidationSource};

fn error(error: ValidationError) -> PyErr {
    match error {
        ValidationError::Memory => PyMemoryError::new_err(error.to_string()),
        other => PyValueError::new_err(other.to_string()),
    }
}

/// Native documents reused while entries are checked under the host's worker budget.
#[pyclass]
pub(crate) struct ValidationSession {
    inner: Mutex<sbml4humans_libsbml::ValidationSession>,
}

#[pymethods]
impl ValidationSession {
    #[new]
    fn new(py: Python<'_>, sources: Vec<(String, String, String, String)>) -> PyResult<Self> {
        let sources = sources
            .into_iter()
            .map(|(location, path, uri, positions)| {
                let positions: ElementPositions = serde_json::from_str(&positions)
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
                Ok(ValidationSource {
                    location,
                    content: ValidationContent::Path(path.into()),
                    uri,
                    positions,
                })
            })
            .collect::<PyResult<Vec<_>>>()?;
        let session = py
            .detach(|| sbml4humans_libsbml::ValidationSession::new(sources))
            .map_err(error)?;
        Ok(Self {
            inner: Mutex::new(session),
        })
    }

    /// Return diagnostics as serialized bytes, releasing the GIL for native validation.
    #[pyo3(signature = (location, *, check_consistency=true))]
    fn issues_json<'py>(
        &self,
        py: Python<'py>,
        location: &str,
        check_consistency: bool,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let output = py
            .detach(|| {
                let mut session = self
                    .inner
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let issues = session.issues(location, check_consistency)?;
                serde_json::to_vec(&issues).map_err(|e| ValidationError::Invalid(e.to_string()))
            })
            .map_err(error)?;
        Ok(PyBytes::new(py, &output))
    }
}

/// Validation of an admitted source, including its keys and comp work gate.
#[pyclass]
pub(crate) struct SourceValidation {
    inner: Mutex<sbml4humans_libsbml::ValidationSession>,
    locations: Vec<String>,
    skipped: std::collections::HashSet<String>,
}

#[pymethods]
impl SourceValidation {
    #[getter]
    fn locations(&self) -> Vec<String> {
        self.locations.clone()
    }

    /// Check one entry so the host can preserve completed results on interruption.
    fn entry_json<'py>(&self, py: Python<'py>, location: &str) -> PyResult<Bound<'py, PyBytes>> {
        let output = py.detach(|| {
            let mut session = self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let normalized = sbml4humans_core::normalize_location(location);
                let skipped = normalized.as_deref().is_some_and(|location| self.skipped.contains(location));
            let issues = session.issues(location, !skipped)?;
            serde_json::to_vec(&serde_json::json!({"issues": issues, "skipped": if skipped { Some("expandedSize") } else { None }}))
                .map_err(|e| ValidationError::Invalid(e.to_string()))
        }).map_err(error)?;
        Ok(PyBytes::new(py, &output))
    }
}

/// Admit source bytes, derive report positions and open the native documents in Rust.
#[pyfunction(name = "_validation_source")]
pub(crate) fn validation_source(
    py: Python<'_>,
    path: &str,
    trusted: bool,
    options: &str,
) -> PyResult<SourceValidation> {
    let options = crate::host_options(options)?;
    py.detach(|| {
        let prepared = sbml4humans_core::prepare_host_path(
            std::path::Path::new(path),
            trusted,
            &options.source,
            options.entry.as_deref(),
        )
        .map_err(crate::source_error)?;
        let indices = prepared.indices().map_err(crate::source_error)?;
        let locations = prepared
            .entries
            .iter()
            .map(|entry| entry.location.clone())
            .collect();
        let mut skipped = std::collections::HashSet::new();
        // libSBML's file reader preserves original encoding diagnostics. Keep all
        // temporary files inside the host worker's TMPDIR and remove them after reading.
        let directory = tempfile::tempdir().map_err(|e| PyValueError::new_err(e.to_string()))?;
        let mut sources = Vec::with_capacity(prepared.entries.len());
        for (number, (entry, index)) in prepared.entries.into_iter().zip(indices).enumerate() {
            if index
                .expanded_size
                .is_some_and(|size| size > options.source.max_expanded_elements as u128)
            {
                skipped.insert(
                    sbml4humans_core::normalize_location(&entry.location)
                        .expect("prepared locations are normalized"),
                );
            }
            let path = directory.path().join(format!("{number}.xml"));
            std::fs::write(&path, entry.xml).map_err(|e| PyValueError::new_err(e.to_string()))?;
            sources.push(ValidationSource {
                location: entry.location.clone(),
                uri: directory
                    .path()
                    .join(entry.location.trim_start_matches("./"))
                    .to_string_lossy()
                    .into_owned(),
                content: ValidationContent::Path(path),
                positions: ElementPositions {
                    document_pk: "document/SBMLDocument:document".into(),
                    elements: index
                        .positions
                        .into_iter()
                        .map(|(line, column, key)| (line, column, 0, key))
                        .collect(),
                },
            });
        }
        let session =
            sbml4humans_libsbml::ValidationSession::from_report(sources).map_err(error)?;
        Ok(SourceValidation {
            inner: Mutex::new(session),
            locations,
            skipped,
        })
    })
}

/// Resolve rule documentation through the validator's own tables.
#[pyfunction]
pub(crate) fn rule_json<'py>(py: Python<'py>, code: u32) -> PyResult<Bound<'py, PyBytes>> {
    let output = py
        .detach(|| {
            let rule = sbml4humans_libsbml::validation_rule(code)?;
            serde_json::to_vec(&rule).map_err(|e| ValidationError::Invalid(e.to_string()))
        })
        .map_err(error)?;
    Ok(PyBytes::new(py, &output))
}
/// Identify the specification a validation rule belongs to.
#[pyfunction]
pub(crate) fn rule_package(py: Python<'_>, code: u32) -> PyResult<String> {
    py.detach(|| sbml4humans_libsbml::rule_package(code))
        .map_err(error)
}
