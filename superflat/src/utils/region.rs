use anyhow::{Context, Result};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use pumpkin_nbt::{Nbt, compound::NbtCompound, tag::NbtTag};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom::Start as SeekStart, Write};

use crate::utils::palette::{BiomePalette, BlockPalette};

const SECTOR_SIZE: usize = 4096;

/// Parse a .mca region file into its timestamp header and chunks.
/// Returns None if the file is empty or has no chunks.
#[must_use]
pub fn read_region<B: Read + Seek>(
    mut buf: B,
    region_x: i32,
    region_z: i32,
) -> Result<Option<([u8; 4096], Vec<(i32, i32, Vec<u8>)>)>> {
    let mut locations = [0u8; 4096];
    if let Err(err) = buf.read_exact(&mut locations) {
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            return Ok(None);
        }
    }

    let mut timestamps = [0u8; 4096];
    buf.read_exact(&mut timestamps)
        .context("Buffer's length < 8192")?;

    let mut compressed_chunks = Vec::new();

    for i in 0..1024usize {
        let loc = &locations[i * 4..(i + 1) * 4];
        let offset = u32::from_be_bytes([0, loc[0], loc[1], loc[2]]) as usize;
        let size = loc[3] as usize;

        if offset == 0 && size == 0 {
            continue;
        }

        let byte_offset = offset * SECTOR_SIZE;
        buf.seek(SeekStart(byte_offset as u64))
            .with_context(|| format!("At chunk #{i}: Failed to seek {byte_offset}"))?;

        let mut header = [0u8; 5];
        buf.read_exact(&mut header)
            .with_context(|| format!("At chunk #{i}: Failed to read chunk header"))?;

        let data_length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let compression_type = header[4];

        let compressed_len = data_length.saturating_sub(1);
        let mut compressed_data = vec![0u8; compressed_len];
        buf.read_exact(&mut compressed_data).with_context(|| {
            format!("At chunk #{i}: Failed to read chunk body (length: {compressed_len})")
        })?;

        compressed_chunks.push((i, compression_type, compressed_data));
    }

    let chunks: Vec<(i32, i32, Vec<u8>)> = compressed_chunks
        .into_par_iter()
        .filter_map(|(i, compression_type, compressed)| {
            if compression_type == 2 {
                let mut decoder = ZlibDecoder::new(&compressed[..]);
                let mut nbt = Vec::new();
                if decoder.read_to_end(&mut nbt).is_ok() {
                    let local_x = (i % 32) as i32;
                    let local_z = (i / 32) as i32;
                    return Some((region_x * 32 + local_x, region_z * 32 + local_z, nbt));
                }
            } else {
                todo!("Support compression type {compression_type}")
            }
            None
        })
        .collect();

    if chunks.is_empty() {
        return Ok(None);
    }

    Ok(Some((timestamps, chunks)))
}

/// Reconstruct a .mca region file from a timestamp header and chunks.
pub fn write_region<B: Write + Seek>(
    region_x: i32,
    region_z: i32,
    timestamp_header: &[u8; 4096],
    chunks: impl IntoParallelIterator<Item = (i32, i32, impl AsRef<[u8]>)>,
    mut buf: B,
) -> Result<()> {
    buf.seek(SeekStart(4096)).context("Failed to seek 4096")?;
    buf.write(timestamp_header)
        .context("Failed to write timestamp header")?;

    let mut current_sector = 2usize;
    let chunks = chunks
        .into_par_iter()
        .map(|(chunk_x, chunk_z, nbt)| {
            let mut encoder = ZlibEncoder::new(Vec::with_capacity(8192), Compression::default());
            encoder.write_all(nbt.as_ref()).with_context(|| {
                format!("At chunk ({chunk_x}, {chunk_z}): Failed to feed data into encoder")
            })?;
            let compressed = encoder.finish().with_context(|| {
                format!("At chunk ({chunk_x}, {chunk_z}): Failed to finalize compression")
            })?;
            Ok((chunk_x, chunk_z, compressed))
        })
        .collect::<Result<Vec<_>>>()?;

    for (chunk_x, chunk_z, compressed) in chunks {
        let local_x = chunk_x - (region_x * 32);
        let local_z = chunk_z - (region_z * 32);
        let index = (local_x + local_z * 32) as usize;

        let content_length = compressed.len() + 1; // + 1 for the compression type byte
        let mut payload = Vec::with_capacity(4 + 1 + compressed.len());
        payload.extend_from_slice(&(content_length as u32).to_be_bytes());
        payload.push(2u8); // 2 means using zlib to compress
        payload.extend_from_slice(&compressed);

        let sectors_needed = payload.len().div_ceil(SECTOR_SIZE);
        if sectors_needed > 255 {
            todo!("Write big chunk (> 1020KiB) to .mcc file")
        }
        let padding = sectors_needed * SECTOR_SIZE - payload.len();

        let loc_offset = index * 4;
        let sector_bytes = (current_sector as u32).to_be_bytes();

        buf.seek(SeekStart(loc_offset as u64)).with_context(|| {
            format!("At chunk ({chunk_x}, {chunk_z}): Failed to seek header@{loc_offset}")
        })?;
        buf.write(&[
            sector_bytes[1],
            sector_bytes[2],
            sector_bytes[3],
            sectors_needed as u8,
        ])
        .with_context(|| format!("At chunk ({chunk_x}, {chunk_z}): Failed to write header"))?;

        buf.seek(SeekStart((current_sector * SECTOR_SIZE) as u64))
            .with_context(|| {
                format!(
                    "At chunk ({chunk_x}, {chunk_z}): Failed to seek chunk@{}",
                    current_sector * SECTOR_SIZE
                )
            })?;
        buf.write(&payload)
            .with_context(|| format!("At chunk ({chunk_x}, {chunk_z}): Failed to write payload"))?;

        buf.write(&std::iter::repeat_n(0u8, padding).collect::<Vec<u8>>())
            .with_context(|| format!("At chunk ({chunk_x}, {chunk_z}): Failed to write padding"))?;

        current_sector += sectors_needed;
    }
    Ok(())
}

/// Parse (region_x, region_z) from a filename like "r.-1.2.mca".
#[must_use]
pub fn parse_xz(filename: &str) -> Result<(i32, i32)> {
    let parts: Vec<&str> = filename.split('.').collect();
    let x: i32 = parts[1]
        .parse()
        .with_context(|| format!("Failed to parse {} as i32", parts[1]))?;
    let z: i32 = parts[2]
        .parse()
        .with_context(|| format!("Failed to parse {} as i32", parts[1]))?;
    Ok((x, z))
}

#[derive(Serialize, Deserialize)]
pub struct Section {
    pub y: i8,
    pub biome: Vec<u8>,
    pub block_state: Vec<u16>,
}

#[derive(Serialize, Deserialize)]
pub struct SectionsDump {
    pub sections: Vec<Section>,
}

fn dump_sections(sections: &[NbtTag]) -> Result<SectionsDump> {
    let sections = sections
        .iter()
        .enumerate()
        .map(|(idx, section)| {
            let section = section.extract_compound().with_context(|| {
                format!("Expect sections.{idx} is a NBT compund, got: {section:#?}")
            })?;
            let y = section.get_byte("Y").with_context(|| {
                format!("Missing NBT byte 'sections.{idx}.Y', got: {section:#?}")
            })?;
            let biome_dump = {
                let Some(biome) = section.get_compound("biomes") else {
                    log::warn!(
                        "Missing 'sections.{idx}.biomes' (y={y}), all fields got: {:?}",
                        section
                            .child_tags
                            .iter()
                            .map(|(field, _)| field)
                            .collect::<Vec<_>>()
                    );
                    return Ok(None);
                };
                BiomePalette::from_disk_nbt(biome)?
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
            };
            let block_dump = {
                let Some(block_states) = section.get_compound("block_states") else {
                    log::warn!(
                        "Missing 'sections.{idx}.block_states' (y={y}), all fields got: {:?}",
                        section
                            .child_tags
                            .iter()
                            .map(|(field, _)| field)
                            .collect::<Vec<_>>()
                    );
                    return Ok(None);
                };
                BlockPalette::from_disk_nbt(block_states)?
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
            };
            // TODO: extract block/sky light
            Ok(Some(Section {
                y,
                biome: biome_dump,
                block_state: block_dump,
            }))
        })
        .map(|e: Result<Option<Section>>| e.transpose())
        .filter_map(|e| e)
        .collect::<Result<_, _>>()?;
    Ok(SectionsDump { sections })
}

fn load_sections(dump: SectionsDump) -> Result<Vec<NbtTag>> {
    dump.sections
        .into_iter()
        .map(|section| {
            let child_tags = vec![
                ("Y".into(), NbtTag::Byte(section.y)),
                (
                    "biomes".into(),
                    NbtTag::Compound(
                        BiomePalette::from_iter(section.biome.into_iter()).to_disk_nbt()?,
                    ),
                ),
                (
                    "block_states".into(),
                    NbtTag::Compound(
                        BlockPalette::from_iter(section.block_state.into_iter()).to_disk_nbt()?,
                    ),
                ),
            ];
            Ok(NbtTag::Compound(NbtCompound { child_tags }))
        })
        .collect::<Result<_>>()
}

/// Split a chunk nbt into (other_nbt, sections_dump, warnings)
pub fn split_chunk(mut nbt: Nbt) -> Result<(Nbt, SectionsDump)> {
    let sections_dump = {
        let sections_idx = nbt
            .root_tag
            .child_tags
            .iter()
            .position(|(field, _)| field == "sections")
            .with_context(|| {
                format!(
                    "Missing 'sections', all fields: {:#?}",
                    nbt.root_tag
                        .child_tags
                        .iter()
                        .map(|(field, _)| field)
                        .collect::<Vec<_>>()
                )
            })?;
        let sections = nbt.root_tag.child_tags.swap_remove(sections_idx).1;
        let sections = sections
            .extract_list()
            .with_context(|| format!("Expect sections is a NBT list, got: {sections:#?}"))?;
        dump_sections(sections)?
    };

    // TODO: extract block/sky light
    if let Some(is_light_on_idx) = nbt
        .root_tag
        .child_tags
        .iter()
        .position(|(field, _)| field == "isLightOn")
    {
        nbt.root_tag.child_tags[is_light_on_idx].1 = NbtTag::Byte(i8::from(false));
    } else {
        anyhow::bail!(
            "Missing 'isLightOn', all fields: {:#?}",
            nbt.root_tag
                .child_tags
                .iter()
                .map(|(field, _)| field)
                .collect::<Vec<_>>()
        );
        // nbt.root_tag.put_bool("isLightOn", false);
    }

    Ok((nbt, sections_dump))
}

/// Restore a chunk nbt from (other_nbt, sections_dump)
pub fn restore_chunk(mut other: Nbt, dump: SectionsDump) -> Result<Nbt> {
    other.root_tag.put_list("sections", load_sections(dump)?);
    Ok(other)
}
