use std::io::{Read, Seek};

use bytes::Bytes;
use pumpkin_nbt::{Nbt, compound::NbtCompound, deserializer::NbtReadHelper, tag::NbtTag};

fn sort_nbt_compound(compound: NbtCompound) -> NbtCompound {
    let mut normalized: Vec<(String, NbtTag)> = compound
        .child_tags
        .into_iter()
        .map(|(field, tag)| (field, sort_nbt_tag(tag)))
        .collect();
    normalized.sort_by(|a, b| a.0.cmp(&b.0));
    NbtCompound {
        child_tags: normalized,
    }
}

fn sort_nbt_tag(tag: NbtTag) -> NbtTag {
    match tag {
        NbtTag::Compound(compound) => NbtTag::Compound(sort_nbt_compound(compound)),
        NbtTag::List(list) => {
            let normalized: Vec<NbtTag> = list.into_iter().map(|tag| sort_nbt_tag(tag)).collect();
            NbtTag::List(normalized)
        }
        other => other,
    }
}

pub fn sort_nbt(nbt: Nbt) -> Nbt {
    Nbt::new(nbt.name, sort_nbt_compound(nbt.root_tag))
}

pub fn load_nbt<R: Read + Seek>(reader: R, named: bool) -> Nbt {
    let mut helper = NbtReadHelper::new(reader);
    if named {
        Nbt::read(&mut helper).expect("failed to read named nbt")
    } else {
        Nbt::read_unnamed(&mut helper).expect("failed to read unnamed nbt")
    }
}

pub fn dump_nbt(nbt: Nbt, named: bool) -> Bytes {
    if named {
        nbt.write()
    } else {
        nbt.write_unnamed()
    }
}
