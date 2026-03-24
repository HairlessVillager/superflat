use std::io::Write;
use std::process::{Command, Stdio};
use std::{collections::HashMap, path::PathBuf};

use crate::odb::{OdbReader, OdbWriter};

pub struct LocalGitOdb {
    repo: gix::Repository,
    /// Current commit used for read operations (get/glob).
    commit: Option<String>,
    /// Accumulated blobs not yet committed: path → sha1.
    pending: HashMap<String, String>,
}

impl LocalGitOdb {
    pub fn new(git_dir: PathBuf) -> Self {
        Self {
            repo: gix::open(git_dir).unwrap(),
            commit: None,
            pending: HashMap::new(),
        }
    }

    pub fn from_commit(git_dir: PathBuf, commit: String) -> Self {
        Self {
            repo: gix::open(git_dir).unwrap(),
            commit: Some(commit).filter(|s| !s.is_empty()),
            pending: HashMap::new(),
        }
    }

    fn git(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.arg("--git-dir").arg(self.repo.git_dir());
        cmd
    }

    /// Create a commit from all pending blobs, consuming self.
    ///
    /// `parents` is a list of 0 or more commit-ish strings. The first becomes
    /// the `from` parent and the rest are additional `merge` parents.  Each is
    /// resolved with the `^0` suffix so that refs and tags are dereferenced to
    /// their underlying commit objects.
    ///
    /// Returns the sha1 of the new commit.
    pub fn commit(
        self,
        parents: &[impl AsRef<str>],
        message: &str,
        r#ref: Option<String>,
    ) -> String {
        let tree_sha = build_tree(self.repo.git_dir(), &self.pending, "");

        let mut cmd = self.git();
        cmd.arg("commit-tree").arg(&tree_sha);
        for parent in parents {
            cmd.arg("-p").arg(&format!("{}^0", parent.as_ref()));
        }
        cmd.arg("-m").arg(message);

        let output = cmd.output().unwrap();
        let commit = String::from_utf8(output.stdout).unwrap().trim().to_string();

        if let Some(r#ref) = r#ref {
            self.git()
                .arg("update-ref")
                .arg(r#ref)
                .arg(&commit)
                .status()
                .unwrap();
        }

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

    let mut mktree_input = String::new();
    for (name, sub_entries) in &dirs {
        let sub_prefix = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let sub_sha = build_tree(git_dir, sub_entries, &sub_prefix);
        mktree_input.push_str(&format!("040000 tree {sub_sha}\t{name}\n"));
    }
    for (name, sha1) in &blobs {
        mktree_input.push_str(&format!("100644 blob {sha1}\t{name}\n"));
    }

    let mut child = Command::new("git")
        .args(["--git-dir", git_dir.to_str().unwrap()])
        .args(["mktree"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(mktree_input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

impl OdbReader for LocalGitOdb {
    fn get(&self, key: &str) -> Vec<u8> {
        let commit_sha = self.commit.as_deref().expect("No commit set");
        let commit_id: gix::ObjectId = commit_sha.parse().unwrap();

        let tree_id = {
            let commit = self.repo.find_commit(commit_id).unwrap();
            commit.tree_id().unwrap().detach()
        };
        let blob_id = {
            let tree = self.repo.find_tree(tree_id).unwrap();
            let entry = tree.lookup_entry_by_path(key).unwrap().unwrap();
            entry.object_id()
        };
        self.repo.find_blob(blob_id).unwrap().data.to_vec()
    }

    fn glob(&self, pattern: &str) -> Vec<String> {
        if let Some(commit) = &self.commit {
            let output = self
                .git()
                .args(["ls-tree", "-r", "--name-only", &commit])
                .output()
                .unwrap();
            let pat = glob::Pattern::new(pattern).unwrap();
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| pat.matches(line))
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl OdbWriter for LocalGitOdb {
    fn put(&mut self, key: &str, value: impl AsRef<[u8]>) {
        let sha1 = self.repo.write_blob(value).unwrap().to_hex().to_string();
        self.pending.insert(key.to_string(), sha1);
    }

    fn put_par(&mut self, entries: impl IntoParallelIterator<Item = (String, impl AsRef<[u8]>)>) {
        let git_dir = self.repo.git_dir().to_path_buf();
        let results: Vec<(String, String)> = entries
            .into_par_iter()
            .map(|(key, value)| {
                let repo = gix::open(&git_dir).unwrap();
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

use rayon::iter::{IntoParallelIterator, ParallelIterator};

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
        let commit_sha = odb.commit(&[] as &[&str], "initial", None);
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
        let commit_sha = odb.commit(&[] as &[&str], "add files", None);

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
        let first = odb.commit(&[] as &[&str], "first", None);

        // Second commit only puts b.txt — a.txt is NOT inherited
        let mut odb = LocalGitOdb::from_commit(repo.path().to_path_buf(), first.clone());
        odb.put("b.txt", &b"v2".to_vec());
        let second = odb.commit(&[&first], "second", None);

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
