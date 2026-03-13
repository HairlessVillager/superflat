use pyo3::prelude::*;

mod pack;

/// Collect Python UnpackedObject-like records into Rust tuples.
///
/// Each item must expose:
///   - `pack_type_num: int`
///   - `sha() -> bytes`
///   - `delta_base: bytes | None`
///   - `decomp_chunks: list[bytes]`
fn collect_records(
    records: &Bound<'_, PyAny>,
    capacity: usize,
) -> PyResult<Vec<(u8, Vec<u8>, Option<Vec<u8>>, Vec<u8>)>> {
    let mut out = Vec::with_capacity(capacity);
    for item_result in records.try_iter()? {
        let item = item_result?;
        let type_num: u8 = item.getattr("pack_type_num")?.extract()?;
        let sha: Vec<u8> = item.call_method0("sha")?.extract()?;
        let db_obj = item.getattr("delta_base")?;
        let delta_base: Option<Vec<u8>> = if db_obj.is_none() {
            None
        } else {
            Some(db_obj.extract()?)
        };
        let decomp_chunks: Vec<Vec<u8>> = item.getattr("decomp_chunks")?.extract()?;
        let raw_data: Vec<u8> = decomp_chunks.into_iter().flatten().collect();
        out.push((type_num, sha, delta_base, raw_data));
    }
    Ok(out)
}

/// Write pack data to `path`.
///
/// `records` is an iterable of objects with attributes:
///   - `pack_type_num: int`
///   - `sha() -> bytes`
///   - `delta_base: bytes | None`
///   - `decomp_chunks: list[bytes]`
///
/// Returns `(entries: dict[bytes, tuple[int, int]], pack_checksum: bytes)`.
/// `entries` maps each object SHA to `(pack_offset, crc32)`.
#[pyfunction]
#[pyo3(signature = (path, records, num_records, compression_level = -1))]
fn write_pack_data(
    py: Python<'_>,
    path: &str,
    records: &Bound<'_, PyAny>,
    num_records: usize,
    compression_level: i32,
) -> PyResult<(Py<pyo3::types::PyDict>, Vec<u8>)> {
    use pyo3::types::{PyBytes, PyDict};
    let rust_records = collect_records(records, num_records)?;
    let (entries_list, pack_checksum) = pack::write_pack_data_raw(
        path,
        rust_records.into_iter(),
        num_records,
        compression_level,
    )
    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    let py_entries = PyDict::new(py);
    for e in &entries_list {
        py_entries.set_item(PyBytes::new(py, &e.sha), (e.offset, e.crc32))?;
    }
    Ok((py_entries.unbind(), pack_checksum.to_vec()))
}

/// Write a pack index v2 file to `path`.
///
/// `entries` must be a list of `(sha: bytes, offset: int, crc32: int)` tuples,
/// sorted by SHA (ascending).
///
/// Returns the 20-byte index checksum.
#[pyfunction]
fn write_pack_index_v2(
    path: &str,
    entries: Vec<(Vec<u8>, u64, u32)>,
    pack_checksum: Vec<u8>,
) -> PyResult<Vec<u8>> {
    let rust_entries: Vec<pack::PackEntry> = entries
        .into_iter()
        .map(|(sha, offset, crc32)| pack::PackEntry { sha, offset, crc32 })
        .collect();
    pack::write_pack_index_v2_raw(&path, &rust_entries, &pack_checksum)
        .map(|cs: [u8; 20]| cs.to_vec())
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

/// Write both a pack file (`filename`.pack) and an index file (`filename`.idx).
///
/// `records` is an iterable of objects with attributes:
///   - `pack_type_num: int`
///   - `sha() -> bytes`
///   - `delta_base: bytes | None`
///   - `decomp_chunks: list[bytes]`
///
/// Returns `(pack_checksum: bytes, index_checksum: bytes)`.
#[pyfunction]
#[pyo3(signature = (filename, records, num_records, compression_level = -1))]
fn write_pack(
    filename: &str,
    records: &Bound<'_, PyAny>,
    num_records: usize,
    compression_level: i32,
) -> PyResult<(Vec<u8>, Vec<u8>)> {
    let pack_path = format!("{filename}.pack");
    let idx_path = format!("{filename}.idx");

    let rust_records = collect_records(records, num_records)?;
    let (mut entries_list, pack_checksum) = pack::write_pack_data_raw(
        &pack_path,
        rust_records.into_iter(),
        num_records,
        compression_level,
    )
    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    // Index requires entries sorted by SHA.
    entries_list.sort_by(|a, b| a.sha.cmp(&b.sha));

    let idx_checksum =
        pack::write_pack_index_v2_raw(&idx_path, &entries_list, &pack_checksum)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    Ok((pack_checksum.to_vec(), idx_checksum.to_vec()))
}

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = pyo3::types::PyModule::new(parent.py(), "superflat_repack")?;
    m.add_function(pyo3::wrap_pyfunction!(write_pack_data, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(write_pack_index_v2, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(write_pack, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
