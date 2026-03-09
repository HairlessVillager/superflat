use std::io::Cursor;
use std::sync::Arc;

use pumpkin_world::world_info::anvil::LevelDat;
use pyo3::exceptions::PyValueError;
use pyo3::{prelude::*, wrap_pyfunction};

use rayon::prelude::*;

use pumpkin_config::lighting::LightingEngineConfig;
use pumpkin_data::dimension::Dimension;
use pumpkin_nbt::deserializer::NbtReadHelper;
use pumpkin_nbt::{Nbt, from_bytes, normalize_nbt_bytes};
use pumpkin_util::world_seed::Seed;
use pumpkin_world::biome::hash_seed;
use pumpkin_world::chunk::ChunkData;
use pumpkin_world::chunk_system::{Chunk, StagedChunkEnum, generate_single_chunk};
use pumpkin_world::generation::get_world_gen;
use pumpkin_world::world::BlockRegistryExt;

mod superflat;

use superflat::Superflatten;

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

fn generate_chunk_data(seed: u64, chunk_x: i32, chunk_z: i32) -> Result<Arc<ChunkData>, String> {
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

#[pyfunction]
fn seed_to_sections_batch(seed: u64, coords: Vec<(i32, i32)>) -> Vec<Vec<u8>> {
    coords
        .into_par_iter()
        .map(|(chunk_x, chunk_z)| {
            let nbt =
                generate_chunk_data(seed, chunk_x, chunk_z).expect("Failed to generate chunk data");
            let sf = Superflatten::from_chunk_data(&nbt);
            let data = sf.dump_to_sections_other().0;
            zstd::encode_all(Cursor::new(&data), 19).expect("Failed to compress sections")
        })
        .collect()
}

#[pyfunction]
fn chunk_region_encode_batch(
    chunk_nbts: Vec<Vec<u8>>,
    sections_dumps: Vec<Vec<u8>>,
) -> Vec<(Vec<u8>, Vec<u8>)> {
    chunk_nbts
        .into_par_iter()
        .zip(sections_dumps.into_par_iter())
        .map(|(chunk_nbt, compressed_base_sections)| {
            let base_sections = zstd::decode_all(Cursor::new(compressed_base_sections))
                .expect("Failed to decompress sections");
            let (target_sections, other) = Superflatten::from_chunk_nbt(&chunk_nbt)
                .expect("Failed to parse nbt")
                .dump_to_sections_other();
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
    others: Vec<Vec<u8>>,
    sections_deltas: Vec<Vec<u8>>,
    sections_dumps: Vec<Vec<u8>>,
) -> Vec<Vec<u8>> {
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
            Superflatten::load_from_sections_other(&target_sections, &other)
                .to_chunk()
                .expect("Failed to chunk")
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
