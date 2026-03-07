use std::io::Cursor;

use pumpkin_nbt::Error as NbtError;
use pumpkin_nbt::deserializer::NbtReadHelper;
use pumpkin_nbt::tag::NbtTag;
use pumpkin_nbt::{Nbt, from_bytes, normalize_nbt_bytes, to_bytes};
use pumpkin_world::chunk::ChunkSections;
use pumpkin_world::chunk::format::ChunkSectionNBT;
use pumpkin_world::chunk::{ChunkData, format::ChunkNbt};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
struct ThinChunk {
    biomes_dump: Vec<u8>,
    blocks_dump: Vec<u16>,
    // drop block & sky lighting because Minecraft will re-compute them
}

#[derive(Serialize, Deserialize)]
pub struct Superflatten {
    thin: ThinChunk,
    #[serde(with = "nbt_opt_codec")]
    other: Option<Nbt>,
}

mod nbt_opt_codec {
    use super::*;
    use serde::de::Error;

    pub fn serialize<S>(opt: &Option<Nbt>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes_opt = opt.as_ref().map(|nbt| nbt.clone().write());
        bytes_opt.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Nbt>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes_opt: Option<Vec<u8>> = Option::deserialize(deserializer)?;
        match bytes_opt {
            Some(bytes) => Nbt::read(&mut NbtReadHelper::new(Cursor::new(bytes)))
                .map(Some)
                .map_err(D::Error::custom),
            None => Ok(None),
        }
    }
}

impl Superflatten {
    pub fn from_chunk_data(chunk_data: &ChunkData) -> Self {
        Self {
            thin: {
                ThinChunk {
                    biomes_dump: chunk_data.section.dump_biomes(),
                    blocks_dump: chunk_data.section.dump_blocks(),
                }
            },
            other: None,
        }
    }
    pub fn from_chunk_nbt(chunk_nbt: &[u8]) -> Result<Self, &'static str> {
        let flatten = Self {
            thin: {
                let nbt = from_bytes::<ChunkNbt>(Cursor::new(chunk_nbt))
                    .map_err(|_| "Failed to parse chunk data when build thin")?;
                let chunk = ChunkData::from_nbt(nbt);
                ThinChunk {
                    biomes_dump: chunk.section.dump_biomes(),
                    blocks_dump: chunk.section.dump_blocks(),
                }
            },

            other: {
                let mut nbt = Nbt::read(&mut NbtReadHelper::new(Cursor::new(chunk_nbt)))
                    .map_err(|_| "Failed to parse chunk data when building other")?;
                let _ = nbt.root_tag.child_tags.pop_if(|(key, _)| key == "sections");
                Some(nbt)
            },
        };
        Ok(flatten)
    }
    pub fn to_chunk(self) -> Result<Vec<u8>, &'static str> {
        let mut chunk = self.other.ok_or("Cannot build chunk without other nbt")?;

        // rebuild sections from thin
        let sections = {
            let section =
                ChunkSections::from_blocks_biomes(&self.thin.blocks_dump, &self.thin.biomes_dump);
            let block_lock = section.block_sections.read().unwrap();
            let biome_lock = section.biome_sections.read().unwrap();
            let min_section_y = (section.min_y >> 4) as i8;

            (0..section.count)
                .map(|i| ChunkSectionNBT {
                    y: i as i8 + min_section_y,
                    block_states: Some(block_lock[i].to_disk_nbt()),
                    biomes: Some(biome_lock[i].to_disk_nbt()),
                    block_light: None,
                    sky_light: None,
                })
                .map(|nbt| {
                    let mut bytes: Vec<u8> = Vec::new();
                    to_bytes(&nbt, &mut bytes)
                        .map_err(|_| "Failed to serialize ChunkSectionNBT to bytes")?;
                    let nbt = Nbt::read(&mut NbtReadHelper::new(Cursor::new(bytes)))
                        .map_err(|_| "Failed to build NBT from ChunkSectionNBT bytes")?;
                    Ok(NbtTag::Compound(nbt.root_tag))
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        // insert to other nbt
        chunk.root_tag.put_list("sections", sections);

        Ok(chunk.write().into())
    }
    pub fn load_from_nbt(nbt: &[u8]) -> Result<Self, NbtError> {
        // let nbt = zstd::decode_all(Cursor::new(&nbt)).expect("Error on zstd decode");
        from_bytes(Cursor::new(nbt))
    }
    pub fn dump_to_nbt(&self) -> Result<Vec<u8>, NbtError> {
        let mut bytes = Vec::new(); // TODO: use .with_capacity here
        to_bytes(self, &mut bytes)?;
        let bytes: Vec<u8> = normalize_nbt_bytes(&bytes).map(|v| v.into())?;
        // let bytes = if compress {
        //     zstd::encode_all(Cursor::new(&bytes), 0).expect("Error on zstd encode")
        // } else {
        //     bytes
        // };
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::Superflatten;
    use crate::sync_generate_chunk_nbt;

    #[test]
    fn to_chunk_works() {
        let data = sync_generate_chunk_nbt(42, 0, 0).unwrap();
        let sf = Superflatten::from_chunk_nbt(&data).unwrap();
        sf.to_chunk().unwrap();
    }
}
