use pumpkin_config::lighting::LightingEngineConfig;
use pumpkin_nbt::deserializer::NbtReadHelper;
use pumpkin_nbt::{Nbt, normalize_nbt_bytes};
use pumpkin_world::chunk::ChunkData;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::{prelude::*, wrap_pyfunction};

use rayon::prelude::*;
use std::io::Cursor;
use std::sync::Arc;

use pumpkin_data::dimension::Dimension;
use pumpkin_util::world_seed::Seed;
use pumpkin_world::biome::hash_seed;
use pumpkin_world::chunk::format::anvil::SingleChunkDataSerializer;
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

fn sync_generate_chunk_nbt(seed: u64, chunk_x: i32, chunk_z: i32) -> Result<Vec<u8>, String> {
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
        futures::executor::block_on(async {
            chunk
                .to_bytes()
                .await
                .map(|v| v.into())
                .map_err(|e| e.to_string())
        })
    } else {
        Err("Failed to upgrade chunk to Level stage".to_string())
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
fn generate_chunk_nbt(seed: u64, chunk_x: i32, chunk_z: i32) -> PyResult<Vec<u8>> {
    sync_generate_chunk_nbt(seed, chunk_x, chunk_z).map_err(|e| PyValueError::new_err(e))
}

#[pyfunction]
fn batch_generate_chunk_nbt(seed: u64, coords: Vec<(i32, i32)>) -> PyResult<Vec<Vec<u8>>> {
    coords
        .into_par_iter()
        .map(|(x, z)| {
            let nbt = sync_generate_chunk_nbt(seed, x, z).map_err(|e| PyValueError::new_err(e))?;
            let normalized = normalize_nbt(&nbt)?;
            Ok(normalized)
        })
        .collect::<PyResult<Vec<Vec<u8>>>>()
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
fn sf_from_nbt(nbt: &[u8]) -> PyResult<Vec<u8>> {
    Ok(Superflatten::from_chunk_nbt(nbt)
        .map_err(|e| PyRuntimeError::new_err(e))?
        .dump_to_nbt()
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?)
}

#[pyfunction]
fn sf_from_seed(seed: u64, chunk_x: i32, chunk_z: i32) -> PyResult<Vec<u8>> {
    let nbt =
        generate_chunk_data(seed, chunk_x, chunk_z).map_err(|e| PyRuntimeError::new_err(e))?;
    let sf = Superflatten::from_chunk_data(&nbt);
    sf.dump_to_nbt()
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

#[pyfunction]
fn sf_to_chunk(superflat: &[u8]) -> PyResult<Vec<u8>> {
    Ok(Superflatten::load_from_nbt(superflat)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?
        .to_chunk()
        .map_err(|e| PyRuntimeError::new_err(e))?)
}

#[pyfunction]
fn sf_from_seed_batch(seed: u64, coords: Vec<(i32, i32)>) -> PyResult<Vec<Vec<u8>>> {
    coords
        .into_par_iter()
        .map(|(chunk_x, chunk_z)| {
            let nbt = generate_chunk_data(seed, chunk_x, chunk_z)
                .map_err(|e| PyRuntimeError::new_err(e))?;
            let sf = Superflatten::from_chunk_data(&nbt);
            sf.dump_to_nbt()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        })
        .collect::<PyResult<Vec<Vec<u8>>>>()
}

#[pyfunction]
fn chunk_to_sections_other(nbt: &[u8]) -> (Vec<u8>, Vec<u8>) {
    Superflatten::from_chunk_nbt(nbt)
        .expect("Failed to parse nbt")
        .dump_to_sections_other()
}

#[pyfunction]
fn seed_to_sections(seed: u64, chunk_x: i32, chunk_z: i32) -> Vec<u8> {
    let nbt = generate_chunk_data(seed, chunk_x, chunk_z).expect("Failed to generate chunk data");
    let sf = Superflatten::from_chunk_data(&nbt);
    sf.dump_to_sections_other().0
}

#[pyfunction]
fn sections_other_to_chunk(sections: &[u8], other: &[u8]) -> Vec<u8> {
    Superflatten::load_from_sections_other(sections, other)
        .to_chunk()
        .expect("Failed to chunk")
}

#[pymodule]
fn pumpkin_py(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(generate_chunk_nbt, m)?)?;
    m.add_function(wrap_pyfunction!(batch_generate_chunk_nbt, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_nbt, m)?)?;
    m.add_function(wrap_pyfunction!(is_chunk_status_full, m)?)?;
    m.add_function(wrap_pyfunction!(sf_from_nbt, m)?)?;
    m.add_function(wrap_pyfunction!(sf_from_seed, m)?)?;
    m.add_function(wrap_pyfunction!(sf_to_chunk, m)?)?;
    m.add_function(wrap_pyfunction!(sf_from_seed_batch, m)?)?;
    m.add_function(wrap_pyfunction!(chunk_to_sections_other, m)?)?;
    m.add_function(wrap_pyfunction!(seed_to_sections, m)?)?;
    m.add_function(wrap_pyfunction!(sections_other_to_chunk, m)?)?;
    Ok(())
}
