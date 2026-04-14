use std::{
    collections::HashMap,
    sync::{LazyLock, OnceLock},
};

use anyhow::{Context, Result};
use minecraft_data_rs::{
    Api,
    api::versions_by_minecraft_version,
    models::{biome::Biome, block::Block, block::StateType},
};

struct McData {
    blocks_by_name: HashMap<String, Block>,
    blocks_by_state_id: HashMap<u32, Block>,
    biomes_by_name: HashMap<String, Biome>,
    biomes_by_id: HashMap<u32, Biome>,
}

static MC_DATA: OnceLock<McData> = OnceLock::new();
static BOOL_VALUES: LazyLock<Vec<String>> =
    LazyLock::new(|| vec!["true".to_string(), "false".to_string()]);

pub fn init_mc_data(version: &str) {
    MC_DATA.get_or_init(|| {
        log::info!("Fetching Minecraft version list");
        let versions =
            versions_by_minecraft_version().expect("failed to load minecraft version list");
        let version = versions
            .get(version)
            .expect(&format!(
                "invalid Minecraft version: {version}, expect one of: {:?}",
                versions.keys()
            ))
            .clone();
        log::info!("Fetching Minecraft version data");
        let api = Api::new(version).expect("failed to fetch Minecraft ver");
        McData {
            blocks_by_name: api
                .blocks
                .blocks_by_name()
                .expect("failed to load blocks by name"),
            blocks_by_state_id: api
                .blocks
                .blocks_by_state_id()
                .expect("failed to load blocks by state id"),
            biomes_by_name: api
                .biomes
                .biomes_by_name()
                .expect("failed to load biomes by name"),
            biomes_by_id: api.biomes.biomes().expect("failed to load biomes by id"),
        }
    });
}

fn mc_data() -> &'static McData {
    MC_DATA
        .get()
        .expect("mc_data not initialized, call init_mc_data() first")
}

fn compute_state_id(block: &Block, props: &[(&str, &str)]) -> Result<u32> {
    let states = match block.states.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => {
            return block
                .default_state
                .context("data missing block.default_state");
        }
    };
    let min_state_id = block
        .min_state_id
        .context("data missing block.min_state_id")?;
    let mut offset = 0u32;
    let mut multiplier = 1u32;
    for state in states.iter().rev() {
        let value = props
            .iter()
            .find(|(k, _)| *k == state.name)
            .map(|(_, v)| *v)
            .with_context(|| {
                format!(
                    "block state {} is required, all states: {:#?}",
                    state.name, props
                )
            })?;
        let values = if let StateType::Bool = state.state_type {
            &BOOL_VALUES
        } else {
            state
                .values
                .as_ref()
                .context("data missing state.values")?
                .as_slice()
        };
        let idx = values.iter().position(|v| v == value).with_context(|| {
            format!(
                "invalid value '{}' for state '{}', expect one of {:?}",
                value, state.name, values
            )
        })? as u32;
        offset += idx * multiplier;
        multiplier *= state.num_values;
    }
    Ok(min_state_id + offset)
}

fn compute_props_from_state_id(
    block: &'static Block,
    state_id: u32,
) -> Result<Vec<(&'static str, &'static str)>> {
    // TODO: maybe use String for better perf
    let states = match block.states.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(vec![]),
    };
    let min_state_id = block
        .min_state_id
        .context("data missing block.min_state_id")?;
    anyhow::ensure!(
        state_id >= min_state_id,
        "expect state_id(={state_id}) >= min_state_id(={min_state_id})"
    );
    let mut relative = state_id - min_state_id;
    let mut result = Vec::with_capacity(states.len());
    for state in states.iter().rev() {
        let idx = (relative % state.num_values) as usize;
        relative /= state.num_values;
        let values = if let StateType::Bool = state.state_type {
            &BOOL_VALUES
        } else {
            state
                .values
                .as_ref()
                .context("data missing state.values")?
                .as_slice()
        };
        let value = values
            .as_ref()
            .get(idx)
            .map(|s| s.as_str())
            .with_context(|| format!("idx(={idx}) out of bound, values: {values:#?}"))?;
        result.push((state.name.as_str(), value));
    }
    Ok(result)
}

/// Returns biome registry ID from name (without `"minecraft:"` prefix).
pub fn biome_id_from_name(name: &str) -> Result<u8> {
    let biome = mc_data()
        .biomes_by_name
        .get(name)
        .with_context(|| format!("unknown biome name: {name}"))?;
    Ok(biome.id as u8)
}

/// Returns biome name (without `"minecraft:"` prefix) from registry ID.
pub fn biome_name_from_id(id: u8) -> Result<&'static str> {
    // TODO: maybe use String for better perf
    let biome = mc_data()
        .biomes_by_id
        .get(&(id as u32))
        .with_context(|| format!("unknown biome id: {id}"))?;
    Ok(biome.name.as_str())
}

/// Returns block state ID from block name (without `"minecraft:"` prefix) and properties.
pub fn block_state_id_from_name_and_props(name: &str, props: &[(&str, &str)]) -> Result<u16> {
    let block = mc_data()
        .blocks_by_name
        .get(name)
        .with_context(|| format!("unknown block name: {name}"))?;
    let default_state = if props.is_empty() {
        block
            .default_state
            .ok_or(anyhow::anyhow!("Data missing block.default_state"))?
    } else {
        compute_state_id(block, props).with_context(|| format!("block.name: {}", block.name))?
    };
    Ok(default_state as u16)
}

/// Returns block name (without `"minecraft:"` prefix) and properties from state ID.
pub fn block_name_and_props_from_state_id(
    state_id: u16,
) -> Result<(String, Vec<(&'static str, &'static str)>)> {
    let block: &'static Block = mc_data()
        .blocks_by_state_id
        .get(&(state_id as u32))
        .with_context(|| format!("unknown block state id: {state_id}"))?;
    let props = compute_props_from_state_id(block, state_id as u32)?;
    Ok((block.name.clone(), props))
}
