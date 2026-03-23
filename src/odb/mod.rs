mod fs;
pub mod git;

pub use fs::LocalFsOdb;
pub use git::LocalGitOdb;

pub trait OdbReader {
    fn get(&self, key: &str) -> impl std::future::Future<Output = Vec<u8>>;
    fn glob(&self, pattern: &str) -> impl std::future::Future<Output = Vec<String>>;
}
pub trait OdbWriter: OdbReader {
    fn put(&mut self, key: &str, value: &[u8]) -> impl std::future::Future<Output = ()>;
}
