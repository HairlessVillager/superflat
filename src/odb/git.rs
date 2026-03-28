use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::odb::{OdbReader, OdbWriter};
use crate::utils::cmd::{exec_stdout, git_cmd};

pub struct LocalGitOdb {
    repo: gix::ThreadSafeRepository,
    /// Accumulated blobs not yet committed: path → sha1.
    pending: HashMap<String, String>,
    /// Blob path → oid, populated once per commit.
    path_to_oid: HashMap<String, gix::ObjectId>,
}

impl LocalGitOdb {
    pub fn new(git_dir: PathBuf) -> Self {
        Self {
            repo: gix::open(git_dir.to_owned())
                .expect(&format!(
                    "Try 'git init --bare {}'",
                    git_dir.to_str().unwrap()
                ))
                .into(),
            pending: HashMap::new(),
            path_to_oid: HashMap::new(),
        }
    }

    pub fn from_commit(git_dir: PathBuf, commit: String) -> Self {
        let repo: gix::ThreadSafeRepository = gix::open(git_dir).unwrap().into();
        let path_to_oid = if commit.is_empty() {
            HashMap::new()
        } else {
            build_path_to_oid(&repo, &commit)
        };
        Self {
            repo,
            pending: HashMap::new(),
            path_to_oid,
        }
    }

    fn git(&self) -> Command {
        git_cmd(self.repo.git_dir())
    }

    /// Create a commit from all pending blobs, consuming self.
    ///
    /// `parents` is a list of 0 or more commit-ish strings. The first becomes
    /// the `from` parent and the rest are additional `merge` parents.  Each is
    /// resolved with the `^0` suffix so that refs and tags are dereferenced to
    /// their underlying commit objects.
    ///
    /// Returns the sha1 of the new commit.
    pub fn commit(self, parents: &[impl AsRef<str>], message: &str) -> String {
        let tree_sha = build_tree(self.repo.git_dir(), &self.pending, "");

        let mut cmd = self.git();
        cmd.arg("commit-tree").arg(&tree_sha);
        for parent in parents {
            cmd.arg("-p").arg(&format!("{}^0", parent.as_ref()));
        }
        cmd.arg("-m").arg(message);

        let commit = exec_stdout(cmd, None).trim().to_string();
        commit
    }
}

/// Recursively build tree objects for `entries` rooted at `prefix`.
/// Returns the sha1 of the root tree.
fn build_tree(
    git_dir: &std::path::Path,
    entries: &HashMap<String, String>,
    prefix: &str,
) -> String {
    let mut blobs: Vec<(String, String)> = Vec::new();
    let mut dirs: std::collections::BTreeMap<String, HashMap<String, String>> =
        std::collections::BTreeMap::new();

    for (path, sha1) in entries {
        let rel = if prefix.is_empty() {
            path.as_str()
        } else {
            path.strip_prefix(&format!("{prefix}/")).unwrap_or(path)
        };
        if let Some((dir, _rest)) = rel.split_once('/') {
            dirs.entry(dir.to_string())
                .or_default()
                .insert(path.clone(), sha1.clone());
        } else {
            blobs.push((rel.to_string(), sha1.clone()));
        }
    }

    let mut dir_shas: Vec<(String, String)> = dirs
        .into_par_iter()
        .map(|(name, sub_entries)| {
            let sub_prefix = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            let sub_sha = build_tree(git_dir, &sub_entries, &sub_prefix);
            (name, sub_sha)
        })
        .collect();
    dir_shas.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    let mut mktree_input = String::new();
    for (name, sub_sha) in &dir_shas {
        mktree_input.push_str(&format!("040000 tree {sub_sha}\t{name}\n"));
    }
    for (name, sha1) in &blobs {
        mktree_input.push_str(&format!("100644 blob {sha1}\t{name}\n"));
    }

    let mut cmd = git_cmd(git_dir);
    cmd.args(["mktree"]);

    exec_stdout(cmd, Some(mktree_input)).trim().to_string()
}

/// Build a path → oid map for a commit using `git ls-tree -r`.
fn build_path_to_oid(
    repo: &gix::ThreadSafeRepository,
    commit_sha: &str,
) -> HashMap<String, gix::ObjectId> {
    let mut cmd = git_cmd(repo.git_dir());
    cmd.arg("--git-dir")
        .arg(repo.git_dir())
        .args(["ls-tree", "-r", "--", commit_sha]);
    exec_stdout(cmd, None)
        .lines()
        .filter_map(|line| {
            let oid_str = line.get(12..52)?;
            let path = line.get(53..)?.trim();
            let oid: gix::ObjectId = oid_str.parse().ok()?;
            Some((path.to_string(), oid))
        })
        .collect()
}

impl OdbReader for LocalGitOdb {
    fn get(&self, key: &str) -> Vec<u8> {
        let oid = self.path_to_oid.get(key).expect("key not found");
        self.repo
            .to_thread_local()
            .find_blob(*oid)
            .unwrap()
            .data
            .to_vec()
    }

    fn get_par(&self, keys: &[&str]) -> Vec<Vec<u8>> {
        let repo = self.repo.clone();
        let path_to_oid = &self.path_to_oid;
        keys.into_par_iter()
            .map(|key| {
                let oid = path_to_oid.get(*key).expect("key not found");
                repo.to_thread_local()
                    .find_blob(*oid)
                    .unwrap()
                    .data
                    .to_vec()
            })
            .collect()
    }

    fn glob(&self, pattern: &str) -> Vec<String> {
        let pat = glob::Pattern::new(pattern).unwrap();
        self.path_to_oid
            .par_iter()
            .map(|(p, _)| p)
            .filter(|p| pat.matches(p.as_str()))
            .cloned()
            .collect()
    }
}

impl OdbWriter for LocalGitOdb {
    fn put(&mut self, key: &str, value: impl AsRef<[u8]>) {
        let sha1 = self
            .repo
            .to_thread_local()
            .write_blob(value)
            .unwrap()
            .to_hex()
            .to_string();
        self.pending.insert(key.to_string(), sha1);
    }

    fn put_par(&mut self, entries: impl IntoParallelIterator<Item = (String, impl AsRef<[u8]>)>) {
        let ts_repo = self.repo.clone();
        let results: Vec<(String, String)> = entries
            .into_par_iter()
            .map(|(key, value)| {
                let repo = ts_repo.to_thread_local();
                let sha1 = repo
                    .write_blob(value.as_ref())
                    .unwrap()
                    .to_hex()
                    .to_string();
                (key, sha1)
            })
            .collect();
        for (key, sha1) in results {
            self.pending.insert(key, sha1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Initialise a bare git repo in a tempdir and return its path.
    fn init_bare_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init", "--bare", dir.path().to_str().unwrap()])
            .output()
            .unwrap();
        // git commit-tree needs author/committer config
        Command::new("git")
            .args(["--git-dir", dir.path().to_str().unwrap()])
            .args(["config", "user.email", "test@test"])
            .output()
            .unwrap();
        Command::new("git")
            .args(["--git-dir", dir.path().to_str().unwrap()])
            .args(["config", "user.name", "Test"])
            .output()
            .unwrap();
        dir
    }

    #[test]
    fn git_put_commit_get_roundtrip() {
        let repo = init_bare_repo();
        let mut odb = LocalGitOdb::from_commit(repo.path().to_path_buf(), String::new());

        let data = b"hello git odb".to_vec();
        odb.put("src/hello.txt", &data);
        let commit_sha = odb.commit(&[] as &[&str], "initial");
        assert_eq!(commit_sha.len(), 40);

        let odb = LocalGitOdb::from_commit(repo.path().to_path_buf(), commit_sha);
        let got = odb.get("src/hello.txt");
        assert_eq!(got, data);
    }

    #[test]
    fn git_glob_after_commit() {
        let repo = init_bare_repo();
        let mut odb = LocalGitOdb::from_commit(repo.path().to_path_buf(), String::new());

        odb.put("a/x.rs", &b"fn x(){}".to_vec());
        odb.put("a/y.rs", &b"fn y(){}".to_vec());
        odb.put("b/z.md", &b"# Z".to_vec());
        let commit_sha = odb.commit(&[] as &[&str], "add files");

        let odb = LocalGitOdb::from_commit(repo.path().to_path_buf(), commit_sha);
        let mut matches = odb.glob("a/*.rs");
        matches.sort();
        assert_eq!(matches, vec!["a/x.rs", "a/y.rs"]);
    }

    #[test]
    fn git_commit_with_parent() {
        let repo = init_bare_repo();
        let mut odb = LocalGitOdb::from_commit(repo.path().to_path_buf(), String::new());

        odb.put("a.txt", &b"v1".to_vec());
        let first = odb.commit(&[] as &[&str], "first");

        // Second commit only puts b.txt — a.txt is NOT inherited
        let mut odb = LocalGitOdb::from_commit(repo.path().to_path_buf(), first.clone());
        odb.put("b.txt", &b"v2".to_vec());
        let second = odb.commit(&[&first], "second");

        // second commit's tree contains only b.txt
        let files: Vec<String> = String::from_utf8(
            Command::new("git")
                .args(["--git-dir", repo.path().to_str().unwrap()])
                .args(["ls-tree", "--name-only", &second])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .lines()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(files, vec!["b.txt"]);

        // parent linkage is recorded
        let parent = String::from_utf8(
            Command::new("git")
                .args(["--git-dir", repo.path().to_str().unwrap()])
                .args(["rev-parse", &format!("{second}^1")])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        assert_eq!(parent.trim(), first);
    }
}
