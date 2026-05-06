use anyhow::{Context, Result};
use simdnbt::{
    Mutf8String,
    owned::{BaseNbt, NbtCompound, NbtList, NbtTag},
};
use std::io::Cursor;

use super::Crafter;
use crate::odb::{OdbReader, OdbWriter};
use crate::utils::nbt::{dump_nbt, load_nbt, sort_nbt};
use crate::utils::region::{parse_xz, read_region, write_region};

const FLATTEN_PATTERNS: &[&str] = &["**/entities/r.*.*.mca"];

const UNFLATTEN_PATTERNS: &[&str] = &["**/entities/r.*.*.mca/timestamp-header"];

pub struct EntitiesRegionCrafter;

impl Crafter for EntitiesRegionCrafter {
    fn flatten(self, save: &impl OdbReader, storage: &mut impl OdbWriter) -> Result<()> {
        for pattern in FLATTEN_PATTERNS {
            for key in save.glob(pattern)? {
                log::info!("Process entities region file {key}");
                let data = save.get(&key)?;
                let filename = key.split('/').next_back().unwrap_or("");
                let (region_x, region_z) = parse_xz(filename)
                    .with_context(|| format!("failed to parse (x,z) from {key}"))
                    .context("failed to parse region coordinates")?;
                let Some((timestamp_header, chunks)) =
                    read_region(Cursor::new(data), region_x, region_z)
                        .with_context(|| format!("failed to read region from {key}"))
                        .context("failed to read region")?
                else {
                    continue;
                };

                let mut data_version: Option<i32> = None;
                let mut all_entities: Vec<NbtCompound> = Vec::new(); // TODO: with_capacity

                for (_chunk_x, _chunk_z, raw_bytes) in chunks {
                    let raw_nbt =
                        load_nbt(Cursor::new(&raw_bytes)).context("failed to load chunk nbt")?;
                    let mut comp = raw_nbt.as_compound();

                    if let Some(got) = comp.int("DataVersion") {
                        if let Some(expected) = data_version {
                            anyhow::ensure!(
                                got == expected,
                                "All 'DataVersion' should equal, first got {expected}, then got {got}"
                            );
                        } else {
                            data_version = Some(got);
                        }
                    } else {
                        log::warn!("Missing field 'DataVersion'");
                    }

                    if let Some(NbtTag::List(NbtList::Compound(entities))) = comp.remove("Entities")
                    {
                        all_entities.extend(entities);
                    } else {
                        log::warn!("Missing field 'Entities'");
                    }
                }
                sort_entities_by_uuid(&mut all_entities);

                let Some(data_version) = data_version else {
                    log::warn!("No DataVersion found in {key}, skipping");
                    continue;
                };

                let mut merged = NbtCompound::new();
                merged.insert("DataVersion", data_version);
                merged.insert("Entities", NbtList::from(all_entities));

                let nbt = sort_nbt(BaseNbt::new(Mutf8String::from(""), merged));
                storage.put(&format!("{key}/entities.nbt"), &dump_nbt(nbt, 0)?)?;
                storage.put(&format!("{key}/timestamp-header"), &timestamp_header)?;
            }
        }
        Ok(())
    }

    fn unflatten(self, save: &mut impl OdbWriter, storage: &impl OdbReader) -> Result<()> {
        for pattern in UNFLATTEN_PATTERNS {
            for ts_key in storage.glob(pattern)? {
                log::info!("Process entities region file {ts_key}");
                let Some(region_key) = ts_key.strip_suffix("/timestamp-header") else {
                    continue;
                };
                let filename = region_key.split('/').next_back().unwrap_or("");
                let (region_x, region_z) = parse_xz(filename)
                    .with_context(|| format!("failed to parse (x,z) from {ts_key}"))
                    .context("failed to parse region coordinates")?;

                let timestamp_header = storage.get(&ts_key)?;
                let data = storage.get(&format!("{region_key}/entities.nbt"))?;
                let nbt = load_nbt(Cursor::new(&data)).context("failed to load entities nbt")?;
                let comp = nbt.as_compound();

                let data_version = comp
                    .int("DataVersion")
                    .context("missing DataVersion in entities.nbt")?;

                let entities = match comp.list("Entities") {
                    Some(NbtList::Compound(entities)) => entities.clone(),
                    _ => {
                        log::warn!("No Entities found in {ts_key}, writing empty region");
                        vec![]
                    }
                };

                let mut chunk_map: std::collections::HashMap<(i32, i32), Vec<NbtCompound>> =
                    std::collections::HashMap::new();

                for entity in entities {
                    let chunk_pos = if let Some(NbtList::Double(pos)) = entity.list("Pos") {
                        let x = pos.first().copied().unwrap_or(0.0);
                        let z = pos.get(2).copied().unwrap_or(0.0);
                        ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
                    } else {
                        log::warn!(
                            "Entity without Pos, skipping: {:?}",
                            entity.keys().collect::<Vec<_>>()
                        );
                        continue;
                    };
                    chunk_map.entry(chunk_pos).or_default().push(entity);
                }

                let mut chunks: Vec<(i32, i32, Vec<u8>)> = Vec::new();

                for ((chunk_x, chunk_z), entities) in chunk_map {
                    let mut chunk_comp = NbtCompound::new();
                    chunk_comp.insert("DataVersion", data_version);
                    chunk_comp.insert("Entities", NbtList::Compound(entities));
                    chunk_comp.insert("Position", NbtList::from(vec![chunk_x, chunk_z]));

                    let chunk_nbt = BaseNbt::new(Mutf8String::from(""), chunk_comp);
                    let bytes = dump_nbt(chunk_nbt, 0)?;
                    chunks.push((chunk_x, chunk_z, bytes));
                }

                let mut mca_buf = Vec::with_capacity(200 * 1024);
                write_region(
                    region_x,
                    region_z,
                    &timestamp_header[..4096]
                        .try_into()
                        .context("timestamp header must be at least 4096 bytes")?,
                    chunks,
                    Cursor::new(&mut mca_buf),
                )
                .with_context(|| format!("failed to write region for {ts_key}"))
                .context("failed to write region")?;
                save.put(region_key, &mca_buf)?;
            }
        }
        Ok(())
    }
}

fn sort_entities_by_uuid(entities: &mut [NbtCompound]) {
    use std::cmp::Ordering;
    entities.sort_unstable_by(|a, b| match (a.list("UUID"), b.list("UUID")) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => a.int_arrays().cmp(&b.int_arrays()),
    });
}
