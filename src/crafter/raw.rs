use super::Crafter;
use crate::odb::{OdbReader, OdbWriter};

const RAW_GLOB_PATTERNS: &[&str] = &["icon.png", "advancements/*.json", "stats/*.json"];

pub struct RawCrafter;

impl Crafter for RawCrafter {
    async fn flatten(self, save: &impl OdbReader, storage: &mut impl OdbWriter) {
        for pattern in RAW_GLOB_PATTERNS {
            for key in save.glob(pattern).await {
                log::info!("Process raw file {key}");
                let data = save.get(&key).await;
                storage.put(&key, &data).await;
            }
        }
    }

    async fn unflatten(self, save: &mut impl OdbWriter, storage: &impl OdbReader) {
        for pattern in RAW_GLOB_PATTERNS {
            for key in storage.glob(pattern).await {
                let data = storage.get(&key).await;
                save.put(&key, &data).await;
            }
        }
    }
}
