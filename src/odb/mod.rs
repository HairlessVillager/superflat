mod fs;
mod git;

pub use fs::LocalFsOdb;
pub use git::LocalGitOdb;

pub trait OdbReader {
    async fn get(&self, key: &str) -> Vec<u8>;
    async fn glob(&self, pattern: &str) -> Vec<String>;
}
pub trait OdbWriter: OdbReader {
    async fn put(&mut self, key: &str, value: &[u8]);
}
