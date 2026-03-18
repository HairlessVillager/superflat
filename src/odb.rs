use std::collections::HashMap;
use std::path::PathBuf;

type Object = Vec<u8>;

pub trait OdbReader {
    async fn get(&self, key: &str) -> Object;
    async fn glob(&self, pattern: &str) -> Vec<String>;
}
pub trait OdbWriter: OdbReader {
    async fn put(&mut self, key: &str, value: &Object);
}

pub struct LocalFsOdb {
    root_dir: PathBuf,
}

impl LocalFsOdb {
    pub fn from_dir(root: PathBuf) -> Self {
        Self { root_dir: root }
    }
}

impl OdbReader for LocalFsOdb {
    async fn get(&self, key: &str) -> Object {
        tokio::fs::read(self.root_dir.join(key)).await.unwrap()
    }

    async fn glob(&self, pattern: &str) -> Vec<String> {
        let full_pattern = self.root_dir.join(pattern);
        let root = self.root_dir.clone();
        tokio::task::spawn_blocking(move || -> Vec<String> {
            glob::glob(full_pattern.to_str().unwrap())
                .unwrap()
                .filter_map(|e| e.ok())
                .filter_map(|path| {
                    path.strip_prefix(&root)
                        .ok()
                        .and_then(|p| p.to_str().map(|s| s.to_string()))
                })
                .collect()
        })
        .await
        .unwrap()
    }
}

impl OdbWriter for LocalFsOdb {
    async fn put(&mut self, key: &str, value: &Object) {
        let path = self.root_dir.join(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.unwrap();
        }
        tokio::fs::write(path, value).await.unwrap();
    }
}

pub struct LocalGitOdb {
    git_dir: PathBuf,
    /// Current commit used for read operations (get/glob).
    commit: String,
    /// Accumulated blobs not yet committed: path → sha1.
    pending: HashMap<String, String>,
}

impl LocalGitOdb {
    pub fn new(git_dir: PathBuf, commit: String) -> Self {
        Self {
            git_dir,
            commit,
            pending: HashMap::new(),
        }
    }

    fn git(&self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new("git");
        cmd.args(["--git-dir", self.git_dir.to_str().unwrap().into()]);
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
    pub async fn commit(self, parents: &[&str], message: &str) -> String {
        let Self {
            git_dir, pending, ..
        } = self;
        let tree_sha = build_tree(&git_dir, &pending, "").await;

        let mut cmd = tokio::process::Command::new("git");
        cmd.args(["--git-dir", git_dir.to_str().unwrap()]);
        cmd.arg("commit-tree").arg(&tree_sha);
        for parent in parents {
            cmd.args(["-p", &format!("{}^0", parent)]);
        }
        cmd.args(["-m", message]);

        let output = cmd.output().await.unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }
}

/// Recursively build tree objects for `entries` rooted at `prefix`.
/// Returns the sha1 of the root tree.
async fn build_tree(git_dir: &PathBuf, entries: &HashMap<String, String>, prefix: &str) -> String {
    use tokio::io::AsyncWriteExt;

    let mut blobs: Vec<(String, String)> = Vec::new();
    let mut dirs: std::collections::BTreeMap<String, HashMap<String, String>> =
        std::collections::BTreeMap::new();

    for (path, sha1) in entries {
        let rel = if prefix.is_empty() {
            path.as_str()
        } else {
            path.strip_prefix(&format!("{}/", prefix)).unwrap_or(path)
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
            format!("{}/{}", prefix, name)
        };
        let sub_sha = Box::pin(build_tree(git_dir, sub_entries, &sub_prefix)).await;
        mktree_input.push_str(&format!("040000 tree {}\t{}\n", sub_sha, name));
    }
    for (name, sha1) in &blobs {
        mktree_input.push_str(&format!("100644 blob {}\t{}\n", sha1, name));
    }

    let mut child = tokio::process::Command::new("git")
        .args(["--git-dir", git_dir.to_str().unwrap()])
        .args(["mktree"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(mktree_input.as_bytes())
        .await
        .unwrap();
    let output = child.wait_with_output().await.unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

impl OdbReader for LocalGitOdb {
    async fn get(&self, key: &str) -> Object {
        self.git()
            .arg("show")
            .arg(format!("{}:{}", self.commit, key))
            .output()
            .await
            .unwrap()
            .stdout
    }

    async fn glob(&self, pattern: &str) -> Vec<String> {
        let output = self
            .git()
            .args(["ls-tree", "-r", "--name-only"])
            .arg(&self.commit)
            .output()
            .await
            .unwrap();
        let pat = glob::Pattern::new(pattern).unwrap();
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| pat.matches(line))
            .map(|s| s.to_string())
            .collect()
    }
}

impl OdbWriter for LocalGitOdb {
    async fn put(&mut self, key: &str, value: &Object) {
        use tokio::io::AsyncWriteExt;
        let mut child = self
            .git()
            .args(["hash-object", "-w", "--stdin"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(value)
            .await
            .unwrap();
        let output = child.wait_with_output().await.unwrap();
        let sha1 = String::from_utf8(output.stdout).unwrap().trim().to_string();
        self.pending.insert(key.to_string(), sha1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    // ── LocalFsOdb ──────────────────────────────────────────────────────────

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

    // ── LocalGitOdb ─────────────────────────────────────────────────────────

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

    #[tokio::test]
    async fn git_put_commit_get_roundtrip() {
        let repo = init_bare_repo();
        let mut odb = LocalGitOdb::new(repo.path().to_path_buf(), String::new());

        let data = b"hello git odb".to_vec();
        odb.put("src/hello.txt", &data).await;
        let commit_sha = odb.commit(&[], "initial").await;
        assert_eq!(commit_sha.len(), 40);

        let odb = LocalGitOdb::new(repo.path().to_path_buf(), commit_sha);
        let got = odb.get("src/hello.txt").await;
        assert_eq!(got, data);
    }

    #[tokio::test]
    async fn git_glob_after_commit() {
        let repo = init_bare_repo();
        let mut odb = LocalGitOdb::new(repo.path().to_path_buf(), String::new());

        odb.put("a/x.rs", &b"fn x(){}".to_vec()).await;
        odb.put("a/y.rs", &b"fn y(){}".to_vec()).await;
        odb.put("b/z.md", &b"# Z".to_vec()).await;
        let commit_sha = odb.commit(&[], "add files").await;

        let odb = LocalGitOdb::new(repo.path().to_path_buf(), commit_sha);
        let mut matches = odb.glob("a/*.rs").await;
        matches.sort();
        assert_eq!(matches, vec!["a/x.rs", "a/y.rs"]);
    }

    #[tokio::test]
    async fn git_commit_with_parent() {
        let repo = init_bare_repo();
        let mut odb = LocalGitOdb::new(repo.path().to_path_buf(), String::new());

        odb.put("a.txt", &b"v1".to_vec()).await;
        let first = odb.commit(&[], "first").await;

        // Second commit only puts b.txt — a.txt is NOT inherited
        let mut odb = LocalGitOdb::new(repo.path().to_path_buf(), first.clone());
        odb.put("b.txt", &b"v2".to_vec()).await;
        let second = odb.commit(&[&first], "second").await;

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
                .args(["rev-parse", &format!("{}^1", second)])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        assert_eq!(parent.trim(), first);
    }
}
