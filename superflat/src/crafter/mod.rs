mod chunk_region;
mod gzip_nbt;
mod other_region;
mod raw;

use anyhow::Result;
pub use chunk_region::ChunkRegionCrafter;
pub use gzip_nbt::GzipNbtCrafter;
pub use other_region::OtherRegionCrafter;
pub use raw::RawCrafter;

use crate::odb::{OdbReader, OdbWriter};

pub trait Crafter {
    fn flatten(self, save_dir: &impl OdbReader, storage: &mut impl OdbWriter) -> Result<()>;
    fn unflatten(self, save_dir: &mut impl OdbWriter, storage: &impl OdbReader) -> Result<()>;
}

pub enum CrafterImpl {
    Raw(RawCrafter),
    GzipNbt(GzipNbtCrafter),
    ChunkRegion(ChunkRegionCrafter),
    OtherRegion(OtherRegionCrafter),
}

impl CrafterImpl {
    pub fn get_crafters() -> [Self; 4] {
        [
            Self::Raw(RawCrafter {}),
            Self::GzipNbt(GzipNbtCrafter {}),
            Self::ChunkRegion(ChunkRegionCrafter {}),
            Self::OtherRegion(OtherRegionCrafter {}),
        ]
    }
}

impl Crafter for CrafterImpl {
    fn flatten(self, save_dir: &impl OdbReader, storage: &mut impl OdbWriter) -> Result<()> {
        match self {
            Self::Raw(c) => c.flatten(save_dir, storage),
            Self::GzipNbt(c) => c.flatten(save_dir, storage),
            Self::ChunkRegion(c) => c.flatten(save_dir, storage),
            Self::OtherRegion(c) => c.flatten(save_dir, storage),
        }
    }
    fn unflatten(self, save_dir: &mut impl OdbWriter, storage: &impl OdbReader) -> Result<()> {
        match self {
            Self::Raw(c) => c.unflatten(save_dir, storage),
            Self::GzipNbt(c) => c.unflatten(save_dir, storage),
            Self::ChunkRegion(c) => c.unflatten(save_dir, storage),
            Self::OtherRegion(c) => c.unflatten(save_dir, storage),
        }
    }
}
