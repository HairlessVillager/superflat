use std::io::Cursor;

use anyhow::Result;
use simdnbt::owned::{BaseNbt, Nbt, NbtCompound, NbtList, NbtTag, read};

pub fn load_nbt(mut data: Cursor<&[u8]>) -> Result<BaseNbt> {
    if let Nbt::Some(base) = read(&mut data)? {
        Ok(base)
    } else {
        Err(anyhow::anyhow!("nbt data is empty"))
    }
}

pub fn dump_nbt(nbt: BaseNbt, size: usize) -> Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(size);
    nbt.write(&mut buf);
    Ok(buf)
}

pub fn sort_nbt(nbt: BaseNbt) -> BaseNbt {
    fn sort_tag(tag: NbtTag) -> NbtTag {
        match tag {
            NbtTag::Compound(compound) => NbtTag::Compound(sort_compound(compound)),
            NbtTag::List(list) => NbtTag::List(sort_list(list)),
            other => other,
        }
    }
    fn sort_list(list: NbtList) -> NbtList {
        match list {
            NbtList::List(lists) => {
                NbtList::List(lists.into_iter().map(|list| sort_list(list)).collect())
            }
            NbtList::Compound(comps) => {
                NbtList::Compound(comps.into_iter().map(|comp| sort_compound(comp)).collect())
            }
            other => other,
        }
    }
    fn sort_compound(comp: NbtCompound) -> NbtCompound {
        let mut kvs = comp
            .into_iter()
            .map(|(k, v)| (k, sort_tag(v)))
            .collect::<Vec<_>>();
        kvs.sort_unstable_by(|(k1, _), (k2, _)| k1.as_bytes().cmp(k2.as_bytes()));
        NbtCompound::from_values(kvs)
    }
    let name = nbt.name().to_owned();
    let comp = sort_compound(nbt.as_compound());
    BaseNbt::new(name, comp)
}
