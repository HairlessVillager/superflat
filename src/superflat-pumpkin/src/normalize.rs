use std::collections::HashMap;

use bytes::Bytes;
use pumpkin_nbt::{Error, Nbt, compound::NbtCompound, deserializer::NbtReadHelper, tag::NbtTag};

trait Normalize {
    fn normalize(self) -> Self;
}

impl Normalize for NbtCompound {
    /// Normalizes the compound by sorting `child_tags` by key name in lexicographical order
    /// and recursively normalizing any nested compound or list structures.
    fn normalize(mut self) -> Self {
        // Sort child_tags by key name
        self.child_tags.sort_by(|a, b| a.0.cmp(&b.0));

        // Recursively normalize nested structures
        for (_, tag) in &mut self.child_tags {
            let placeholder = NbtTag::End;
            let normalized_tag = std::mem::replace(tag, placeholder).normalize();
            let _placeholder = std::mem::replace(tag, normalized_tag);
        }

        self
    }
}

impl Normalize for NbtTag {
    /// Normalizes the NBT tag by recursively sorting any compound structures
    /// and their nested elements in lexicographical order by key name.
    fn normalize(self) -> Self {
        match self {
            // Self::String(old_s) => Self::String(
            //     string_mapping
            //         .as_ref()
            //         .and_then(|m| m.get(&old_s))
            //         .cloned()
            //         .unwrap_or(old_s),
            // ),
            Self::Compound(compound) => Self::Compound(compound.normalize()),
            Self::List(list) => {
                let normalized_list: Vec<Self> =
                    list.into_iter().map(|tag| tag.normalize()).collect();
                Self::List(normalized_list)
            }
            // All other types don't contain nested structures, so return as-is
            other => other,
        }
    }
}

/// Normalizes NBT data by sorting compound tag keys in lexicographical order.
///
/// This function takes raw NBT bytes, deserializes them, sorts all compound tag
/// key-value pairs by key name in lexicographical order (recursively for nested
/// structures), and then re-serializes the data back to bytes.
///
/// # Arguments
/// * `bytes` - The input NBT data as bytes
///
/// # Returns
/// * `Result<Bytes, Error>` - The normalized NBT data, or an error if deserialization/serialization fails
///
/// # Example
/// ```rust
/// use pumpkin_nbt::normalize_nbt_bytes;
/// # let nbt_data: &[u8] = &[0x0A, 0x00, 0x00, 0x00]; // Example NBT bytes
/// let normalized = normalize_nbt_bytes(&nbt_data).unwrap();
/// ```
pub fn normalize_nbt_bytes(
    bytes: &[u8],
    block_id_mapping: Option<&HashMap<&str, &str>>,
) -> Result<Bytes, String> {
    use std::io::Cursor;

    let (is_named, nbt) = {
        let cursor = Cursor::new(bytes);
        let nbt_result = Nbt::read(&mut NbtReadHelper::new(cursor));

        // Try to deserialize as named NBT first
        if let Ok(nbt) = nbt_result {
            // Successfully parsed as named NBT
            (true, nbt)
            // Ok(normalized_nbt.write())
        } else {
            // Try as unnamed NBT
            let cursor = Cursor::new(bytes);
            let nbt = Nbt::read_unnamed(&mut NbtReadHelper::new(cursor))
                .map_err(|e| format!("Failed to parse nbt as named or unnamed: {}", e))?;
            (false, nbt)
            // let normalized_nbt = Nbt::new(nbt.name, nbt.root_tag.normalize());
            // Ok(normalized_nbt.write_unnamed())
        }
    };

    let normalized_nbt = Nbt::new(nbt.name, nbt.root_tag.normalize());

    if let Some(block_id_mapping) = block_id_mapping {
        let sections = normalized_nbt
            .get_list("sections")
            .ok_or_else(|| format!("Failed to mapping block id: missing sections field"))?;
        for (section_idx, section) in sections.iter().enumerate() {
            if let NbtTag::Compound(section) = section {
                let block_states = section.get_compound("block_states").ok_or_else(|| {
                    format!(
                        "Failed to mapping block id: missing sections.{}.block_states field",
                        section_idx
                    )
                })?;
                let palette = block_states.get_list("palette").ok_or_else(|| {
                    format!(
                        "Failed to mapping block id: missing sections.{}.block_states.palette field",
                        section_idx
                    )
                })?;
                for (block_state_id, block_state) in palette.iter().enumerate() {
                    if let NbtTag::Compound(block_state) = block_state {
                        let block_id = block_state.get_string("Name").ok_or_else(|| {
                            format!(
                                "Failed to mapping block id: missing sections.{}.block_states.palette.{}.Name field",
                                section_idx, block_state_id
                            )
                        })?;
                    } else {
                        return Err(format!(
                            "Failed to mapping block id: sections.{}.block_states.palette.{} field is not a NbtTag::Compound: {:?}",
                            section_idx, block_state_id, block_state
                        ));
                    }
                }
            } else {
                return Err(format!(
                    "Failed to mapping block id: sections.{} field is not a NbtTag::Compound: {:?}",
                    section_idx, section
                ));
            }
        }
    }

    let bytes = if is_named {
        normalized_nbt.write()
    } else {
        normalized_nbt.write_unnamed()
    };
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use pumpkin_nbt::{from_bytes_unnamed, to_bytes_unnamed};
    use serde::{Deserialize, Serialize};

    #[test]
    fn normalize_nbt_bytes_works() {
        use crate::normalize_nbt_bytes;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        #[allow(clippy::struct_field_names)]
        struct TestStruct {
            z_field: String,
            a_field: i32,
            m_field: bool,
        }

        let test_data = TestStruct {
            z_field: "last".to_string(),
            a_field: 42,
            m_field: true,
        };

        let mut bytes = Vec::new();
        to_bytes_unnamed(&test_data, &mut bytes).unwrap();
        let normalized_bytes = normalize_nbt_bytes(&bytes).unwrap();
        let reconstructed: TestStruct =
            from_bytes_unnamed(std::io::Cursor::new(normalized_bytes.clone())).unwrap();
        assert_eq!(test_data, reconstructed);

        let normalized_again = normalize_nbt_bytes(&normalized_bytes).unwrap();
        assert_eq!(
            normalized_bytes, normalized_again,
            "Normalize should be idempotent"
        );

        let mut bytes2 = Vec::new();
        to_bytes_unnamed(&test_data, &mut bytes2).unwrap();
        let normalized_bytes2 = normalize_nbt_bytes(&bytes2).unwrap();
        assert_eq!(
            normalized_bytes, normalized_bytes2,
            "Same data should normalize to same bytes"
        );
    }

    #[test]
    fn normalize_nested_compounds() {
        use crate::normalize_nbt_bytes;
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Inner {
            z_inner: i32,
            a_inner: String,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Outer {
            z_outer: Inner,
            a_outer: Inner,
        }

        let test_data = Outer {
            z_outer: Inner {
                z_inner: 1,
                a_inner: "first".to_string(),
            },
            a_outer: Inner {
                z_inner: 2,
                a_inner: "second".to_string(),
            },
        };

        // Serialize to bytes
        let mut bytes = Vec::new();
        to_bytes_unnamed(&test_data, &mut bytes).unwrap();

        // Normalize the bytes
        let normalized_bytes = normalize_nbt_bytes(&bytes).unwrap();

        // Deserialize back and verify it's the same data
        let reconstructed: Outer =
            from_bytes_unnamed(std::io::Cursor::new(normalized_bytes)).unwrap();
        assert_eq!(test_data, reconstructed);
    }

    #[test]
    fn normalize_with_lists() {
        use crate::normalize_nbt_bytes;
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestStruct {
            z_field: String,
            a_field: i32,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestWithList {
            z_list: Vec<TestStruct>,
            a_single: TestStruct,
        }

        let test_data = TestWithList {
            z_list: vec![
                TestStruct {
                    z_field: "item1".to_string(),
                    a_field: 1,
                },
                TestStruct {
                    z_field: "item2".to_string(),
                    a_field: 2,
                },
            ],
            a_single: TestStruct {
                z_field: "single".to_string(),
                a_field: 3,
            },
        };

        // Serialize to bytes
        let mut bytes = Vec::new();
        to_bytes_unnamed(&test_data, &mut bytes).unwrap();

        // Normalize the bytes
        let normalized_bytes = normalize_nbt_bytes(&bytes).unwrap();

        // Deserialize back and verify it's the same data
        let reconstructed: TestWithList =
            from_bytes_unnamed(std::io::Cursor::new(normalized_bytes)).unwrap();
        assert_eq!(test_data, reconstructed);
    }
}

#[cfg(test)]
mod tests_pumpkin_world_gen {
    use futures::executor::block_on;
    use pumpkin_data::dimension::Dimension;
    use pumpkin_util::world_seed::Seed;
    use pumpkin_world::biome::hash_seed;
    use pumpkin_world::chunk::format::anvil::SingleChunkDataSerializer;
    use pumpkin_world::chunk_system::{Chunk, StagedChunkEnum, generate_single_chunk};
    use pumpkin_world::generation::get_world_gen;
    use pumpkin_world::world::BlockRegistryExt;
    use std::sync::Arc;

    use crate::normalize::normalize_nbt_bytes;

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

    #[test]
    fn generate_chunk_should_return() {
        let dimension = Dimension::OVERWORLD;
        let seed = Seed(42);
        let block_registry = Arc::new(BlockRegistry);
        let world_gen = get_world_gen(seed, dimension);
        let biome_mixer_seed = hash_seed(world_gen.random_config.seed);

        let _ = generate_single_chunk(
            &dimension,
            biome_mixer_seed,
            &world_gen,
            block_registry.as_ref(),
            0,
            0,
            StagedChunkEnum::Full,
        );
    }

    async fn is_chunks_identical(chunk1: &Chunk, chunk2: &Chunk) -> bool {
        let (Chunk::Level(level1), Chunk::Level(level2)) = (chunk1, chunk2) else {
            panic!("Expected Level chunks");
        };

        let nbt1 = normalize_nbt_bytes(&level1.to_bytes().await.unwrap()).unwrap();
        let nbt2 = normalize_nbt_bytes(&level2.to_bytes().await.unwrap()).unwrap();
        if nbt1 == nbt2 {
            return true;
        }

        if level1.x != level2.x {
            println!("Chunk X coordinates differ");
            return false;
        }
        if level1.z != level2.z {
            println!("Chunk Z coordinates differ");
            return false;
        }

        let mut different_flag = false;

        let blocks1 = level1.section.dump_blocks();
        let blocks2 = level2.section.dump_blocks();
        if blocks1 != blocks2 {
            let sections1 = level1.section.block_sections.read().unwrap();
            let sections2 = level2.section.block_sections.read().unwrap();
            for (sec_idx, (sec1, sec2)) in sections1.iter().zip(sections2.iter()).enumerate() {
                for z in 0..16 {
                    for y in 0..16 {
                        for x in 0..16 {
                            let b1 = sec1.get(x, y, z);
                            let b2 = sec2.get(x, y, z);
                            if b1 != b2 {
                                different_flag = true;
                                println!(
                                    "Different on block: section index: {}, local XYZ: ({}, {}, {}), {} != {}",
                                    sec_idx, x, y, z, b1, b2
                                );
                            }
                        }
                    }
                }
            }
        }

        let biomes1 = level1.section.dump_biomes();
        let biomes2 = level2.section.dump_biomes();
        if biomes1 != biomes2 {
            let sections1 = level1.section.biome_sections.read().unwrap();
            let sections2 = level2.section.biome_sections.read().unwrap();
            for (sec_idx, (sec1, sec2)) in sections1.iter().zip(sections2.iter()).enumerate() {
                for z in 0..4 {
                    for y in 0..4 {
                        for x in 0..4 {
                            let b1 = sec1.get(x, y, z);
                            let b2 = sec2.get(x, y, z);
                            if b1 != b2 {
                                different_flag = true;
                                println!(
                                    "Different on biome: section index: {}, local XYZ: ({}, {}, {}), {} != {}",
                                    sec_idx, x, y, z, b1, b2
                                );
                            }
                        }
                    }
                }
            }
        }

        let heightmap1 = level1.heightmap.lock().unwrap();
        let heightmap2 = level2.heightmap.lock().unwrap();
        if heightmap1.world_surface.as_ref() != heightmap2.world_surface.as_ref() {
            println!("World surface heightmap differs",);
            different_flag = true;
        }
        if heightmap1.motion_blocking.as_ref() != heightmap2.motion_blocking.as_ref() {
            println!("Motion blocking heightmap differs",);
            different_flag = true;
        }
        if heightmap1.motion_blocking_no_leaves.as_ref()
            != heightmap2.motion_blocking_no_leaves.as_ref()
        {
            println!("Motion blocking no leaves heightmap differs",);
            different_flag = true;
        }

        !different_flag
    }

    #[tokio::test]
    #[ignore = "very slow, should be tested under release profile (-r)"]
    async fn slow_generate_chunk_should_identical() {
        use rayon::prelude::*;

        let chunk_x = 669;
        let chunk_z = 473;
        let world_seed = 657830420;

        let dimension = Dimension::OVERWORLD;
        let block_registry = Arc::new(BlockRegistry);
        let world_gen = get_world_gen(Seed(world_seed), dimension);
        let biome_mixer_seed = hash_seed(world_gen.random_config.seed);

        let initial_chunk = generate_single_chunk(
            &dimension,
            biome_mixer_seed,
            &world_gen,
            block_registry.as_ref(),
            chunk_x,
            chunk_z,
            StagedChunkEnum::Full,
        );

        let all_match = (0..3000).into_par_iter().all(|_| {
            let compared_chunk = generate_single_chunk(
                &dimension,
                biome_mixer_seed,
                &world_gen,
                block_registry.as_ref(),
                chunk_x,
                chunk_z,
                StagedChunkEnum::Full,
            );

            block_on(is_chunks_identical(&initial_chunk, &compared_chunk))
        });

        assert!(
            all_match,
            "Found at least one chunk that is different from the initial chunk"
        );
    }
}
