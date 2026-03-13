use std::io::Cursor;

use serde::{Deserialize, Serialize};

use pumpkin_nbt::{Nbt, deserializer::NbtReadHelper, tag::NbtTag};

pub mod normalize;
pub mod region_crafter;
pub mod region_decode;

#[derive(Serialize, Deserialize)]
struct SectionsDump {
    biomes_dump: Vec<u8>,
    blocks_dump: Vec<u16>,
}

pub fn check_chunk_status_full(input: &[u8]) -> Result<bool, String> {
    let cursor = Cursor::new(input);
    let nbt = Nbt::read(&mut NbtReadHelper::new(cursor)).map_err(|e| e.to_string())?;
    let status = nbt
        .get_string("Status")
        .ok_or_else(|| "Chunk NBT does not have Status field".to_string())?;
    Ok(status == "minecraft:full")
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
