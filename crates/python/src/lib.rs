//! Native byte-to-byte parsing keeps Python out of the per-element report loop.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

#[cfg(feature = "validation")]
mod validation;

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
    let options = parse_options(options)?;
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
        .map_err(source_error)?;
    Ok(PyBytes::new(py, &output))
}

#[derive(serde::Deserialize)]
pub(crate) struct HostOptions {
    #[serde(flatten)]
    pub(crate) source: sbml4humans_core::SourceOptions,
    pub(crate) entry: Option<String>,
}
pub(crate) fn host_options(options: &str) -> PyResult<HostOptions> {
    serde_json::from_str(options).map_err(|e| PyValueError::new_err(e.to_string()))
}

fn parse_options(options: &str) -> PyResult<sbml4humans_core::SourceOptions> {
    serde_json::from_str(options).map_err(|e| PyValueError::new_err(e.to_string()))
}
fn source_error(error: sbml4humans_core::ReportError) -> PyErr {
    let exception = PyValueError::new_err(error.message);
    Python::attach(|py| {
        let _ = exception.value(py).setattr("kind", error.kind);
    });
    exception
}

/// Read trusted or untrusted paths through the shared native source pipeline.
#[pyfunction(name = "_source_path_json")]
fn source_path_json<'py>(
    py: Python<'py>,
    path: &str,
    trusted: bool,
    options: &str,
) -> PyResult<Bound<'py, PyBytes>> {
    let options = parse_options(options)?;
    let output = py
        .detach(|| {
            let start = std::time::Instant::now();
            let prepared =
                sbml4humans_core::prepare_path(std::path::Path::new(path), trusted, &options)?;
            let mut response = prepared.report(&options)?;
            let elapsed = format!("{:.3} [s]", start.elapsed().as_secs_f64());
            for report in response.reports.values_mut() {
                report.debug.json_report_time = elapsed.clone();
            }
            serde_json::to_vec(&response).map_err(|e| sbml4humans_core::ReportError {
                kind: "internal",
                message: e.to_string(),
            })
        })
        .map_err(source_error)?;
    Ok(PyBytes::new(py, &output))
}

#[derive(serde::Deserialize)]
struct ExportRequest {
    source: sbml4humans_core::SourceOptions,
    format: String,
    entry: Option<String>,
    simulator: bool,
    #[serde(rename = "sourceEntry")]
    source_entry: Option<String>,
}

/// Source admission and ODE export stay inside the same native pipeline.
#[pyfunction(name = "_ode_path_json")]
fn ode_path_json<'py>(
    py: Python<'py>,
    path: &str,
    trusted: bool,
    request: &str,
) -> PyResult<Bound<'py, PyBytes>> {
    let request: ExportRequest =
        serde_json::from_str(request).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let output = py
        .detach(|| {
            let prepared = sbml4humans_core::prepare_host_path(
                std::path::Path::new(path),
                trusted,
                &request.source,
                request.source_entry.as_deref(),
            )?;
            let exported = prepared.export(
                &request.format,
                request.entry.as_deref(),
                request.simulator,
                request.source.max_expanded_elements,
            )?;
            serde_json::to_vec(&exported).map_err(|e| sbml4humans_core::ReportError {
                kind: "internal",
                message: e.to_string(),
            })
        })
        .map_err(source_error)?;
    Ok(PyBytes::new(py, &output))
}

/// Native report transport keeps serialization, compression and diagnostics in Rust.
#[pyfunction]
fn report_path<'py>(
    py: Python<'py>,
    path: &str,
    trusted: bool,
    options: &str,
    gzip: bool,
) -> PyResult<(Bound<'py, PyBytes>, Vec<String>)> {
    let options = host_options(options)?;
    let output = py
        .detach(|| {
            sbml4humans_core::report_path(
                std::path::Path::new(path),
                trusted,
                &options.source,
                gzip,
                options.entry.as_deref(),
            )
        })
        .map_err(source_error)?;
    Ok((PyBytes::new(py, &output.content), output.diagnostics))
}
/// Lightweight example metadata without report construction or archive extraction.
#[pyfunction]
fn example_json<'py>(
    py: Python<'py>,
    path: &str,
    mode: &str,
    example_id: Option<&str>,
) -> PyResult<Bound<'py, PyBytes>> {
    let output = py
        .detach(|| {
            let summary =
                sbml4humans_core::example_summary(std::path::Path::new(path), mode, example_id)?;
            serde_json::to_vec(&summary).map_err(|e| sbml4humans_core::ReportError {
                kind: "internal",
                message: e.to_string(),
            })
        })
        .map_err(source_error)?;
    Ok(PyBytes::new(py, &output))
}

/// Native module loaded by the Python package.
#[pymodule]
fn _core(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("HOST_CONTRACT", "host/1")?;
    module.add(
        "SOURCE_REVISION",
        option_env!("SBML4HUMANS_CORE_REVISION").unwrap_or("unversioned"),
    )?;
    module.add_function(wrap_pyfunction!(report_path, module)?)?;
    module.add_function(wrap_pyfunction!(example_json, module)?)?;
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add("REPORT_CONTRACT", sbml4humans_core::contract())?;
    module.add_function(wrap_pyfunction!(report_json, module)?)?;
    module.add_function(wrap_pyfunction!(source_json, module)?)?;
    module.add("SOURCE_CONTRACT", "source/1")?;
    module.add("SOURCE_PATH_CONTRACT", "source-path/1")?;
    module.add("ODE_SOURCE_CONTRACT", "ode-source/1")?;
    module.add_function(wrap_pyfunction!(ode_path_json, module)?)?;
    module.add_function(wrap_pyfunction!(source_path_json, module)?)?;
    #[cfg(feature = "validation")]
    {
        module.add_class::<validation::ValidationSession>()?;
        module.add_class::<validation::SourceValidation>()?;
        module.add_function(wrap_pyfunction!(validation::rule_json, module)?)?;
        module.add_function(wrap_pyfunction!(validation::rule_package, module)?)?;
        module.add_function(wrap_pyfunction!(validation::validation_source, module)?)?;
        module.add("VALIDATION_SOURCE_CONTRACT", "validation-source/1")?;
        module.add("VALIDATION_CONTRACT", "validation/1")?;
    }
    Ok(())
}
