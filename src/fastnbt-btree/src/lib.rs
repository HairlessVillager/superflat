use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
mod fastnbt_btree {
    use fastnbt::Value;
    use pyo3::exceptions::{PyRuntimeError, PyValueError};
    use pyo3::prelude::*;
    use pyo3::types::PyBytes;

    #[pyfunction]
    fn normalize<'py>(py: Python<'py>, input: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
        let v: Value =
            fastnbt::from_bytes(input).map_err(|err| PyValueError::new_err(err.to_string()))?;
        let bytes =
            fastnbt::to_bytes(&v).map_err(|err| PyRuntimeError::new_err(err.to_string()))?;
        let output = PyBytes::new(py, &bytes);
        Ok(output)
    }
}
