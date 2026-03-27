// Copyright (C) 2026
// - HairlessVillager <64526732+HairlessVillager@users.noreply.github.com>
//
// Licensed under the GNU General Public License v3.0
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.gnu.org/licenses/gpl-3.0.html
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: GPL-3.0

use std::{
    collections::HashMap,
    sync::{LazyLock, OnceLock},
};

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
        let versions =
            versions_by_minecraft_version().expect("failed to load minecraft version list");
        let version = versions
            .get(version)
            .expect(&format!(
                "invalid Minecraft version: {version}, expect one of: {:?}",
                versions.keys()
            ))
            .clone();
        let api = Api::new(version).unwrap();
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
        .expect("mc_data not initialized — call init_mc_data() first")
}

fn compute_state_id(block: &Block, props: &[(&str, &str)]) -> u32 {
    let states = match block.states.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => {
            return block.default_state.unwrap();
        }
    };
    let min_state_id = block.min_state_id.unwrap();
    let mut offset = 0u32;
    let mut multiplier = 1u32;
    for state in states.iter().rev() {
        let value = props
            .iter()
            .find(|(k, _)| *k == state.name)
            .map(|(_, v)| *v)
            .unwrap();
        let values = if let StateType::Bool = state.state_type {
            &BOOL_VALUES
        } else {
            state.values.as_ref().unwrap().as_slice()
        };
        let idx = values.iter().position(|v| v == value).expect(&format!(
            "invalid value for {}.{}: {}, expect one of {:?}",
            block.name, state.name, value, values
        )) as u32;
        offset += idx * multiplier;
        multiplier *= state.num_values;
    }
    min_state_id + offset
}

fn compute_props_from_state_id(
    block: &'static Block,
    state_id: u32,
) -> Vec<(&'static str, &'static str)> {
    // TODO: maybe use String for better perf
    let states = match block.states.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => return vec![],
    };
    let min_state_id = block.min_state_id.unwrap();
    let mut relative = state_id.checked_sub(min_state_id).unwrap();
    let mut result = Vec::with_capacity(states.len());
    for state in states.iter().rev() {
        let idx = (relative % state.num_values) as usize;
        relative /= state.num_values;
        let values = if let StateType::Bool = state.state_type {
            &BOOL_VALUES
        } else {
            state.values.as_ref().unwrap().as_slice()
        };
        let value = values.as_ref().get(idx).map(|s| s.as_str()).unwrap();
        result.push((state.name.as_str(), value));
    }
    result
}

/// Returns biome registry ID from name (without `"minecraft:"` prefix).
pub fn biome_id_from_name(name: &str) -> u8 {
    mc_data()
        .biomes_by_name
        .get(name)
        .expect(&format!("unknown biome name: {name}"))
        .id as u8
}

/// Returns biome name (without `"minecraft:"` prefix) from registry ID.
pub fn biome_name_from_id(id: u8) -> &'static str {
    // TODO: maybe use String for better perf
    mc_data()
        .biomes_by_id
        .get(&(id as u32))
        .expect(&format!("unknown biome id: {id}"))
        .name
        .as_str()
}

/// Returns block state ID from block name (without `"minecraft:"` prefix) and properties.
pub fn block_state_id_from_name_and_props(name: &str, props: &[(&str, &str)]) -> u16 {
    let block = mc_data()
        .blocks_by_name
        .get(name)
        .expect(&format!("unknown block name: {name}"));
    if props.is_empty() {
        block.default_state.unwrap() as u16
    } else {
        compute_state_id(block, props) as u16
    }
}

/// Returns block name (without `"minecraft:"` prefix) and properties from state ID.
pub fn block_name_and_props_from_state_id(
    state_id: u16,
) -> (String, Vec<(&'static str, &'static str)>) {
    let block: &'static Block = mc_data()
        .blocks_by_state_id
        .get(&(state_id as u32))
        .expect(&format!("unknown block state id: {state_id}"));
    let props = compute_props_from_state_id(block, state_id as u32);
    (block.name.clone(), props)
}
