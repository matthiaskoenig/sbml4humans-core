//! Native byte-to-byte parsing keeps Python out of the per-element report loop.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Build report JSON while releasing the Python interpreter lock.
#[pyfunction]
fn report_json<'py>(
    py: Python<'py>,
    source: &Bound<'py, PyBytes>,
) -> PyResult<Bound<'py, PyBytes>> {
    let bytes = source.as_bytes();
    let output = py
        .detach(|| sbml4humans_core::report_json(bytes))
        .map_err(|error| PyValueError::new_err(error.message))?;
    Ok(PyBytes::new(py, &output))
}

/// Build the full source envelope from plain/gzipped SBML or OMEX bytes.
#[pyfunction(name = "_source_json")]
fn source_json<'py>(
    py: Python<'py>,
    source: &Bound<'py, PyBytes>,
    options: &str,
) -> PyResult<Bound<'py, PyBytes>> {
    let options = serde_json::from_str::<sbml4humans_core::SourceOptions>(options)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let bytes = source.as_bytes();
    let output = py
        .detach(|| {
            let start = std::time::Instant::now();
            let mut response = sbml4humans_core::source(bytes, &options)?;
            let elapsed = format!("{:.3} [s]", start.elapsed().as_secs_f64());
            for report in response.reports.values_mut() {
                report.debug.json_report_time = elapsed.clone();
            }
            serde_json::to_vec(&response).map_err(|e| sbml4humans_core::ReportError {
                kind: "internal",
                message: e.to_string(),
            })
        })
        .map_err(|error| {
            let exception = PyValueError::new_err(error.message);
            // ValueError preserves the public error type while hosts can distinguish limits.
            let _ = exception.value(py).setattr("kind", error.kind);
            exception
        })?;
    Ok(PyBytes::new(py, &output))
}

/// Native module loaded by the Python package.
#[pymodule]
fn _core(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add("REPORT_CONTRACT", sbml4humans_core::contract())?;
    module.add_function(wrap_pyfunction!(report_json, module)?)?;
    module.add_function(wrap_pyfunction!(source_json, module)?)?;
    module.add("SOURCE_CONTRACT", "source/1")?;
    Ok(())
}
