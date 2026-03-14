use flate2::read::ZlibDecoder;
use pumpkin_data::item::Item;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub type ObjID = [u8; 20];

/// `git init` a new repo in `dir`.
#[allow(dead_code)] // TODO: remove allow
pub fn git_init(dir: &Path) {
    let status = Command::new("git")
        .args(["init", "-q", dir.to_str().unwrap()])
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed: {}", status);
}

/// Write `content` as a loose blob via `git hash-object -w --stdin`.
/// Returns the 40-char hex SHA-1.
#[allow(dead_code)] // TODO: remove allow
pub fn git_hash_object(repo: &Path, content: &[u8]) -> String {
    let mut child = Command::new("git")
        .args(["-C", repo.to_str().unwrap(), "hash-object", "-w", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn git hash-object");
    child.stdin.take().unwrap().write_all(content).unwrap();
    let out = child.wait_with_output().expect("git hash-object wait");
    assert!(out.status.success(), "git hash-object failed");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Decompress the loose object at `<repo>/.git/objects/<xx>/<rest>`.
#[allow(dead_code)] // TODO: remove allow
pub fn read_loose_object(repo: &Path, sha_hex: &str) -> Vec<u8> {
    let path = repo
        .join(".git")
        .join("objects")
        .join(&sha_hex[..2])
        .join(&sha_hex[2..]);
    let file = File::open(&path).expect("open loose object");
    let mut decoder = ZlibDecoder::new(file);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).expect("zlib decompress");
    out
}

/// Split `<type> <size>\0<content>` into (header, content).
#[allow(dead_code)] // TODO: remove allow
pub fn parse_object(data: &[u8]) -> (&[u8], &[u8]) {
    let nul = data.iter().position(|&b| b == 0).expect("NUL separator");
    (&data[..nul], &data[nul + 1..])
}

/// SHA-1 of a git object: `sha1(<header>\0<content>)`.
#[allow(dead_code)] // TODO: remove allow
pub fn object_sha1(header: &[u8], content: &[u8]) -> ObjID {
    let mut h = Sha1::new();
    h.update(header);
    h.update(b"\0");
    h.update(content);
    h.finalize().into()
}

pub fn build_object(r#type: &[u8], size: usize, content: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size + 10);
    buf.write(r#type).unwrap();
    buf.write(b" ").unwrap();
    buf.write(size.to_string().as_bytes()).unwrap();
    buf.write(b"\0").unwrap();
    buf.write(content).unwrap();
    buf.flush().unwrap();
    buf
}

// git --git_dir <git-dir> rev-list --parents --topo-order <commit>
// Returns Vec<(child, parents)>
// TODO: use struct to hold ObjIDs and store ref to ObjIDs separately
pub fn get_commit_topo(git_dir: impl AsRef<OsStr>, commit: &ObjID) -> HashMap<ObjID, Vec<ObjID>> {
    let child = Command::new("git")
        .args([
            OsStr::new("--git_dir"),
            (git_dir.as_ref()),
            OsStr::new("rev-list"),
            OsStr::new("--parents"),
            OsStr::new("--topo-order"),
            OsStr::new(hex::encode(commit).as_str()),
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "git rev-list failed: {}", out.status);
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();
    stdout
        .lines()
        .filter_map(|line| {
            if line.is_empty() {
                return None;
            }
            let mut parts = line.split_whitespace();
            if let Some(child) = parts.next() {
                let child: ObjID = hex::decode(child.as_bytes()).unwrap().try_into().unwrap();
                let parents: Vec<ObjID> = parts
                    .map(|parnet| hex::decode(parnet.as_bytes()).unwrap().try_into().unwrap())
                    .collect();
                Some((child, parents))
            } else {
                None
            }
        })
        .collect()
}

// git --git-dir <git-dir> ls-tree -r <commit>
// Returns Vec<(blob_id, path)>
pub fn get_commit_tree(git_dir: impl AsRef<OsStr>, commit: &ObjID) -> HashMap<PathBuf, ObjID> {
    let child = Command::new("git")
        .args([
            OsStr::new("--git_dir"),
            git_dir.as_ref(),
            OsStr::new("ls-tree"),
            OsStr::new("--format=%(objectname) %(path)"),
            OsStr::new("-r"),
            OsStr::new(hex::encode(commit).as_str()),
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "git ls-tree failed: {}", out.status);
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();
    stdout
        .lines()
        .filter_map(|line| {
            if line.is_empty() {
                return None;
            }
            let mut parts = line.split_whitespace();
            let blob_id: ObjID = hex::decode(parts.next().unwrap().as_bytes())
                .unwrap()
                .try_into()
                .unwrap();
            let path = parts.next().unwrap();
            Some((PathBuf::from(path), blob_id))
        })
        .collect()
}

pub struct CatFileEntry {
    name: ObjID,
    deltabase: ObjID,
    size: usize,
    size_disk: usize,
    content: Vec<u8>,
}

// git cat-file "--batch=%(objectname) %(deltabase) %(objectsize) %(objectsize:disk)"
pub fn cat_files<I: Iterator<Item = ObjID>>(
    git_dir: impl AsRef<OsStr>,
    blob_ids: I,
) -> impl Iterator<Item = CatFileEntry> {
    let mut child = Command::new("git")
        .args([
            OsStr::new("--git-dir"),
            git_dir.as_ref(),
            OsStr::new("cat-file"),
            OsStr::new("--batch=%(objectname) %(deltabase) %(objectsize) %(objectsize:disk)"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn git cat-file");

    // 取出 stdin 和 stdout
    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    blob_ids.map(move |blob_id| {
        writeln!(stdin, "{}", hex::encode(blob_id)).expect("Failed to write to stdin");
        stdin.flush().expect("Failed to flush stdin");

        let mut header = String::new();
        reader
            .read_line(&mut header)
            .expect("Failed to read header");

        if header.ends_with("missing\n") {
            panic!("Object {} is missing", hex::encode(blob_id));
        }

        let parts: Vec<&str> = header.split_whitespace().collect();
        let name_str = parts[0];
        let deltabase_str = parts[1];
        let size: usize = parts[2].parse().expect("Invalid size");
        let size_disk: usize = parts[3].parse().expect("Invalid size_disk");

        let mut content = vec![0u8; size];
        reader
            .read_exact(&mut content)
            .expect("Failed to read content");

        {
            let mut newline = [0u8; 1];
            reader
                .read_exact(&mut newline)
                .expect("Failed to read trailing newline");
        }

        CatFileEntry {
            name: hex::decode(name_str).unwrap().try_into().unwrap(),
            deltabase: hex::decode(deltabase_str).unwrap().try_into().unwrap(),
            size,
            size_disk,
            content,
        }
    })
}
