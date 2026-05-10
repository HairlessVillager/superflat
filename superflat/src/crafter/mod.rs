mod chunk_region;
mod entities_region;
mod gzip_nbt;
mod poi_region;
mod raw;

use anyhow::Result;
pub(crate) use chunk_region::ChunkRegionCrafter;
pub(crate) use entities_region::EntitiesRegionCrafter;
pub(crate) use gzip_nbt::GzipNbtCrafter;
pub(crate) use poi_region::PoiRegionCrafter;
pub(crate) use raw::RawCrafter;
use versions::Versioning;

use crate::odb::{OdbReader, OdbWriter};

pub(crate) trait Crafter {
    fn flatten(self, save_dir: &impl OdbReader, storage: &mut impl OdbWriter) -> Result<()>;
    fn unflatten(self, save_dir: &mut impl OdbWriter, storage: &impl OdbReader) -> Result<()>;
}

pub(crate) enum CrafterImpl {
    Raw(RawCrafter),
    GzipNbt(GzipNbtCrafter),
    ChunkRegion(ChunkRegionCrafter),
    EntitiesRegion(EntitiesRegionCrafter),
    PoiRegion(PoiRegionCrafter),
}

impl CrafterImpl {
    pub(crate) fn get_crafters(version: Versioning) -> Vec<Self> {
        vec![
            Self::ChunkRegion(ChunkRegionCrafter {}),
            Self::EntitiesRegion(EntitiesRegionCrafter {}),
            Self::PoiRegion(PoiRegionCrafter {}),
            Self::Raw(RawCrafter {}),
            Self::GzipNbt(GzipNbtCrafter { version }),
        ]
    }
}

impl Crafter for CrafterImpl {
    fn flatten(self, save_dir: &impl OdbReader, storage: &mut impl OdbWriter) -> Result<()> {
        match self {
            Self::Raw(c) => c.flatten(save_dir, storage),
            Self::GzipNbt(c) => c.flatten(save_dir, storage),
            Self::ChunkRegion(c) => c.flatten(save_dir, storage),
            Self::EntitiesRegion(c) => c.flatten(save_dir, storage),
            Self::PoiRegion(c) => c.flatten(save_dir, storage),
        }
    }
    fn unflatten(self, save_dir: &mut impl OdbWriter, storage: &impl OdbReader) -> Result<()> {
        match self {
            Self::Raw(c) => c.unflatten(save_dir, storage),
            Self::GzipNbt(c) => c.unflatten(save_dir, storage),
            Self::ChunkRegion(c) => c.unflatten(save_dir, storage),
            Self::EntitiesRegion(c) => c.unflatten(save_dir, storage),
            Self::PoiRegion(c) => c.unflatten(save_dir, storage),
        }
    }
}
