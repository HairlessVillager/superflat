use std::io::Cursor;

use rayon::prelude::*;

use pumpkin_nbt::{Nbt, deserializer::NbtReadHelper, from_bytes, tag::NbtTag, to_bytes};
use pumpkin_world::chunk::ChunkSections;
use pumpkin_world::chunk::format::ChunkSectionNBT;

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

pub fn chunk_region_decode_batch(
    others: Vec<&[u8]>,
    sections_deltas: Vec<&[u8]>,
    sections_dumps: Vec<&[u8]>,
    compressed: bool,
) -> Result<Vec<Vec<u8>>, String> {
    others
        .into_par_iter()
        .zip(sections_deltas.into_par_iter())
        .zip(sections_dumps.into_par_iter())
        .map(|((other, delta_sections), base_sections_raw)| {
            let base_sections = if compressed {
                zstd::decode_all(Cursor::new(base_sections_raw))
                    .map_err(|e| format!("Failed to decompress sections: {}", e))?
            } else {
                base_sections_raw.to_vec()
            };

            let target_sections: Vec<u8> = base_sections
                .iter()
                .zip(delta_sections.iter())
                .map(|(x, y)| x ^ y)
                .collect();

            let result = restore_chunk_from_raw(&target_sections, other);

            Ok(result)
        })
        .collect::<Result<_, _>>()
}
