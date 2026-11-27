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

/// Native module loaded by the Python package.
#[pymodule]
fn _core(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add("REPORT_CONTRACT", sbml4humans_core::contract())?;
    module.add_function(wrap_pyfunction!(report_json, module)?)?;
    Ok(())
}
