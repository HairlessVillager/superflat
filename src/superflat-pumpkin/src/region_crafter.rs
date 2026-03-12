use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use rayon::prelude::*;

use crate::{chunk_data_to_sections_dump, delete_section_from_nbt, is_chunk_status_full};

const SECTOR_SIZE: usize = 4096;

#[derive(Debug)]
#[allow(dead_code)]
struct RegionFile {
    region_x: i32,
    region_z: i32,
    is_empty: bool,
    timestamp_header: Vec<u8>,
    chunkxz2nbt: HashMap<(i32, i32), Vec<u8>>,
}

fn extract_xz(filename: &str) -> Option<(i32, i32)> {
    let re = regex::Regex::new(r"\.(-?\d+)\.(-?\d+)\.").ok()?;
    let caps = re.captures(filename)?;
    let x = caps.get(1)?.as_str().parse().ok()?;
    let z = caps.get(2)?.as_str().parse().ok()?;
    Some((x, z))
}

fn read_region_file(
    region_filepath: &Path,
    region_x: i32,
    region_z: i32,
) -> Result<RegionFile, String> {
    let size = region_filepath
        .metadata()
        .map_err(|e| {
            format!(
                "Failed to read metadata for file {:?}: {}",
                region_filepath.as_os_str(),
                e
            )
        })?
        .len();
    if size == 0 {
        return Ok(RegionFile {
            region_x,
            region_z,
            is_empty: true,
            timestamp_header: vec![],
            chunkxz2nbt: HashMap::new(),
        });
    }

    let mut file = fs::File::open(region_filepath)
        .map_err(|e| format!("Failed to open region file: {}", e))?;

    use std::io::Read;

    let mut locations_raw = vec![0u8; 0x1000];
    let mut timestamps_raw = vec![0u8; 0x1000];

    file.read_exact(&mut locations_raw)
        .map_err(|e| format!("Failed to read locations: {}", e))?;
    file.read_exact(&mut timestamps_raw)
        .map_err(|e| format!("Failed to read timestamps: {}", e))?;

    let mut chunks = Vec::new();

    for i in 0..1024 {
        let x = (i % 32) as i32;
        let z = (i / 32) as i32;
        let chunk_x = region_x * 32 + x;
        let chunk_z = region_z * 32 + z;

        let loc_idx = i * 4;
        let offset = u32::from_be_bytes([
            0,
            locations_raw[loc_idx],
            locations_raw[loc_idx + 1],
            locations_raw[loc_idx + 2],
        ]) as usize;
        let size = locations_raw[loc_idx + 3] as usize;

        if offset == 0 && size == 0 {
            continue;
        }

        if offset < 2 {
            return Err(format!("Invalid sector offset {} at index {}", offset, i));
        }

        if size == 0 {
            return Err(format!("Invalid sector size 0 at index {}", i));
        }

        chunks.push((i, chunk_x, chunk_z, offset, size));
    }

    chunks.sort_by_key(|c| c.3);

    let mut chunkxz2nbt = HashMap::new();

    use std::io::Seek;

    for (_index, chunk_x, chunk_z, offset, size) in chunks {
        file.seek(std::io::SeekFrom::Start((offset * SECTOR_SIZE) as u64))
            .map_err(|e| format!("Failed to seek: {}", e))?;

        let mut raw = vec![0u8; size * SECTOR_SIZE];
        file.read_exact(&mut raw)
            .map_err(|e| format!("Failed to read chunk data: {}", e))?;

        let data_length = u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
        if data_length + 4 > raw.len() {
            return Err(format!(
                "Chunk length ({}) + 4 outside of sector size ({}) declared before in header at chunk ({}, {}) (offset: {:x})",
                data_length,
                size * SECTOR_SIZE,
                chunk_x,
                chunk_z,
                offset * SECTOR_SIZE,
            ));
        }
        let data = &raw[4..4 + data_length];
        let compression_type = data[0];
        let compressed_data = &data[1..];

        let data = match compression_type {
            2 => {
                use flate2::read::ZlibDecoder;
                let mut decoder = ZlibDecoder::new(compressed_data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed).map_err(|e| {
                    format!(
                        "Failed to decompress: {} at chunk ({}, {})",
                        e, chunk_x, chunk_z
                    )
                })?;
                decompressed
            }
            129 => {
                return Err(format!(
                    "MCC file format not supported at chunk ({}, {})",
                    chunk_x, chunk_z
                ));
            }
            _ => {
                return Err(format!(
                    "Unsupported compression type: {} at chunk ({}, {})",
                    compression_type, chunk_x, chunk_z
                ));
            }
        };

        let nbt = crate::normalize_nbt(&data).map_err(|e| {
            format!(
                "Failed to normalize NBT: {} at chunk ({}, {})",
                e, chunk_x, chunk_z
            )
        })?;

        chunkxz2nbt.insert((chunk_x, chunk_z), nbt);
    }

    Ok(RegionFile {
        region_x,
        region_z,
        is_empty: false,
        timestamp_header: timestamps_raw,
        chunkxz2nbt,
    })
}

fn write_bin(filepath: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = filepath.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(filepath, data)
}

#[pyfunction]
pub fn chunk_region_flatten<'py>(
    _py: Python<'py>,
    save_dir: PathBuf,
    repo_dir: PathBuf,
) -> PyResult<Vec<PathBuf>> {
    // Collect region files
    let mut region_files = Vec::new();

    for dimensions_dir in &["", "DIM1", "DIM-1"] {
        let region_dir = if dimensions_dir.is_empty() {
            save_dir.join("region")
        } else {
            save_dir.join(dimensions_dir).join("region")
        };

        if !region_dir.exists() {
            continue;
        }

        for entry in fs::read_dir(&region_dir)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to read directory: {}", e)))?
        {
            let entry = entry
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("mca") {
                let filename = path.file_name().unwrap().to_str().unwrap();
                if let Some((region_x, region_z)) = extract_xz(filename) {
                    let rel_path = path
                        .strip_prefix(&save_dir)
                        .map_err(|e| {
                            PyRuntimeError::new_err(format!("Failed to get relative path: {}", e))
                        })?
                        .to_path_buf();
                    region_files.push((rel_path, region_x, region_z));
                }
            }
        }
    }

    // Process region files in parallel
    // TODO: flatten two into_par_iter into only one for better pref
    let processed_paths = region_files
        .into_par_iter()
        .map(
            |(rel_path, region_x, region_z)| -> Result<Option<PathBuf>, String> {
                let region_filepath = save_dir.join(&rel_path);
                let region =
                    read_region_file(&region_filepath, region_x, region_z).map_err(|e| {
                        format!(
                            "Failed to read region file (path: {:?}): {}",
                            region_filepath.as_os_str(),
                            e,
                        )
                    })?;

                if region.is_empty {
                    return Ok(None);
                }

                // Write timestamp header
                let timestamp_path = repo_dir.join(&rel_path).join("timestamp-header");
                write_bin(&timestamp_path, &region.timestamp_header)
                    .map_err(|e| format!("Failed to write timestamp: {}", e))?;

                // Process and write chunks in parallel, propagating errors via try_for_each
                region
                    .chunkxz2nbt
                    .into_par_iter()
                    .filter(|(_, nbt)| is_chunk_status_full(nbt).unwrap_or(false))
                    .try_for_each(|((chunk_x, chunk_z), nbt)| -> Result<(), String> {
                        let sections_dump = {
                            let chunk_nbt_struct = pumpkin_nbt::from_bytes::<
                                pumpkin_world::chunk::format::ChunkNbt,
                            >(Cursor::new(&nbt))
                            .map_err(|e| format!("Failed to parse ChunkNbt: {}", e))?;
                            let chunk_data =
                                pumpkin_world::chunk::ChunkData::from_nbt(chunk_nbt_struct);
                            chunk_data_to_sections_dump(&chunk_data)
                        };
                        let other = delete_section_from_nbt(&nbt);

                        write_bin(
                            &repo_dir
                                .join(&rel_path)
                                .join("other")
                                .join(format!("c.{}.{}.nbt", chunk_x, chunk_z)),
                            &other,
                        )
                        .map_err(|e| format!("Failed to write other: {}", e))?;

                        write_bin(
                            &repo_dir
                                .join(&rel_path)
                                .join("sections")
                                .join(format!("c.{}.{}.delta", chunk_x, chunk_z)),
                            &sections_dump,
                        )
                        .map_err(|e| format!("Failed to write delta: {}", e))?;

                        Ok(())
                    })?;

                Ok(Some(rel_path))
            },
        )
        .collect::<Result<Vec<_>, String>>()
        .map_err(PyRuntimeError::new_err)?
        .into_iter()
        .flatten()
        .collect::<Vec<PathBuf>>();

    Ok(processed_paths)
}

#[pyfunction]
pub fn _chunk_region_unflatten<'py>(
    _py: Python<'py>,
    _save_dir: &str,
    _repo_dir: &str,
    _dumper_get: Bound<'py, PyAny>,
    _dumper_compressed: bool,
) -> PyResult<Vec<String>> {
    // TODO: Implement unflatten
    Ok(vec![])
}
