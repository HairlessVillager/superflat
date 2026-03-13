use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::ZlibEncoder;
use rayon::prelude::*;

use super::normalize::{apply_block_id_mapping, normalize_nbt_bytes_mapping};
use super::{check_chunk_status_full, delete_section_from_nbt};

const SECTOR_SIZE: usize = 4096;

use pumpkin_nbt::{Nbt, deserializer::NbtReadHelper, from_bytes, tag::NbtTag, to_bytes};
use pumpkin_world::chunk::format::{ChunkNbt, ChunkSectionNBT};
use pumpkin_world::chunk::{ChunkData, ChunkSections};

pub fn restore_chunk_from_raw(sections_dump: &[u8], other: &[u8]) -> Vec<u8> {
    let sections = from_bytes::<crate::_pumpkin::SectionsDump>(Cursor::new(sections_dump))
        .expect("Failed to load sections");
    let other =
        Nbt::read(&mut NbtReadHelper::new(Cursor::new(other))).expect("Failed to load other");

    let mut chunk = other;

    // load sections
    let sections = {
        let section =
            ChunkSections::from_blocks_biomes(&sections.blocks_dump, &sections.biomes_dump);
        let block_lock = section.block_sections.read().unwrap();
        let biome_lock = section.biome_sections.read().unwrap();
        let min_section_y = (section.min_y >> 4) as i8;

        (0..section.count)
            .map(|i| ChunkSectionNBT {
                y: i as i8 + min_section_y,
                block_states: Some(block_lock[i].to_disk_nbt()),
                biomes: Some(biome_lock[i].to_disk_nbt()),

                // drop block & sky lighting because Minecraft will re-compute them
                block_light: None,
                sky_light: None,
            })
            .map(|nbt| {
                let mut bytes: Vec<u8> = Vec::new();
                to_bytes(&nbt, &mut bytes).expect("Failed to serialize ChunkSectionNBT to bytes");
                let nbt = Nbt::read(&mut NbtReadHelper::new(Cursor::new(bytes)))
                    .expect("Failed to build NBT from ChunkSectionNBT bytes");
                NbtTag::Compound(nbt.root_tag)
            })
            .collect::<Vec<_>>()
    };

    // insert to other nbt
    chunk.root_tag.put_list("sections", sections);

    chunk.write().into()
}

fn chunk_data_to_sections_dump(chunk_data: &ChunkData) -> Vec<u8> {
    let dump = super::SectionsDump {
        biomes_dump: chunk_data.section.dump_biomes(),
        blocks_dump: chunk_data.section.dump_blocks(),
    };
    let mut buf = Vec::new(); // TODO: use .with_capacity here
    to_bytes(&dump, &mut buf).expect("Failed to dump thin data");
    super::normalize::normalize_nbt_bytes(&buf)
        .expect("Failed to normalize sections dump")
        .into()
}

fn normalize_chunk_nbt(nbt: &[u8], mapping: &HashMap<&str, &str>) -> Result<Vec<u8>, String> {
    let bytes: Vec<u8> = normalize_nbt_bytes_mapping(&nbt, |v| apply_block_id_mapping(v, mapping))
        .map(|v| v.into())?;
    Ok(bytes)
}

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
    block_id_mapping: &HashMap<&str, &str>,
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

        let nbt = normalize_chunk_nbt(&data, block_id_mapping).map_err(|e| {
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
    // not write in test
    if cfg!(test) {
        return std::io::Result::Ok(());
    }

    if let Some(parent) = filepath.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(filepath, data)
}

pub fn chunk_region_flatten(
    save_dir: &PathBuf,
    repo_dir: &PathBuf,
    block_id_mapping: &HashMap<&str, &str>,
) -> Result<Vec<PathBuf>, String> {
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

        for entry in
            fs::read_dir(&region_dir).map_err(|e| format!("Failed to read directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("mca") {
                let filename = path.file_name().unwrap().to_str().unwrap();
                if let Some((region_x, region_z)) = extract_xz(filename) {
                    let rel_path = path
                        .strip_prefix(&save_dir)
                        .map_err(|e| format!("Failed to get relative path: {}", e))?
                        .to_path_buf();
                    region_files.push((rel_path, region_x, region_z));
                    eprintln!("collected {:?}", path.as_os_str());
                }
            }
        }
    }

    // Process region files in parallel
    // TODO: flatten two into_par_iter into only one for better pref
    // TODO: use cooler and more readable stderr log
    let processed_paths = region_files
        .into_par_iter()
        .map(
            |(rel_path, region_x, region_z)| -> Result<Option<PathBuf>, String> {
                eprintln!("process {:?}", rel_path.as_os_str());
                let region_filepath = save_dir.join(&rel_path);
                let region =
                    read_region_file(&region_filepath, region_x, region_z, &block_id_mapping)
                        .map_err(|e| {
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
                    .filter(|(_, nbt)| check_chunk_status_full(nbt).unwrap_or(false))
                    .try_for_each(|((chunk_x, chunk_z), nbt)| -> Result<(), String> {
                        eprintln!("process chunk ({}, {})", chunk_x, chunk_z);
                        let sections_dump = {
                            let chunk_nbt_struct = from_bytes::<ChunkNbt>(Cursor::new(&nbt))
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
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<PathBuf>>();

    Ok(processed_paths)
}

fn write_region_file(
    region_filepath: &Path,
    region_x: i32,
    region_z: i32,
    timestamp_header: &[u8],
    chunkxz2nbt: &HashMap<(i32, i32), Vec<u8>>,
) -> Result<(), String> {
    use std::io::Write;

    if timestamp_header.len() != SECTOR_SIZE {
        return Err(format!(
            "Invalid timestamp header length: {} != {}",
            timestamp_header.len(),
            SECTOR_SIZE,
        ));
    }

    let mut locations = vec![0u8; SECTOR_SIZE];
    let mut chunk_data_buffer: Vec<u8> = Vec::new();
    let mut current_sector: usize = 2;

    // Sort chunks for deterministic output
    let mut chunks: Vec<_> = chunkxz2nbt.iter().collect();
    chunks.sort_by_key(|((cx, cz), _)| (*cx, *cz));

    for ((chunk_x, chunk_z), nbt) in chunks {
        let local_x = chunk_x - region_x * 32;
        let local_z = chunk_z - region_z * 32;
        if !(0..32).contains(&local_x) || !(0..32).contains(&local_z) {
            return Err(format!(
                "Chunk outside region boundary: chunk_x={}, chunk_z={}",
                chunk_x, chunk_z
            ));
        }
        let index = (local_x + local_z * 32) as usize;

        // zlib compress
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(nbt)
            .map_err(|e| format!("Failed to compress chunk nbt: {}", e))?;
        let compressed = encoder
            .finish()
            .map_err(|e| format!("Failed to finish zlib compression: {}", e))?;

        let content_length = compressed.len() + 1; // +1 for compression type byte
        let mut chunk_payload = Vec::with_capacity(4 + content_length);
        chunk_payload.extend_from_slice(&(content_length as u32).to_be_bytes());
        chunk_payload.push(2u8); // zlib compression
        chunk_payload.extend_from_slice(&compressed);

        let sectors_needed = chunk_payload.len().div_ceil(SECTOR_SIZE);
        if sectors_needed >= 256 {
            return Err(format!(
                "Chunk too large: {} sectors needed (>= 256) at ({}, {})",
                sectors_needed, chunk_x, chunk_z
            ));
        }

        // Update location header
        let loc_offset = index * 4;
        let sector_bytes = current_sector.to_be_bytes();
        locations[loc_offset] = sector_bytes[5];
        locations[loc_offset + 1] = sector_bytes[6];
        locations[loc_offset + 2] = sector_bytes[7];
        locations[loc_offset + 3] = sectors_needed as u8;

        // Write chunk data padded to sector boundary
        let padded_size = sectors_needed * SECTOR_SIZE;
        chunk_data_buffer.extend_from_slice(&chunk_payload);
        chunk_data_buffer.resize(
            chunk_data_buffer.len() + padded_size - chunk_payload.len(),
            0,
        );

        current_sector += sectors_needed;
    }

    let mut content = Vec::with_capacity(2 * SECTOR_SIZE + chunk_data_buffer.len());
    content.extend_from_slice(&locations);
    content.extend_from_slice(timestamp_header);
    content.extend_from_slice(&chunk_data_buffer);

    if let Some(parent) = region_filepath.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directories: {}", e))?;
    }
    fs::write(region_filepath, &content)
        .map_err(|e| format!("Failed to write region file: {}", e))?;

    Ok(())
}

// TODO: use cooler and more readable stderr log
pub fn chunk_region_unflatten(
    save_dir: &PathBuf,
    repo_dir: &PathBuf,
) -> Result<Vec<PathBuf>, String> {
    // Collect timestamp-header files across region dirs
    let mut region_dirs: Vec<(PathBuf, i32, i32)> = Vec::new();

    for dimensions_dir in &["", "DIM1", "DIM-1"] {
        let region_dir = if dimensions_dir.is_empty() {
            repo_dir.join("region")
        } else {
            repo_dir.join(dimensions_dir).join("region")
        };

        if !region_dir.exists() {
            continue;
        }

        for entry in
            fs::read_dir(&region_dir).map_err(|e| format!("Failed to read directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dirname = path.file_name().unwrap().to_str().unwrap();
            if let Some((region_x, region_z)) = extract_xz(dirname) {
                let timestamp_path = path.join("timestamp-header");
                if timestamp_path.exists() {
                    let rel_path = path
                        .strip_prefix(repo_dir)
                        .map_err(|e| format!("Failed to get relative path: {}", e))?
                        .to_path_buf();
                    region_dirs.push((rel_path, region_x, region_z));
                }
            }
        }
    }

    let processed_paths = region_dirs
        .into_par_iter()
        .map(
            |(rel_path, region_x, region_z)| -> Result<Vec<PathBuf>, String> {
                let mut processed_paths = Vec::with_capacity(1024 + 1024 + 1);
                let region_repo_dir = repo_dir.join(&rel_path);
                let timestamp_header_filepath = region_repo_dir.join("timestamp-header");
                let timestamp_header = fs::read(&timestamp_header_filepath)
                    .map_err(|e| format!("Failed to read timestamp-header: {}", e))?;
                processed_paths.push(
                    timestamp_header_filepath
                        .strip_prefix(repo_dir)
                        .unwrap()
                        .to_path_buf(),
                );

                let mut chunkxz2nbt: HashMap<(i32, i32), Vec<u8>> = HashMap::new();

                // Read other/*.nbt and sections/*.delta
                let other_dir = region_repo_dir.join("other");
                let sections_dir = region_repo_dir.join("sections");

                let mut chunkxz2other: HashMap<(i32, i32), Vec<u8>> = HashMap::new();
                let mut chunkxz2delta: HashMap<(i32, i32), Vec<u8>> = HashMap::new();

                if other_dir.exists() {
                    for entry in fs::read_dir(&other_dir)
                        .map_err(|e| format!("Failed to read other dir: {}", e))?
                    {
                        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
                        let path = entry.path();
                        let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                        if let Some(chunk_xz) = extract_xz(&filename) {
                            if path.extension().and_then(|s| s.to_str()) == Some("nbt") {
                                let data = fs::read(&path)
                                    .map_err(|e| format!("Failed to read other file: {}", e))?;
                                chunkxz2other.insert(chunk_xz, data);
                                processed_paths
                                    .push(path.strip_prefix(repo_dir).unwrap().to_path_buf());
                            }
                        }
                    }
                }

                if sections_dir.exists() {
                    for entry in fs::read_dir(&sections_dir)
                        .map_err(|e| format!("Failed to read sections dir: {}", e))?
                    {
                        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
                        let path = entry.path();
                        let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                        if let Some(chunk_xz) = extract_xz(&filename) {
                            if path.extension().and_then(|s| s.to_str()) == Some("delta") {
                                let data = fs::read(&path)
                                    .map_err(|e| format!("Failed to read delta file: {}", e))?;
                                chunkxz2delta.insert(chunk_xz, data);
                                processed_paths
                                    .push(path.strip_prefix(repo_dir).unwrap().to_path_buf());
                            }
                        }
                    }
                }

                // Decode each chunk: delta IS the sections_dump (no base XOR needed here,
                // since flatten writes sections_dump directly as .delta)
                for chunk_xz in chunkxz2delta
                    .keys()
                    .chain(chunkxz2other.keys())
                    .cloned()
                    .collect::<std::collections::HashSet<_>>()
                {
                    let other = chunkxz2other.get(&chunk_xz).ok_or_else(|| {
                        format!(
                            "Missing other nbt for chunk ({}, {})",
                            chunk_xz.0, chunk_xz.1
                        )
                    })?;
                    let sections_dump = chunkxz2delta.get(&chunk_xz).ok_or_else(|| {
                        format!(
                            "Missing sections delta for chunk ({}, {})",
                            chunk_xz.0, chunk_xz.1
                        )
                    })?;
                    let chunk_nbt = restore_chunk_from_raw(sections_dump, other);
                    chunkxz2nbt.insert(chunk_xz, chunk_nbt);
                }

                // rel_path in repo_dir maps to same relative path in save_dir
                let save_region_path = save_dir.join(&rel_path);

                write_region_file(
                    &save_region_path,
                    region_x,
                    region_z,
                    &timestamp_header,
                    &chunkxz2nbt,
                )?;

                Ok(processed_paths)
            },
        )
        .collect::<Result<Vec<Vec<_>>, String>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    Ok(processed_paths)
}
