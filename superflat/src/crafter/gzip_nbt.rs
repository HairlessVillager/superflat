use anyhow::{Context, Result};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::io::{Read, Write};

use super::Crafter;
use crate::odb::{OdbReader, OdbWriter};

const GZIP_NBT_GLOB_PATTERNS: &[&str] = &[
    "level.dat",
    "data/idcounts.dat",
    "data/command_storage_*.dat",
    "data/map_*.dat",
    "data/scoreboard.dat",
    "data/stopwatches.dat",
    "generated/*/structures/*.nbt",
    "playerdata/*.dat",
    // root dimension data
    "data/chunks.dat",
    "data/raids.dat",
    "data/raids_end.dat",
    "data/random_sequences.dat",
    "data/world_border.dat",
    // DIM1
    "DIM1/data/chunks.dat",
    "DIM1/data/raids.dat",
    "DIM1/data/raids_end.dat",
    "DIM1/data/random_sequences.dat",
    "DIM1/data/world_border.dat",
    // DIM-1
    "DIM-1/data/chunks.dat",
    "DIM-1/data/raids.dat",
    "DIM-1/data/raids_end.dat",
    "DIM-1/data/random_sequences.dat",
    "DIM-1/data/world_border.dat",
    // custom dimensions
    "dimensions/*/*/data/chunks.dat",
    "dimensions/*/*/data/raids.dat",
    "dimensions/*/*/data/raids_end.dat",
    "dimensions/*/*/data/random_sequences.dat",
    "dimensions/*/*/data/world_border.dat",
];

pub struct GzipNbtCrafter;

impl Crafter for GzipNbtCrafter {
    fn flatten(self, save: &impl OdbReader, storage: &mut impl OdbWriter) -> Result<()> {
        for pattern in GZIP_NBT_GLOB_PATTERNS {
            for key in save.glob(pattern)? {
                log::info!("Process gzip nbt file {key}");
                let compressed = save.get(&key)?;
                let mut decoder = GzDecoder::new(compressed.as_slice());
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .context("failed to decompress gzip data")?;
                storage.put(&key, &decompressed)?;
            }
        }
        Ok(())
    }

    fn unflatten(self, save: &mut impl OdbWriter, storage: &impl OdbReader) -> Result<()> {
        for pattern in GZIP_NBT_GLOB_PATTERNS {
            for key in storage.glob(pattern)? {
                log::info!("Process gzip nbt file {key}");
                let data = storage.get(&key)?;
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder
                    .write_all(&data)
                    .context("failed to write data to gzip encoder")?;
                let compressed = encoder
                    .finish()
                    .context("failed to finish gzip compression")?;
                save.put(&key, &compressed)?;
            }
        }
        Ok(())
    }
}
