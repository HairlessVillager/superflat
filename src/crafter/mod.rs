use crate::odb::{OdbReader, OdbWriter};

mod chunk_region;
mod gzip_nbt;
mod other_region;
mod raw;

pub use chunk_region::ChunkRegionCrafter;
pub use gzip_nbt::GzipNbtCrafter;
pub use other_region::OtherRegionCrafter;
pub use raw::RawCrafter;

pub trait Crafter {
    async fn flatten(self, save_dir: &impl OdbReader, storage: &mut impl OdbWriter);
    async fn unflatten(self, save_dir: &mut impl OdbWriter, storage: &impl OdbReader);
}
