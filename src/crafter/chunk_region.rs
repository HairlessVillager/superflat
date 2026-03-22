use std::io::Cursor;

use pumpkin_nbt::{from_bytes, to_bytes};

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
    async fn flatten(self, save: &impl OdbReader, storage: &mut impl OdbWriter) {
        for pattern in FLATTEN_PATTERNS {
            for key in save.glob(pattern).await {
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
                    let (other, dump) = {
                        let nbt = load_nbt(Cursor::new(&nbt), true);
                        if nbt.get_string("Status").unwrap() != "minecraft:full" {
                            continue;
                        }
                        // dbg!(chunk_x, chunk_z);
                        let (other, sections) = split_chunk(nbt);
                        let other_dump = dump_nbt(sort_nbt(other), true);
                        let mut sections_dump = Vec::with_capacity(200 * 1024); // TODO: use .with_capacity
                        to_bytes(&sections, &mut sections_dump).unwrap();
                        (other_dump, sections_dump)
                    };
                    storage
                        .put(
                            &format!("{}/other/c.{}.{}.nbt", key, chunk_x, chunk_z),
                            &other,
                        )
                        .await;
                    storage
                        .put(
                            &format!("{}/sections/c.{}.{}.dump", key, chunk_x, chunk_z),
                            &dump,
                        )
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

                let chunk_pattern = format!("{}/other/c.*.*.nbt", region_key);

                let mut chunks = Vec::new();
                for chunk_key in storage.glob(&chunk_pattern).await {
                    let chunk_filename = chunk_key.split('/').next_back().unwrap_or("");
                    let (chunk_x, chunk_z) = parse_xz(chunk_filename);

                    let nbt_data = storage.get(&chunk_key).await;

                    let dump_key =
                        format!("{}/sections/c.{}.{}.dump", region_key, chunk_x, chunk_z);

                    let dump_data = storage.get(&dump_key).await;

                    let nbt = {
                        let other = load_nbt(Cursor::new(&nbt_data), true);
                        let sections_dump: SectionsDump =
                            from_bytes(Cursor::new(&dump_data)).unwrap();
                        dump_nbt(restore_chunk(other, sections_dump), true)
                    };
                    chunks.push((chunk_x, chunk_z, nbt.to_vec()));
                }
                let mca_data = write_region(region_x, region_z, &timestamp_header, &chunks);
                save.put(region_key, &mca_data).await;
            }
        }
    }
}
