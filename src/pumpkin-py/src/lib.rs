use std::io::Cursor;
use std::sync::Arc;

use pumpkin_nbt::tag::NbtTag;
use pumpkin_world::chunk::format::{ChunkNbt, ChunkSectionNBT};
use pumpkin_world::world_info::anvil::LevelDat;
use pyo3::exceptions::PyValueError;
use pyo3::types::{PyBytes, PyList};
use pyo3::{prelude::*, wrap_pyfunction};

use rayon::prelude::*;

use pumpkin_config::lighting::LightingEngineConfig;
use pumpkin_data::dimension::Dimension;
use pumpkin_nbt::deserializer::NbtReadHelper;
use pumpkin_nbt::{Nbt, from_bytes, normalize_nbt_bytes, to_bytes};
use pumpkin_util::world_seed::Seed;
use pumpkin_world::biome::hash_seed;
use pumpkin_world::chunk::{ChunkData, ChunkSections};
use pumpkin_world::chunk_system::{Chunk, StagedChunkEnum, generate_single_chunk};
use pumpkin_world::generation::get_world_gen;
use pumpkin_world::world::BlockRegistryExt;
use serde::{Deserialize, Serialize};

fn generate_chunk_data(seed: u64, chunk_x: i32, chunk_z: i32) -> Result<Arc<ChunkData>, String> {
    struct BlockRegistry;
    impl BlockRegistryExt for BlockRegistry {
        fn can_place_at(
            &self,
            _block: &pumpkin_data::Block,
            _state: &pumpkin_data::BlockState,
            _block_accessor: &dyn pumpkin_world::world::BlockAccessor,
            _block_pos: &pumpkin_util::math::position::BlockPos,
        ) -> bool {
            true
        }
    }

    let dimension = Dimension::OVERWORLD;
    let seed_val = Seed(seed);
    let block_registry = Arc::new(BlockRegistry);
    let world_gen = get_world_gen(seed_val, dimension);
    let biome_mixer_seed = hash_seed(world_gen.random_config.seed);

    let mut chunk = generate_single_chunk(
        &dimension,
        biome_mixer_seed,
        &world_gen,
        block_registry.as_ref(),
        chunk_x,
        chunk_z,
        StagedChunkEnum::Full,
    );

    if let Chunk::Proto(_) = chunk {
        chunk.upgrade_to_level_chunk(&dimension, &LightingEngineConfig::Default);
    }

    if let Chunk::Level(chunk) = chunk {
        Ok(chunk)
    } else {
        Err("Failed to upgrade chunk to Level stage".to_string())
    }
}

#[pyfunction]
fn normalize_nbt<'py>(nbt: &[u8]) -> PyResult<Vec<u8>> {
    let bytes: Vec<u8> = normalize_nbt_bytes(&nbt)
        .map(|v| v.into())
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(bytes)
}

#[pyfunction]
fn is_chunk_status_full(input: &[u8]) -> PyResult<bool> {
    let cursor = Cursor::new(input);
    let nbt = Nbt::read(&mut NbtReadHelper::new(cursor))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let status = nbt.get_string("Status").ok_or(PyValueError::new_err(
        "Chunk NBT does not have Status field".to_string(),
    ))?;
    Ok(status == "minecraft:full")
}

#[derive(Serialize, Deserialize)]
struct SectionsDump {
    biomes_dump: Vec<u8>,
    blocks_dump: Vec<u16>,
}

fn chunk_data_to_sections_dump(chunk_data: &ChunkData) -> Vec<u8> {
    let dump = SectionsDump {
        biomes_dump: chunk_data.section.dump_biomes(),
        blocks_dump: chunk_data.section.dump_blocks(),
    };
    let mut buf = Vec::new(); // TODO: use .with_capacity here
    to_bytes(&dump, &mut buf).expect("Failed to dump thin data");
    normalize_nbt_bytes(&buf)
        .expect("Failed to normalize sections dump")
        .into()
}

fn delete_section_from_nbt(chunk_nbt: &[u8]) -> Vec<u8> {
    let mut nbt = Nbt::read(&mut NbtReadHelper::new(Cursor::new(chunk_nbt)))
        .expect("Failed to parse chunk data when building other");

    nbt.root_tag.child_tags = nbt
        .root_tag
        .child_tags
        .into_iter()
        .filter_map(|(k, v)| match k.as_str() {
            // remove sections field
            "sections" => None,

            // turn off light, Minecraft server will re-compute them
            "isLightOn" => Some((k, NbtTag::Byte(0b0))), // 0b0 => false

            _ => Some((k, v)),
        })
        .collect();
    nbt.write().into()
}

fn restore_chunk_from(sections_dump: &[u8], other: &[u8]) -> Vec<u8> {
    let sections =
        from_bytes::<SectionsDump>(Cursor::new(sections_dump)).expect("Failed to load sections");
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

#[pyfunction]
fn seed_to_sections_batch(seed: u64, coords: Bound<'_, PyList>) -> Vec<Vec<u8>> {
    let coords = coords
        .iter()
        .map(|e| {
            e.extract::<(i32, i32)>()
                .expect("Failed to exrtact (i32, i32) from coords element")
        })
        .collect::<Vec<_>>();
    coords
        .into_par_iter()
        .map(|(chunk_x, chunk_z)| {
            let nbt =
                generate_chunk_data(seed, chunk_x, chunk_z).expect("Failed to generate chunk data");
            let data = chunk_data_to_sections_dump(&nbt);
            zstd::encode_all(Cursor::new(&data), 19).expect("Failed to compress sections")
        })
        .collect()
}

#[pyfunction]
fn chunk_region_encode_batch(
    chunk_nbts: Vec<Bound<'_, PyBytes>>,
    sections_dumps: Vec<Bound<'_, PyBytes>>,
) -> Vec<(Vec<u8>, Vec<u8>)> {
    let chunk_nbts = chunk_nbts.iter().map(|e| e.as_bytes()).collect::<Vec<_>>();
    let sections_dumps = sections_dumps
        .iter()
        .map(|e| e.as_bytes())
        .collect::<Vec<_>>();
    chunk_nbts
        .into_par_iter()
        .zip(sections_dumps.into_par_iter())
        .map(|(chunk_nbt, compressed_base_sections)| {
            let base_sections = zstd::decode_all(Cursor::new(compressed_base_sections))
                .expect("Failed to decompress sections");

            let target_sections = {
                let chunk_nbt = from_bytes::<ChunkNbt>(Cursor::new(chunk_nbt))
                    .expect("Failed to load ChunkNbt from raw nbt bytes");
                let chunk_data = ChunkData::from_nbt(chunk_nbt);
                chunk_data_to_sections_dump(&chunk_data)
            };
            let other = delete_section_from_nbt(&chunk_nbt);
            let delta_sections: Vec<u8> = base_sections
                .iter()
                .zip(target_sections.iter())
                .map(|(x, y)| x ^ y)
                .collect();
            (delta_sections, other)
        })
        .collect()
}

#[pyfunction]
fn chunk_region_decode_batch(
    others: Vec<Bound<'_, PyBytes>>,
    sections_deltas: Vec<Bound<'_, PyBytes>>,
    sections_dumps: Vec<Bound<'_, PyBytes>>,
) -> Vec<Vec<u8>> {
    let others = others.iter().map(|e| e.as_bytes()).collect::<Vec<_>>();
    let sections_deltas = sections_deltas
        .iter()
        .map(|e| e.as_bytes())
        .collect::<Vec<_>>();
    let sections_dumps = sections_dumps
        .iter()
        .map(|e| e.as_bytes())
        .collect::<Vec<_>>();
    others
        .into_par_iter()
        .zip(sections_deltas.into_par_iter())
        .zip(sections_dumps.into_par_iter())
        .map(|((other, delta_sections), compressed_base_sections)| {
            let base_sections = zstd::decode_all(Cursor::new(compressed_base_sections))
                .expect("Failed to decompress sections");
            let target_sections: Vec<u8> = base_sections
                .iter()
                .zip(delta_sections.iter())
                .map(|(x, y)| x ^ y)
                .collect();
            restore_chunk_from(&target_sections, &other)
        })
        .collect()
}

#[pyfunction]
fn seed_from_level(level_nbt: &[u8]) -> i64 {
    from_bytes::<LevelDat>(Cursor::new(level_nbt))
        .expect("Failed to load level nbt")
        .data
        .world_gen_settings
        .seed
}

#[pymodule]
fn pumpkin_py(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(normalize_nbt, m)?)?;
    m.add_function(wrap_pyfunction!(seed_to_sections_batch, m)?)?;
    m.add_function(wrap_pyfunction!(is_chunk_status_full, m)?)?;
    m.add_function(wrap_pyfunction!(chunk_region_encode_batch, m)?)?;
    m.add_function(wrap_pyfunction!(chunk_region_decode_batch, m)?)?;
    m.add_function(wrap_pyfunction!(seed_from_level, m)?)?;
    Ok(())
}
