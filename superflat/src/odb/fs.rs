use std::fs;
use std::path::PathBuf;

use rayon::iter::{IntoParallelIterator, ParallelIterator};

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
    fn get(&self, key: &str) -> Vec<u8> {
        fs::read(self.root_dir.join(key)).expect("failed to read file from odb")
    }

    fn get_par(&self, keys: &[&str]) -> Vec<Vec<u8>> {
        keys.into_par_iter().map(|key| self.get(key)).collect()
    }

    fn glob(&self, pattern: &str) -> Vec<String> {
        let full_pattern = self.root_dir.join(pattern);
        let root = self.root_dir.clone();
        glob::glob(full_pattern.to_str().expect("glob pattern path is not valid utf-8"))
            .expect("failed to run glob")
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
    fn put(&mut self, key: &str, value: impl AsRef<[u8]>) {
        let path = self.root_dir.join(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create parent directory");
        }
        fs::write(path, value).expect("failed to write file to odb");
    }

    fn put_par(&mut self, entries: impl IntoParallelIterator<Item = (String, impl AsRef<[u8]>)>) {
        entries.into_par_iter().for_each(|(key, value)| {
            let path = self.root_dir.join(&key);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("failed to create parent directory"); // TODO: create dir before par write
            }
            fs::write(path, value).expect("failed to write file to odb");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_put_get_roundtrip() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let mut odb = LocalFsOdb::from_dir(dir.path().to_path_buf());
        let data = b"hello superflat".to_vec();
        odb.put("foo/bar.bin", &data);
        let got = odb.get("foo/bar.bin");
        assert_eq!(got, data);
    }

    #[test]
    fn fs_glob_returns_matching_keys() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let mut odb = LocalFsOdb::from_dir(dir.path().to_path_buf());
        odb.put("a/x.txt", &b"1".to_vec());
        odb.put("a/y.txt", &b"2".to_vec());
        odb.put("b/z.bin", &b"3".to_vec());
        let mut matches = odb.glob("a/*.txt");
        matches.sort();
        assert_eq!(matches, vec!["a/x.txt", "a/y.txt"]);
    }
}
