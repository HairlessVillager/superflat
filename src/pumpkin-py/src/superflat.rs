use std::io::Cursor;

use pumpkin_nbt::deserializer::NbtReadHelper;
use pumpkin_nbt::tag::NbtTag;
use pumpkin_nbt::{Nbt, from_bytes, normalize_nbt_bytes, to_bytes};
use pumpkin_world::chunk::ChunkSections;
use pumpkin_world::chunk::format::ChunkSectionNBT;
use pumpkin_world::chunk::{ChunkData, format::ChunkNbt};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
struct SectionsDump {
    biomes_dump: Vec<u8>,
    blocks_dump: Vec<u16>,
    // drop block & sky lighting because Minecraft will re-compute them
}

#[derive(Serialize, Deserialize)]
pub struct Superflatten {
    sections: SectionsDump,
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
            sections: {
                SectionsDump {
                    biomes_dump: chunk_data.section.dump_biomes(),
                    blocks_dump: chunk_data.section.dump_blocks(),
                }
            },
            other: None,
        }
    }
    pub fn from_chunk_nbt(chunk_nbt: &[u8]) -> Result<Self, &'static str> {
        let flatten = Self {
            sections: {
                let nbt = from_bytes::<ChunkNbt>(Cursor::new(chunk_nbt))
                    .map_err(|_| "Failed to parse chunk data when build thin")?;
                let chunk = ChunkData::from_nbt(nbt);
                SectionsDump {
                    biomes_dump: chunk.section.dump_biomes(),
                    blocks_dump: chunk.section.dump_blocks(),
                }
            },

            other: {
                let mut nbt = Nbt::read(&mut NbtReadHelper::new(Cursor::new(chunk_nbt)))
                    .map_err(|_| "Failed to parse chunk data when building other")?;

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

                Some(nbt)
            },
        };
        Ok(flatten)
    }
    pub fn to_chunk(self) -> Result<Vec<u8>, &'static str> {
        let mut chunk = self.other.ok_or("Cannot build chunk without other nbt")?;

        // rebuild sections from thin
        let sections = {
            let section = ChunkSections::from_blocks_biomes(
                &self.sections.blocks_dump,
                &self.sections.biomes_dump,
            );
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
    pub fn load_from_sections_other(sections: &[u8], other: &[u8]) -> Self {
        let sections =
            from_bytes::<SectionsDump>(Cursor::new(sections)).expect("Failed to load sections");
        let other =
            Nbt::read(&mut NbtReadHelper::new(Cursor::new(other))).expect("Failed to load other");
        Self {
            sections,
            other: Some(other),
        }
    }
    pub fn dump_to_sections_other(self) -> (Vec<u8>, Vec<u8>) {
        let mut sections = Vec::new(); // TODO: use .with_capacity here
        to_bytes(&self.sections, &mut sections).expect("Failed to dump thin data");
        let sections: Vec<u8> = normalize_nbt_bytes(&sections)
            .map(|v| v.into())
            .expect("Failed to normalize thin data");
        let other: Vec<u8> = self.other.map(|v| v.write().into()).unwrap_or(Vec::new());
        (sections, other)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::superflat::Superflatten;

    #[test]
    fn something_works() {
        let nbt =
            std::fs::read(Path::new("/home/hlsvillager/Desktop/superflat/temp/nbt2")).unwrap();
        let sf = Superflatten::from_chunk_nbt(&nbt).unwrap();
        sf.dump_to_sections_other();
    }
}
