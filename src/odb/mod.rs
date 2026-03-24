mod fs;
pub mod git;

pub use fs::LocalFsOdb;
pub use git::LocalGitOdb;

pub trait OdbReader {
    fn get(&self, key: &str) -> Vec<u8>;
    fn glob(&self, pattern: &str) -> Vec<String>;
}
pub trait OdbWriter: OdbReader {
    fn put(&mut self, key: &str, value: &[u8]);

    fn put_many(&mut self, entries: Vec<(String, Vec<u8>)>) {
        for (key, value) in entries {
            self.put(&key, &value);
        }
    }
}
