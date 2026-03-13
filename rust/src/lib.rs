use pyo3::prelude::*;

mod _pumpkin;

#[pymodule]
mod pumpkin {
    use pyo3::{exceptions::PyRuntimeError, prelude::*, types::PyBytes};

    use std::{collections::HashMap, path::PathBuf};

    use super::_pumpkin;

    #[pyfunction]
    fn chunk_region_flatten(
        save_dir: PathBuf,
        repo_dir: PathBuf,
        block_id_mapping: HashMap<String, String>,
    ) -> PyResult<Vec<PathBuf>> {
        let block_id_mapping = block_id_mapping
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<HashMap<_, _>>();
        _pumpkin::region_crafter::chunk_region_flatten(&save_dir, &repo_dir, &block_id_mapping)
            .map_err(|e| PyRuntimeError::new_err(e))
    }

    #[pyfunction]
    fn chunk_region_unflatten(
        save_dir: PathBuf,
        repo_dir: PathBuf,
    ) -> PyResult<Vec<PathBuf>> {
        _pumpkin::region_crafter::chunk_region_unflatten(&save_dir, &repo_dir)
            .map_err(|e| PyRuntimeError::new_err(e))
    }

    #[pyfunction]
    pub fn normalize_nbt<'py>(nbt: &[u8]) -> PyResult<Vec<u8>> {
        let bytes: Vec<u8> = _pumpkin::normalize::normalize_nbt_bytes(&nbt)
            .map(|v| v.into())
            .map_err(|e| PyRuntimeError::new_err(e))?;
        Ok(bytes)
    }

    #[pyfunction]
    fn chunk_region_decode_batch(
        others: Vec<Bound<'_, PyBytes>>,
        sections_deltas: Vec<Bound<'_, PyBytes>>,
        sections_dumps: Vec<Bound<'_, PyBytes>>,
        compressed: bool,
    ) -> PyResult<Vec<Vec<u8>>> {
        let others: Vec<&[u8]> = others.iter().map(|e| e.as_bytes()).collect();
        let sections_deltas: Vec<&[u8]> = sections_deltas.iter().map(|e| e.as_bytes()).collect();
        let sections_dumps: Vec<&[u8]> = sections_dumps.iter().map(|e| e.as_bytes()).collect();
        _pumpkin::region_decode::chunk_region_decode_batch(
            others,
            sections_deltas,
            sections_dumps,
            compressed,
        )
        .map_err(|e| PyRuntimeError::new_err(e))
    }

    #[pyfunction]
    fn is_chunk_status_full(input: &[u8]) -> PyResult<bool> {
        _pumpkin::check_chunk_status_full(input).map_err(|e| PyRuntimeError::new_err(e))
    }
}

#[pymodule]
mod _superflat {
    use pyo3::prelude::*;

    #[pymodule_export]
    use super::pumpkin;

    #[pyfunction]
    fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
        Ok((a + b).to_string())
    }
}
