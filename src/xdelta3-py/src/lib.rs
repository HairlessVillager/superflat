use pyo3::prelude::*;

#[pymodule]
mod xdelta3_py {
    use pyo3::{exceptions::PyRuntimeError, prelude::*, types::PyBytes};

    #[pyfunction]
    fn encode<'py>(py: Python<'py>, base: &[u8], target: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = xdelta3::encode(target, base)
            .map_err(|e| PyRuntimeError::new_err(format!("{:?}", e)))?;
        Ok(PyBytes::new(py, &bytes))
    }

    #[pyfunction]
    fn decode<'py>(py: Python<'py>, base: &[u8], delta: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = xdelta3::decode(delta, base)
            .map_err(|e| PyRuntimeError::new_err(format!("{:?}", e)))?;
        Ok(PyBytes::new(py, &bytes))
    }
}
