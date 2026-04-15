mod fs;
pub mod git;

pub use fs::LocalFsOdb;
pub use git::LocalGitOdb;
use rayon::iter::IntoParallelIterator;

use anyhow::Result;

pub trait OdbReader {
    fn get(&self, key: &str) -> Result<Vec<u8>>;
    fn get_par(&self, keys: &[&str]) -> Result<Vec<Vec<u8>>>;
    fn glob(&self, pattern: &str) -> Result<Vec<String>>;
}
pub trait OdbWriter: OdbReader {
    fn put(&mut self, key: &str, value: impl AsRef<[u8]>) -> Result<()>;
    fn put_par(&mut self, entries: impl IntoParallelIterator<Item = (String, impl AsRef<[u8]>)>) -> Result<()>;
}
