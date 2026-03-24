use anyhow::Result;
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use pumpkin_nbt::{Nbt, compound::NbtCompound, tag::NbtTag};
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
) -> Option<([u8; 4096], Vec<(i32, i32, Vec<u8>)>)> {
    // TODO: Streaming output here.
    // A .mca file in 16MiB size can generate tons of bytes after decompression.

    let mut locations = [0u8; 4096];
    if let Err(err) = buf.read_exact(&mut locations)
        && err.kind() == std::io::ErrorKind::UnexpectedEof
    {
        return None; // empty file
    }
    let mut timestamps = [0u8; 4096];
    buf.read_exact(&mut timestamps).unwrap();

    let mut chunks = Vec::new();
    for i in 0..1024usize {
        let loc = &locations[i * 4..(i + 1) * 4];
        let offset = u32::from_be_bytes([0, loc[0], loc[1], loc[2]]) as usize;
        let size = loc[3] as usize;
        if offset == 0 && size == 0 {
            continue;
        }
        let byte_offset = offset * SECTOR_SIZE;
        let byte_size = size * SECTOR_SIZE;
        buf.seek(SeekStart(byte_offset as u64)).unwrap();
        let mut raw = vec![0u8; byte_size];
        buf.read_exact(&mut raw).unwrap();

        let data_length = u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
        let compression_type = raw[4];
        let compressed_len = data_length - 1;
        let compressed = &raw[5..5 + compressed_len];

        if compression_type == 2 {
            let mut decoder = ZlibDecoder::new(compressed);
            let mut nbt = Vec::new();
            decoder.read_to_end(&mut nbt).unwrap(); // TODO: par here
            let local_x = (i % 32) as i32;
            let local_z = (i / 32) as i32;
            chunks.push((region_x * 32 + local_x, region_z * 32 + local_z, nbt));
        }
    }

    Some((timestamps, chunks))
}

/// Reconstruct a .mca region file from a timestamp header and chunks.
pub fn write_region<B: Write + Seek>(
    region_x: i32,
    region_z: i32,
    timestamp_header: &[u8; 4096],
    chunks: &[(i32, i32, impl AsRef<[u8]>)],
    mut buf: B,
) {
    buf.seek(SeekStart(4096)).unwrap();
    buf.write(timestamp_header).unwrap();

    let mut current_sector = 2usize;
    for (chunk_x, chunk_z, nbt) in chunks {
        let local_x = chunk_x - (region_x * 32);
        let local_z = chunk_z - (region_z * 32);
        let index = (local_x + local_z * 32) as usize;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(nbt.as_ref()).unwrap();
        let compressed = encoder.finish().unwrap(); // TODO: par here

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

        buf.seek(SeekStart(loc_offset as u64)).unwrap();
        buf.write(&[
            sector_bytes[1],
            sector_bytes[2],
            sector_bytes[3],
            sectors_needed as u8,
        ])
        .unwrap();

        buf.seek(SeekStart((current_sector * SECTOR_SIZE) as u64))
            .unwrap();
        buf.write(&payload).unwrap();
        buf.write(&std::iter::repeat_n(0u8, padding).collect::<Vec<u8>>())
            .unwrap();

        current_sector += sectors_needed;
    }
}

/// Parse (region_x, region_z) from a filename like "r.-1.2.mca".
#[must_use]
pub fn parse_xz(filename: &str) -> (i32, i32) {
    let parts: Vec<&str> = filename.split('.').collect();
    let x: i32 = parts[1].parse().unwrap();
    let z: i32 = parts[2].parse().unwrap();
    (x, z)
}

#[derive(Serialize, Deserialize)]
pub struct Section {
    pub y: i8,
    pub biome: Vec<u8>,
    pub block_state: Vec<u16>,
}

#[derive(Serialize, Deserialize)]
pub struct SectionsDump {
    sections: Vec<Section>,
}

fn dump_sections(sections: &[NbtTag]) -> (SectionsDump, Vec<String>) {
    let mut warnings = Vec::new();
    let sections = sections
        .iter()
        .enumerate()
        .filter_map(|(idx, section)| {
            let section = section.extract_compound().unwrap();
            let y = section.get_byte("Y").unwrap();
            let biome_dump = {
                let Some(biome) = section.get_compound("biomes") else {
                    warnings.push(format!(
                        "Expect sections.{idx} (y={y}) contains 'biomes', but all fields got: {:?}",
                        section
                            .child_tags
                            .iter()
                            .map(|(field, _)| field)
                            .collect::<Vec<_>>()
                    ));
                    return None;
                };
                BiomePalette::from_disk_nbt(biome)
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
            };
            let block_dump = {
                let block_states = section.get_compound("block_states").unwrap();
                BlockPalette::from_disk_nbt(block_states)
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
            };
            // TODO: extract block/sky light
            Some(Ok(Section {
                y,
                biome: biome_dump,
                block_state: block_dump,
            }))
        })
        .collect::<Result<Vec<_>>>()
        .unwrap();
    (SectionsDump { sections }, warnings)
}

fn load_sections(dump: SectionsDump) -> Vec<NbtTag> {
    dump.sections
        .into_iter()
        .map(|section| {
            NbtTag::Compound(NbtCompound {
                child_tags: vec![
                    ("Y".into(), NbtTag::Byte(section.y)),
                    (
                        "biomes".into(),
                        NbtTag::Compound(
                            BiomePalette::from_iter(section.biome.into_iter()).to_disk_nbt(),
                        ),
                    ),
                    (
                        "block_states".into(),
                        NbtTag::Compound(
                            BlockPalette::from_iter(section.block_state.into_iter()).to_disk_nbt(),
                        ),
                    ),
                ],
            })
        })
        .collect()
}

/// Split a chunk nbt into (other_nbt, sections_dump, warnings)
pub fn split_chunk(mut nbt: Nbt) -> Result<(Nbt, SectionsDump, Vec<String>)> {
    let (sections_dump, warnings) = {
        let sections_idx = nbt
            .root_tag
            .child_tags
            .iter()
            .position(|(field, _)| field == "sections")
            .unwrap();
        let sections = nbt.root_tag.child_tags.swap_remove(sections_idx).1;
        let sections = sections.extract_list().unwrap();
        dump_sections(sections)
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
        panic!("Should contain 'isLightOn', got: {:#?}", nbt);
        // nbt.root_tag.put_bool("isLightOn", false);
    }

    Ok((nbt, sections_dump, warnings))
}

/// Restore a chunk nbt from (other_nbt, sections_dump)
pub fn restore_chunk(mut other: Nbt, dump: SectionsDump) -> Nbt {
    other.root_tag.put_list("sections", load_sections(dump));
    other
}
