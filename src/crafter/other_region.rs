use std::io::Cursor;

use super::Crafter;
use crate::odb::{OdbReader, OdbWriter};
use crate::utils::nbt::{dump_nbt, load_nbt, sort_nbt};
use crate::utils::region::{parse_xz, read_region, write_region};

const FLATTEN_PATTERNS: &[&str] = &[
    "entities/r.*.*.mca",
    "poi/r.*.*.mca",
    "DIM1/entities/r.*.*.mca",
    "DIM1/poi/r.*.*.mca",
    "DIM-1/entities/r.*.*.mca",
    "DIM-1/poi/r.*.*.mca",
    "dimensions/*/*/entities/r.*.*.mca",
    "dimensions/*/*/poi/r.*.*.mca",
];

const UNFLATTEN_PATTERNS: &[&str] = &[
    "entities/r.*.*.mca/timestamp-header",
    "poi/r.*.*.mca/timestamp-header",
    "DIM1/entities/r.*.*.mca/timestamp-header",
    "DIM1/poi/r.*.*.mca/timestamp-header",
    "DIM-1/entities/r.*.*.mca/timestamp-header",
    "DIM-1/poi/r.*.*.mca/timestamp-header",
    "dimensions/*/*/entities/r.*.*.mca/timestamp-header",
    "dimensions/*/*/poi/r.*.*.mca/timestamp-header",
];

pub struct OtherRegionCrafter;

impl Crafter for OtherRegionCrafter {
    fn flatten(self, save: &impl OdbReader, storage: &mut impl OdbWriter) {
        for pattern in FLATTEN_PATTERNS {
            for key in save.glob(pattern) {
                log::info!("Process other region file {key}");
                let data = save.get(&key);
                let filename = key.split('/').next_back().unwrap_or("");
                let (region_x, region_z) = parse_xz(filename);
                let Some((timestamp_header, chunks)) =
                    read_region(Cursor::new(data), region_x, region_z)
                else {
                    continue;
                };
                storage.put(&format!("{key}/timestamp-header"), &timestamp_header);
                for (chunk_x, chunk_z, nbt) in chunks {
                    let nbt = {
                        let raw = load_nbt(Cursor::new(&nbt), true);
                        let sorted = sort_nbt(raw);
                        dump_nbt(sorted, true)
                    };
                    storage.put(&format!("{key}/c.{chunk_x}.{chunk_z}.nbt"), &nbt);
                }
            }
        }
    }

    fn unflatten(self, save: &mut impl OdbWriter, storage: &impl OdbReader) {
        for pattern in UNFLATTEN_PATTERNS {
            for ts_key in storage.glob(pattern) {
                log::info!("Process other region file (timestamp header) {ts_key}");
                let Some(region_key) = ts_key.strip_suffix("/timestamp-header") else {
                    continue;
                };
                let filename = region_key.split('/').next_back().unwrap_or("");
                let (region_x, region_z) = parse_xz(filename);
                let timestamp_header = storage.get(&ts_key);
                let chunk_pattern = format!("{region_key}/c.*.*.nbt");
                let mut chunks = Vec::new();
                for chunk_key in storage.glob(&chunk_pattern) {
                    let chunk_filename = chunk_key.split('/').next_back().unwrap_or("");
                    let (chunk_x, chunk_z) = parse_xz(chunk_filename);
                    let nbt = storage.get(&chunk_key);
                    chunks.push((chunk_x, chunk_z, nbt));
                }
                let mut mca_buf = Vec::with_capacity(200 * 1024); // 200KiB
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
