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
                let (region_x, region_z) = parse_xz(filename);
                let Some((timestamp_header, chunks)) =
                    read_region(Cursor::new(data), region_x, region_z)
                else {
                    continue;
                };
                storage.put(&format!("{key}/timestamp-header"), &timestamp_header);

                let result = chunks
                    .into_par_iter()
                    .map(|(chunk_x, chunk_z, nbt)| {
                        let nbt = load_nbt(Cursor::new(&nbt), true);
                        if nbt.get_string("Status").unwrap() != "minecraft:full" {
                            return Ok(None);
                        }
                        let (other, sections, warnings) = split_chunk(nbt).with_context(|| {
                            format!("Failed to process chunk ({chunk_x}, {chunk_z}) at file {key}")
                        })?;
                        for w in warnings {
                            log::warn!("At chunk ({chunk_x}, {chunk_z}) at file {key}: {w}");
                        }

                        let other_dump = dump_nbt(sort_nbt(other), true);
                        let mut sections_dump = Vec::with_capacity(200 * 1024);
                        to_bytes(&sections, &mut sections_dump).unwrap();
                        Ok(Some((chunk_x, chunk_z, other_dump, sections_dump)))
                    })
                    .collect::<Result<Vec<_>>>()
                    .unwrap()
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                for (chunk_x, chunk_z, other, dump) in result {
                    storage.put(&format!("{key}/other/c.{chunk_x}.{chunk_z}.nbt"), &other);
                    storage.put(&format!("{key}/sections/c.{chunk_x}.{chunk_z}.dump"), &dump);
                }
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
                let (region_x, region_z) = parse_xz(filename);
                let timestamp_header = storage.get(&ts_key);

                let chunk_pattern = format!("{region_key}/other/c.*.*.nbt");

                let mut tasks = Vec::with_capacity(1024);
                for chunk_key in storage.glob(&chunk_pattern) {
                    let chunk_filename = chunk_key.split('/').next_back().unwrap_or("");
                    let (chunk_x, chunk_z) = parse_xz(chunk_filename);
                    let nbt_data = storage.get(&chunk_key);
                    let dump_key = format!("{region_key}/sections/c.{chunk_x}.{chunk_z}.dump");
                    let dump_data = storage.get(&dump_key);
                    tasks.push((chunk_x, chunk_z, nbt_data, dump_data))
                }

                let chunks = tasks
                    .into_par_iter()
                    .map(|(chunk_x, chunk_z, nbt_data, dump_data)| {
                        let other = load_nbt(Cursor::new(&nbt_data), true);
                        let sections_dump: SectionsDump =
                            from_bytes(Cursor::new(&dump_data)).unwrap();
                        let nbt = dump_nbt(restore_chunk(other, sections_dump), true);
                        (chunk_x, chunk_z, nbt)
                    })
                    .collect::<Vec<_>>();

                let mut mca_buf = Vec::with_capacity(8 * 1024 * 1024); // 8MiB
                write_region(
                    region_x,
                    region_z,
                    &timestamp_header[..4096].try_into().unwrap(),
                    &chunks,
                    Cursor::new(&mut mca_buf),
                );
                save.put(region_key, &mca_buf);
            }
        }
    }
}
