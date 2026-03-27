// Copyright (C) 2026
// - Alexander Medvedev <lilalexmed@proton.me>
// - HairlessVillager <64526732+HairlessVillager@users.noreply.github.com>
// - unschlagbar <153451111+unschlagbar@users.noreply.github.com>
// - 4lve <72332750+4lve@users.noreply.github.com>
// - kralverde <80051564+kralverde@users.noreply.github.com>
// - Clicks <58398364+CuzImClicks@users.noreply.github.com>
// - FabseGP <fabse_utilities@pm.me>
// - ioterw <iotrwewe12@protonmail.com>
// - teknostom <42341753+teknostom@users.noreply.github.com>
//
// Modified work Copyright (C) 2026 HairlessVillager <64526732+HairlessVillager@users.noreply.github.com>
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

use std::{hash::Hash, iter::repeat_n};

use pumpkin_data::{Block, chunk::Biome};
use pumpkin_nbt::{compound::NbtCompound, tag::NbtTag};
use pumpkin_util::encompassing_bits;

/// 3d array indexed by y,z,x
type AbstractCube<T, const DIM: usize> = [[[T; DIM]; DIM]; DIM];

#[derive(Clone)]
pub struct HeterogeneousPaletteData<V: Hash + Eq + Copy, const DIM: usize> {
    cube: Box<AbstractCube<V, DIM>>,
    palette: Vec<V>,
    counts: Vec<u16>,
}

/// A paletted container is a cube of registry ids. It uses a custom compression scheme based on how
/// may distinct registry ids are in the cube.
#[derive(Clone)]
pub enum PalettedContainer<V: Hash + Eq + Copy + Default, const DIM: usize> {
    Homogeneous(V),
    Heterogeneous(Box<HeterogeneousPaletteData<V, DIM>>),
}

impl<V: Hash + Eq + Copy + Default, const DIM: usize> PalettedContainer<V, DIM> {
    pub const VOLUME: usize = DIM * DIM * DIM;

    fn from_cube(cube: Box<AbstractCube<V, DIM>>) -> Self {
        let mut palette: Vec<V> = Vec::new();
        let mut counts: Vec<u16> = Vec::new();

        // Iterate over the flattened cube to populate the palette and counts
        for val in cube.as_flattened().as_flattened() {
            if let Some(index) = palette.iter().position(|v| v == val) {
                // Value already exists, increment its count
                counts[index] += 1;
            } else {
                // New value, add it to the palette and start its count
                palette.push(*val);
                counts.push(1);
            }
        }

        if palette.len() == 1 {
            // Fast path: the cube is homogeneous, so we can store just one value
            Self::Homogeneous(palette[0])
        } else {
            // Heterogeneous cube, store the full data
            Self::Heterogeneous(Box::new(HeterogeneousPaletteData {
                cube,
                palette,
                counts,
            }))
        }
    }

    fn bits_per_entry(&self) -> u8 {
        match self {
            Self::Homogeneous(_) => 0,
            Self::Heterogeneous(data) => encompassing_bits(data.counts.len()),
        }
    }

    pub fn to_palette_and_packed_data(&self, bits_per_entry: u8) -> (Box<[V]>, Box<[i64]>) {
        match self {
            Self::Homogeneous(registry_id) => (Box::new([*registry_id]), Box::new([])),
            Self::Heterogeneous(data) => {
                debug_assert!(bits_per_entry >= encompassing_bits(data.counts.len()));
                debug_assert!(bits_per_entry <= 15);

                // Don't use HashMap's here, because its slow
                let blocks_per_i64 = 64 / bits_per_entry;

                let packed_indices: Box<[i64]> = data
                    .cube
                    .as_flattened()
                    .as_flattened()
                    .chunks(blocks_per_i64 as usize)
                    .map(|chunk| {
                        chunk.iter().enumerate().fold(0, |acc, (index, key)| {
                            let key_index = data.palette.iter().position(|&x| x == *key).unwrap();
                            debug_assert!((1 << bits_per_entry) > key_index);

                            let packed_offset_index =
                                (key_index as u64) << (bits_per_entry as u64 * index as u64);
                            acc | packed_offset_index as i64
                        })
                    })
                    .collect();

                (data.palette.clone().into_boxed_slice(), packed_indices)
            }
        }
    }

    #[must_use]
    pub fn from_palette_and_packed_data(
        palette: Vec<V>,
        packed_data: Option<&[i64]>,
        minimum_bits_per_entry: u8,
    ) -> Self {
        if palette.is_empty() {
            // warn!("No palette data! Defaulting...");
            return Self::Homogeneous(V::default());
        }

        if palette.len() == 1 {
            return Self::Homogeneous(palette[0]);
        }
        let packed_data = packed_data.unwrap();

        let bits_per_key = encompassing_bits(palette.len()).max(minimum_bits_per_entry);
        let index_mask = (1 << bits_per_key) - 1;
        let keys_per_i64 = 64 / bits_per_key;

        let mut decompressed_values = Vec::with_capacity(Self::VOLUME);

        // We already have the palette from the input `palette_slice`.
        // The counts will be created in the next step.

        let mut packed_data_iter = packed_data.iter();
        let mut current_packed_word = *packed_data_iter.next().unwrap_or(&0);

        for i in 0..Self::VOLUME {
            let bit_index_in_word = i % keys_per_i64 as usize;

            if bit_index_in_word == 0 && i > 0 {
                current_packed_word = *packed_data_iter.next().unwrap_or(&0);
            }

            let lookup_index = (current_packed_word as u64
                >> (bit_index_in_word as u64 * bits_per_key as u64))
                & index_mask;

            let value = palette
                .get(lookup_index as usize)
                .copied()
                .unwrap_or_else(|| {
                    // warn!("Lookup index out of bounds! Defaulting...");
                    V::default()
                });

            decompressed_values.push(value);
        }

        // Now, with all decompressed values, build the counts.
        let mut counts = vec![0; palette.len()];

        for &value in &decompressed_values {
            // This is the key optimization: find the index in the palette Vec
            // and increment the corresponding count.
            if let Some(index) = palette.iter().position(|v| v == &value) {
                counts[index] += 1;
            } else {
                // This case should ideally not happen if the palette is complete.
                // warn!("Decompressed value not found in palette!");
            }
        }

        let mut cube = Box::new([[[V::default(); DIM]; DIM]; DIM]);
        cube.as_flattened_mut()
            .as_flattened_mut()
            .copy_from_slice(&decompressed_values);

        Self::Heterogeneous(Box::new(HeterogeneousPaletteData {
            cube,
            palette,
            counts,
        }))
    }

    pub fn iter(&self) -> Box<dyn Iterator<Item = &V> + '_> {
        match self {
            Self::Homogeneous(registry_id) => Box::new(repeat_n(registry_id, Self::VOLUME)),
            Self::Heterogeneous(data) => Box::new(data.cube.as_flattened().as_flattened().iter()),
        }
    }

    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = V>,
    {
        let mut cube = Box::new([[[V::default(); DIM]; DIM]; DIM]);
        let flattened = cube.as_flattened_mut().as_flattened_mut();
        for (i, val) in iter.into_iter().enumerate() {
            if i >= Self::VOLUME {
                break;
            }
            flattened[i] = val;
        }
        Self::from_cube(cube)
    }
}

impl<V: Default + Hash + Eq + Copy, const DIM: usize> Default for PalettedContainer<V, DIM> {
    fn default() -> Self {
        Self::Homogeneous(V::default())
    }
}

impl BiomePalette {
    #[must_use]
    pub fn from_disk_nbt(nbt: &NbtCompound) -> Self {
        let palette = nbt
            .get_list("palette")
            .unwrap()
            .into_iter()
            .map(|entry| {
                let s = entry.extract_string().unwrap();
                let key = s.strip_prefix("minecraft:").unwrap_or(s);
                Biome::from_name(key).unwrap().id // TODO: remove depends on pumpkin-data
            })
            .collect::<Vec<_>>();

        Self::from_palette_and_packed_data(palette, nbt.get_long_array("data"), BIOME_DISK_MIN_BITS)
    }

    #[must_use]
    pub fn to_disk_nbt(&self) -> NbtCompound {
        #[expect(clippy::unnecessary_min_or_max)]
        let bits_per_entry = self.bits_per_entry().max(BIOME_DISK_MIN_BITS);
        let (palette, packed_data) = self.to_palette_and_packed_data(bits_per_entry);

        let palette = NbtTag::List(
            palette
                .into_iter()
                .map(|registry_id| {
                    NbtTag::String(format!(
                        "minecraft:{}",
                        Biome::from_id(registry_id).unwrap().registry_id
                    ))
                })
                .collect(),
        );

        NbtCompound {
            child_tags: if packed_data.is_empty() {
                vec![("palette".into(), palette)]
            } else {
                vec![
                    ("data".into(), NbtTag::LongArray(packed_data.to_vec())),
                    ("palette".into(), palette),
                ]
            },
        }
    }
}

impl BlockPalette {
    #[must_use]
    pub fn from_disk_nbt(nbt: &NbtCompound) -> Self {
        let palette = nbt
            .get_list("palette")
            .unwrap()
            .into_iter()
            .map(|entry| {
                let entry = entry.extract_compound().unwrap();
                let block = {
                    let block_name = entry.get_string("Name").unwrap();
                    Block::from_name(block_name)
                        .expect(format!("unknown block name: {block_name}").as_str()) // TODO: remove depends on pumpkin-data
                };
                if let Some(props) = entry.get_compound("Properties") {
                    let props_map = props
                        .child_tags
                        .iter()
                        .map(|(key, value)| (key.as_str(), value.extract_string().unwrap()))
                        .collect::<Vec<_>>();
                    block.from_properties(&props_map).to_state_id(block) // TODO: remove depends on pumpkin-data
                } else {
                    return block.default_state.id;
                }
            })
            .collect::<Vec<_>>();

        Self::from_palette_and_packed_data(palette, nbt.get_long_array("data"), BLOCK_DISK_MIN_BITS)
    }

    pub fn to_disk_nbt(&self) -> NbtCompound {
        let bits_per_entry = self.bits_per_entry().max(BLOCK_DISK_MIN_BITS);
        let (palette, packed_data) = self.to_palette_and_packed_data(bits_per_entry);

        let palette = NbtTag::List(
            palette
                .into_iter()
                .map(Self::block_state_id_to_palette_entry)
                .map(NbtTag::Compound)
                .collect(),
        );

        NbtCompound {
            child_tags: if packed_data.is_empty() {
                vec![("palette".into(), palette)]
            } else {
                vec![
                    ("data".into(), NbtTag::LongArray(packed_data.to_vec())),
                    ("palette".into(), palette),
                ]
            },
        }
    }

    fn block_state_id_to_palette_entry(registry_id: u16) -> NbtCompound {
        let block = Block::from_state_id(registry_id);

        let child_tags = if let Some(props) = block.properties(registry_id) {
            let props = props
                .to_props()
                .into_iter()
                .map(|(k, v)| (k.to_string(), NbtTag::String(v.to_string())))
                .collect::<Vec<_>>();
            vec![
                ("Name".into(), NbtTag::String(block.name.into())),
                (
                    "Properties".into(),
                    NbtTag::Compound(NbtCompound { child_tags: props }),
                ),
            ]
        } else {
            vec![("Name".into(), NbtTag::String(block.name.into()))]
        };

        NbtCompound { child_tags }
    }
}

// According to the wiki, palette serialization for disk and network is different. Disk
// serialization always uses a palette if greater than one entry. Network serialization packs ids
// directly instead of using a palette above a certain bits-per-entry

// TODO: Do our own testing; do we really need to handle network and disk serialization differently?
pub type BlockPalette = PalettedContainer<u16, 16>;
const BLOCK_DISK_MIN_BITS: u8 = 4;

pub type BiomePalette = PalettedContainer<u8, 4>;
const BIOME_DISK_MIN_BITS: u8 = 0;
