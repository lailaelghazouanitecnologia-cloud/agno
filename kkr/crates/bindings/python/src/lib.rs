//! Python bindings for KKR

use pyo3::prelude::*;

/// KKR Python module
#[pymodule]
fn kkr(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
