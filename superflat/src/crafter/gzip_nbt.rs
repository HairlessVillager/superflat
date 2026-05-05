use anyhow::{Context, Result};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use simdnbt::owned::{BaseNbt, NbtCompound, NbtList};
use std::io::{Cursor, Read, Write};
use versions::Versioning;

use super::Crafter;
use crate::{
    odb::{OdbReader, OdbWriter},
    utils::nbt::{dump_nbt, load_nbt, sort_nbt},
};

const GZIP_NBT_GLOB_PATTERNS: &[&str] = &["**/*.dat"];

pub struct GzipNbtCrafter {
    pub version: Versioning,
}

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
                let sorted = {
                    let nbt = load_nbt(Cursor::new(&decompressed))?;
                    let nbt = sort_nbt(nbt);

                    // Sort recipe book for player data
                    let nbt = {
                        let field_exists = &self.version
                            >= &Versioning::new("1.12").context("failed to parse version")?;
                        let match_file_old = &self.version
                            < &Versioning::new("26.1").context("failed to parse version")?
                            && glob::Pattern::new("playerdata/*.dat")
                                .context("failed to compile glob pattern")?
                                .matches(&key);
                        let match_file_new = &self.version
                            >= &Versioning::new("26.1").context("failed to parse version")?
                            && glob::Pattern::new("players/data/*.dat")
                                .context("failed to compile glob pattern")?
                                .matches(&key);
                        if field_exists && (match_file_old || match_file_new) {
                            let name = nbt.name().to_owned();
                            let mut comp = nbt.as_compound();
                            sort_recipe_book(&mut comp);
                            BaseNbt::new(name, comp)
                        } else {
                            nbt
                        }
                    };

                    // Sort recipe book for player data in level.dat
                    let nbt = {
                        if &self.version
                            < &Versioning::new("26.1").context("failed to parse version")?
                            && glob::Pattern::new("level.dat")
                                .context("failed to compile glob pattern")?
                                .matches(&key)
                        {
                            let name = nbt.name().to_owned();
                            let mut comp = nbt.as_compound();
                            if let Some(data) = comp.compound_mut("Data") {
                                if let Some(player) = data.compound_mut("Player") {
                                    sort_recipe_book(player);
                                } else {
                                    log::warn!("Field 'Data.Player' does not exist");
                                }
                            } else {
                                log::warn!("Field 'Data' does not exist");
                            }
                            BaseNbt::new(name, comp)
                        } else {
                            nbt
                        }
                    };

                    dump_nbt(nbt, decompressed.len())?
                };
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

fn sort_recipe_book(comp: &mut NbtCompound) {
    if let Some(recipe_book) = comp.compound_mut("recipeBook") {
        for key in ["recipes", "toBeDisplayed"] {
            if let Some(NbtList::String(strings)) = recipe_book.list_mut(key) {
                strings.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            }
        }
    } else {
        log::warn!("Field 'recipeBook' does not exist");
    }
}
