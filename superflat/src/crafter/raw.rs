use anyhow::Result;

use super::Crafter;
use crate::odb::{OdbReader, OdbWriter};

const RAW_GLOB_PATTERNS: &[&str] = &["**/*.png", "**/*.json"];

pub struct RawCrafter;

impl Crafter for RawCrafter {
    fn flatten(self, save: &impl OdbReader, storage: &mut impl OdbWriter) -> Result<()> {
        for pattern in RAW_GLOB_PATTERNS {
            for key in save.glob(pattern)? {
                log::info!("Process raw file {key}");
                let data = save.get(&key)?;
                storage.put(&key, &data)?;
            }
        }
        Ok(())
    }

    fn unflatten(self, save: &mut impl OdbWriter, storage: &impl OdbReader) -> Result<()> {
        for pattern in RAW_GLOB_PATTERNS {
            for key in storage.glob(pattern)? {
                log::info!("Process raw file {key}");
                let data = storage.get(&key)?;
                save.put(&key, &data)?;
            }
        }
        Ok(())
    }
}
