mod fs;
pub mod git;

pub use fs::LocalFsOdb;
pub use git::LocalGitOdb;
use rayon::iter::IntoParallelIterator;

pub trait OdbReader {
    fn get(&self, key: &str) -> Vec<u8>;
    fn glob(&self, pattern: &str) -> Vec<String>;
}
pub trait OdbWriter: OdbReader {
    fn put(&mut self, key: &str, value: impl AsRef<[u8]>);

    fn put_par(&mut self, entries: impl IntoParallelIterator<Item = (String, impl AsRef<[u8]>)>);
}
