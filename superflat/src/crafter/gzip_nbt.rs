use anyhow::{Context, Result};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::io::{Cursor, Read, Write};

use super::Crafter;
use crate::{
    odb::{OdbReader, OdbWriter},
    utils::nbt::{dump_nbt, load_nbt, sort_nbt},
};

const GZIP_NBT_GLOB_PATTERNS: &[&str] = &["**/*.dat"];

pub struct GzipNbtCrafter;

impl Crafter for GzipNbtCrafter {
    fn flatten(self, save: &impl OdbReader, storage: &mut impl OdbWriter) -> Result<()> {
        for pattern in GZIP_NBT_GLOB_PATTERNS {
            for key in save.glob(pattern)? {
                log::info!("Process gzip nbt file {key}");
                let compressed = save.get(&key)?;
                let mut decoder = GzDecoder::new(compressed.as_slice());
                let decompressed = if decoder.header().is_some() {
                    let mut decompressed = Vec::new();
                    decoder
                        .read_to_end(&mut decompressed)
                        .context("failed to decompress gzip data")?;
                    decompressed
                } else {
                    log::warn!(
                        "Failed to decompress because header is invalid, treat as uncompressed"
                    );
                    compressed
                };
                let sorted = dump_nbt(
                    sort_nbt(load_nbt(Cursor::new(&decompressed))?),
                    decompressed.len(),
                )?;
                storage.put(&key, &sorted)?;
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
