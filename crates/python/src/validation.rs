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
