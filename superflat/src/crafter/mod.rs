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
