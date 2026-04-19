use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::process::Command;
use superflat::odb::{LocalGitOdb, OdbReader, OdbWriter};

fn init_bare_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    Command::new("git")
        .args([
            "init",
            "--bare",
            dir.path()
                .to_str()
                .expect("temp dir path is not valid utf-8"),
        ])
        .output()
        .expect("failed to run git init");
    Command::new("git")
        .args([
            "--git-dir",
            dir.path()
                .to_str()
                .expect("temp dir path is not valid utf-8"),
        ])
        .args(["config", "user.email", "bench@bench"])
        .output()
        .expect("failed to run git config user.email");
    Command::new("git")
        .args([
            "--git-dir",
            dir.path()
                .to_str()
                .expect("temp dir path is not valid utf-8"),
        ])
        .args(["config", "user.name", "Bench"])
        .output()
        .expect("failed to run git config user.name");
    dir
}

/// Generate a single 200 KB blob that compresses to ~5 KB.
/// A 16 KB block with moderate arithmetic entropy, repeated 12 times,
/// gives a ~35:1 zlib ratio (≈6 KB compressed).
fn make_blob() -> (String, Vec<u8>) {
    let mut block = vec![0u8; 16 * 1024];
    for (i, b) in block.iter_mut().enumerate() {
        *b = ((i as u64 * 13 ^ (i as u64 >> 2) ^ (i as u64 * 7 >> 4)) & 0xFF) as u8;
    }
    let mut data = block.repeat(12); // 16 * 1024 * 12 = 196_608 bytes
    data.extend_from_slice(&data.clone()[..200 * 1024 - data.len()]); // pad to 200 KB
    ("file.bin".to_string(), data)
}

fn bench_writer(c: &mut Criterion) {
    let (key, data) = make_blob();

    c.bench_function("put_200KB_blob", |b| {
        b.iter(|| {
            let repo = init_bare_repo();
            let mut odb = LocalGitOdb::new(repo.path().to_path_buf()).unwrap();
            odb.put(&key, &data).unwrap();
            odb.commit(&[] as &[&str], "bench").unwrap();
        });
    });
}

fn bench_reader(c: &mut Criterion) {
    let (key, data) = make_blob();

    // Commit once, reuse across iterations.
    let repo = init_bare_repo();
    let commit_sha = {
        let mut odb = LocalGitOdb::new(repo.path().to_path_buf()).unwrap();
        odb.put(&key, &data).unwrap();
        odb.commit(&[] as &[&str], "bench-data")
    }
    .unwrap();

    c.bench_function("get_200KB_blob", |b| {
        b.iter(|| {
            let odb =
                LocalGitOdb::from_commit(repo.path().to_path_buf(), commit_sha.clone()).unwrap();
            black_box(odb.get(&key)).unwrap();
        });
    });
}

criterion_group!(benches, bench_writer, bench_reader);
criterion_main!(benches);
