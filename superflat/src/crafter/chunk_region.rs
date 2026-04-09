use std::io::Cursor;

use anyhow::{Context, Result};
use pumpkin_nbt::{from_bytes, to_bytes};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use super::Crafter;
use crate::odb::{OdbReader, OdbWriter};
use crate::utils::nbt::{dump_nbt, load_nbt, sort_nbt};
use crate::utils::region::{
    SectionsDump, parse_xz, read_region, restore_chunk, split_chunk, write_region,
};

const FLATTEN_PATTERNS: &[&str] = &[
    "region/r.*.*.mca",
    "DIM1/region/r.*.*.mca",
    "DIM-1/region/r.*.*.mca",
    "dimensions/*/*/region/r.*.*.mca",
];

const UNFLATTEN_PATTERNS: &[&str] = &[
    "region/r.*.*.mca/timestamp-header", // timestamp-header is sentry
    "DIM1/region/r.*.*.mca/timestamp-header",
    "DIM-1/region/r.*.*.mca/timestamp-header",
    "dimensions/*/*/region/r.*.*.mca/timestamp-header",
];

pub struct ChunkRegionCrafter;

impl Crafter for ChunkRegionCrafter {
    fn flatten(self, save: &impl OdbReader, storage: &mut impl OdbWriter) {
        for pattern in FLATTEN_PATTERNS {
            for key in save.glob(pattern) {
                log::info!("Process chunk region file {key}");
                let data = save.get(&key);
                let filename = key.split('/').next_back().unwrap_or("");
                let (region_x, region_z) = parse_xz(filename)
                    .with_context(|| format!("failed to parse (x,z) from {key}"))
                    .expect("failed to parse region coordinates");
                let Some((timestamp_header, chunks)) =
                    read_region(Cursor::new(data), region_x, region_z)
                        .with_context(|| format!("failed to read region from {key}"))
                        .expect("failed to read region")
                else {
                    continue;
                };
                storage.put(&format!("{key}/timestamp-header"), &timestamp_header);

                let result = chunks
                    .into_par_iter()
                    .map(|(chunk_x, chunk_z, nbt)| {
                        let nbt = load_nbt(Cursor::new(&nbt), true);
                        if nbt.get_string("Status").expect("missing Status field in chunk nbt") != "minecraft:full" {
                            return Ok(None);
                        }
                        let (other, sections) = split_chunk(nbt).with_context(|| {
                            format!("failed to process chunk ({chunk_x}, {chunk_z}) at file {key}")
                        })?;
                        let other_dump = dump_nbt(sort_nbt(other), true);
                        let mut sections_dump = Vec::with_capacity(200 * 1024);
                        to_bytes(&sections, &mut sections_dump).expect("failed to serialize sections dump");
                        Ok(Some((chunk_x, chunk_z, other_dump, sections_dump)))
                    })
                    .collect::<Result<Vec<_>>>()
                    .expect("failed to process chunks")
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                storage.put_par(
                    result
                        .iter()
                        .flat_map(|(chunk_x, chunk_z, other, dump)| {
                            [
                                (
                                    format!("{key}/other/c.{chunk_x}.{chunk_z}.nbt"),
                                    other.as_ref(),
                                ),
                                (
                                    format!("{key}/sections/c.{chunk_x}.{chunk_z}.dump"),
                                    dump.as_slice(),
                                ),
                            ]
                        })
                        .collect::<Vec<_>>(),
                );
            }
        }
    }

    fn unflatten(self, save: &mut impl OdbWriter, storage: &impl OdbReader) {
        for pattern in UNFLATTEN_PATTERNS {
            for ts_key in storage.glob(pattern) {
                log::info!("Process chunk region file (timestamp header) {ts_key}");
                let Some(region_key) = ts_key.strip_suffix("/timestamp-header") else {
                    continue;
                };
                let filename = region_key.split('/').next_back().unwrap_or("");
                let (region_x, region_z) = parse_xz(filename)
                    .with_context(|| format!("failed to parse (x,z) from {ts_key}"))
                    .expect("failed to parse region coordinates");
                let timestamp_header = storage.get(&ts_key);

                let other_pattern = format!("{region_key}/other/c.*.*.nbt");

                let other_keys: Vec<String> = storage.glob(&other_pattern);
                let coords: Vec<(i32, i32)> = other_keys
                    .iter()
                    .map(|k| {
                        parse_xz(k.split('/').next_back().unwrap_or(""))
                            .with_context(|| format!("failed to parse (x,z) from {k}"))
                    })
                    .collect::<Result<_>>()
                    .expect("failed to parse chunk coordinates");
                let dump_keys: Vec<String> = coords
                    .iter()
                    .map(|(cx, cz)| format!("{region_key}/sections/c.{cx}.{cz}.dump"))
                    .collect();

                let all_keys: Vec<&str> = other_keys
                    .iter()
                    .map(|s| s.as_str())
                    .chain(dump_keys.iter().map(|s| s.as_str()))
                    .collect();
                let mut all_data = storage.get_par(&all_keys);
                let dump_data = all_data.split_off(other_keys.len());
                let nbt_data = all_data;

                let tasks: Vec<(i32, i32, Vec<u8>, Vec<u8>)> = coords
                    .into_iter()
                    .zip(nbt_data)
                    .zip(dump_data)
                    .map(|(((cx, cz), nbt), dump)| (cx, cz, nbt, dump))
                    .collect();

                let chunks = tasks
                    .into_par_iter()
                    .map(|(chunk_x, chunk_z, nbt_data, dump_data)| {
                        let other = load_nbt(Cursor::new(&nbt_data), true);
                        let sections_dump: SectionsDump =
                            from_bytes(Cursor::new(&dump_data)).expect("failed to deserialize sections dump");
                        let nbt = dump_nbt(
                            restore_chunk(other, sections_dump)
                                .with_context(|| format!("failed to restore chunk for {ts_key}"))
                                .expect("failed to restore chunk"),
                            true,
                        );
                        (chunk_x, chunk_z, nbt)
                    })
                    .collect::<Vec<_>>();

                let mut mca_buf = Vec::with_capacity(8 * 1024 * 1024); // 8MiB
                write_region(
                    region_x,
                    region_z,
                    &timestamp_header[..4096].try_into().expect("timestamp header must be at least 4096 bytes"),
                    chunks,
                    Cursor::new(&mut mca_buf),
                )
                .with_context(|| format!("failed to write region for {ts_key}"))
                .expect("failed to write region");
                save.put(region_key, &mca_buf);
            }
        }
    }
}
