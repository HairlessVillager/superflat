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
    async fn flatten(self, save: &impl OdbReader, storage: &mut impl OdbWriter) {
        for pattern in FLATTEN_PATTERNS {
            for key in save.glob(pattern).await {
                log::info!("Process other region file {key}");
                let data = save.get(&key).await;
                let filename = key.split('/').next_back().unwrap_or("");
                let (region_x, region_z) = parse_xz(filename);
                let Some((timestamp_header, chunks)) = read_region(&data, region_x, region_z)
                else {
                    continue;
                };
                storage
                    .put(&format!("{}/timestamp-header", key), timestamp_header)
                    .await;
                for (chunk_x, chunk_z, nbt) in chunks {
                    let nbt = {
                        let raw = load_nbt(Cursor::new(&nbt), true);
                        let sorted = sort_nbt(raw);
                        let bytes = dump_nbt(sorted, true);
                        bytes
                    };
                    storage
                        .put(&format!("{}/c.{}.{}.nbt", key, chunk_x, chunk_z), &nbt)
                        .await;
                }
            }
        }
    }

    async fn unflatten(self, save: &mut impl OdbWriter, storage: &impl OdbReader) {
        for pattern in UNFLATTEN_PATTERNS {
            for ts_key in storage.glob(pattern).await {
                let Some(region_key) = ts_key.strip_suffix("/timestamp-header") else {
                    continue;
                };
                let filename = region_key.split('/').next_back().unwrap_or("");
                let (region_x, region_z) = parse_xz(filename);
                let timestamp_header = storage.get(&ts_key).await;
                let chunk_pattern = format!("{}/c.*.*.nbt", region_key);
                let mut chunks = Vec::new();
                for chunk_key in storage.glob(&chunk_pattern).await {
                    let chunk_filename = chunk_key.split('/').next_back().unwrap_or("");
                    let (chunk_x, chunk_z) = parse_xz(chunk_filename);
                    let nbt = storage.get(&chunk_key).await;
                    chunks.push((chunk_x, chunk_z, nbt));
                }
                let mca_data = write_region(region_x, region_z, &timestamp_header, &chunks);
                save.put(region_key, &mca_data).await;
            }
        }
    }
}
