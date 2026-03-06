use pyo3::prelude::*;

#[pymodule]
mod xdelta3_py {
    use pyo3::{exceptions::PyRuntimeError, prelude::*, types::PyBytes};

    #[pyfunction]
    fn encode<'py>(py: Python<'py>, base: &[u8], patched: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = xdelta3::encode(base, patched)
            .map_err(|e| PyRuntimeError::new_err(format!("{:?}", e)))?;
        Ok(PyBytes::new(py, &bytes))
    }

    #[pyfunction]
    fn decode<'py>(
        py: Python<'py>,
        base: &[u8],
        difference: &[u8],
    ) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = xdelta3::decode(base, difference)
            .map_err(|e| PyRuntimeError::new_err(format!("{:?}", e)))?;
        Ok(PyBytes::new(py, &bytes))
    }
}
