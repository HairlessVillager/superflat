use std::path::PathBuf;

use crate::odb::{OdbReader, OdbWriter};

pub struct LocalFsOdb {
    root_dir: PathBuf,
}

impl LocalFsOdb {
    pub fn from_dir(root: PathBuf) -> Self {
        Self { root_dir: root }
    }
}

impl OdbReader for LocalFsOdb {
    async fn get(&self, key: &str) -> Vec<u8> {
        tokio::fs::read(self.root_dir.join(key)).await.unwrap()
    }

    async fn glob(&self, pattern: &str) -> Vec<String> {
        let full_pattern = self.root_dir.join(pattern);
        let root = self.root_dir.clone();
        glob::glob(full_pattern.to_str().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter_map(|path| {
                path.strip_prefix(&root)
                    .ok()
                    .and_then(|p| p.to_str().map(|s| s.to_string()))
            })
            .collect()
    }
}

impl OdbWriter for LocalFsOdb {
    async fn put(&mut self, key: &str, value: &[u8]) {
        let path = self.root_dir.join(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.unwrap();
        }
        tokio::fs::write(path, value).await.unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fs_put_get_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut odb = LocalFsOdb::from_dir(dir.path().to_path_buf());
        let data = b"hello superflat".to_vec();
        odb.put("foo/bar.bin", &data).await;
        let got = odb.get("foo/bar.bin").await;
        assert_eq!(got, data);
    }

    #[tokio::test]
    async fn fs_glob_returns_matching_keys() {
        let dir = tempfile::tempdir().unwrap();
        let mut odb = LocalFsOdb::from_dir(dir.path().to_path_buf());
        odb.put("a/x.txt", &b"1".to_vec()).await;
        odb.put("a/y.txt", &b"2".to_vec()).await;
        odb.put("b/z.bin", &b"3".to_vec()).await;
        let mut matches = odb.glob("a/*.txt").await;
        matches.sort();
        assert_eq!(matches, vec!["a/x.txt", "a/y.txt"]);
    }
}
