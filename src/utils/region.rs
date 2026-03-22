use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use pumpkin_nbt::{Nbt, compound::NbtCompound, tag::NbtTag};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use crate::utils::palette::{BiomePalette, BlockPalette};

const SECTOR_SIZE: usize = 4096;

/// Parse a .mca region file into its timestamp header and chunks.
/// Returns None if the file is empty or has no chunks.
pub fn read_region(
    data: &'_ [u8],
    region_x: i32,
    region_z: i32,
) -> Option<(&'_ [u8], Vec<(i32, i32, Vec<u8>)>)> {
    // TODO: Streaming output here.
    // A .mca file in 16MiB size can generate tons of bytes after decompression.
    if data.len() < 8192 {
        return None;
    }
    let locations = &data[0..4096];
    let timestamps = &data[4096..8192];
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
        let raw = &data[byte_offset..byte_offset + byte_size];
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
pub fn write_region(
    region_x: i32,
    region_z: i32,
    timestamp_header: &[u8],
    chunks: &[(i32, i32, Vec<u8>)],
) -> Vec<u8> {
    let mut locations = vec![0u8; SECTOR_SIZE];
    let mut chunk_data = Vec::new();
    let mut current_sector = 2usize;

    for (chunk_x, chunk_z, nbt) in chunks {
        let local_x = chunk_x - (region_x * 32);
        let local_z = chunk_z - (region_z * 32);
        let index = (local_x + local_z * 32) as usize;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(nbt).unwrap();
        let compressed = encoder.finish().unwrap(); // TODO: par here

        let content_length = compressed.len() + 1; // + 1 for the compression type byte
        let mut payload = Vec::with_capacity(4 + 1 + compressed.len());
        payload.extend_from_slice(&(content_length as u32).to_be_bytes());
        payload.push(2u8); // 2 means using zlib to compress
        payload.extend_from_slice(&compressed);

        let sectors_needed = payload.len().div_ceil(SECTOR_SIZE);
        let padding = sectors_needed * SECTOR_SIZE - payload.len();

        let loc_offset = index * 4;
        let sector_bytes = (current_sector as u32).to_be_bytes();
        locations[loc_offset] = sector_bytes[1];
        locations[loc_offset + 1] = sector_bytes[2];
        locations[loc_offset + 2] = sector_bytes[3];
        locations[loc_offset + 3] = sectors_needed as u8;

        chunk_data.extend_from_slice(&payload); // TODO: mutex result then append to it
        chunk_data.extend(std::iter::repeat_n(0u8, padding));
        current_sector += sectors_needed;
    }

    let mut result = locations;
    result.extend_from_slice(timestamp_header); // TODO: remove coping
    result.extend_from_slice(&chunk_data);
    result
}

/// Parse (region_x, region_z) from a filename like "r.-1.2.mca".
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

fn dump_sections(sections: &[NbtTag]) -> SectionsDump {
    let sections = sections
        .iter()
        .map(|section| {
            let section = section.extract_compound().unwrap();
            let y = section.get_byte("Y").unwrap();
            let biome_dump = {
                let biome = section.get_compound("biomes").expect(
                    format!("Should section contain 'biomes', got: {:#?}", section).as_str(),
                );
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
            Section {
                y,
                biome: biome_dump,
                block_state: block_dump,
            }
        })
        .collect::<Vec<_>>();
    SectionsDump { sections }
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

/// Split a chunk nbt into (other_nbt, sections_dump)
pub fn split_chunk(mut nbt: Nbt) -> (Nbt, SectionsDump) {
    let sections_dump = {
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

    (nbt, sections_dump)
}

/// Restore a chunk nbt from (other_nbt, sections_dump)
pub fn restore_chunk(mut other: Nbt, dump: SectionsDump) -> Nbt {
    other.root_tag.put_list("sections", load_sections(dump));
    other
}
