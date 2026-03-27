use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::path::PathBuf;
use superflat::{flatten, unflatten};

fn bench_unflatten(c: &mut Criterion) {
    let fixture = std::env::var("SF_BENCH_FIXTURE")
        .expect("set SF_BENCH_FIXTURE to a save dir with a single region/r.0.0.mca");
    let version = std::env::var("SF_BENCH_VERSION")
        .expect("set SF_BENCH_VERSION to a Minecraft version (eg. 1.21.11)");
    let flattened = tempfile::tempdir().unwrap();
    flatten(
        PathBuf::from(&fixture),
        flattened.path().to_path_buf(),
        &version,
    );

    c.bench_function("unflatten", |b| {
        b.iter_batched(
            || tempfile::tempdir().unwrap(),
            |output| {
                unflatten(
                    output.path().to_path_buf(),
                    flattened.path().to_path_buf(),
                    &version,
                );
                output
            },
            BatchSize::LargeInput,
        )
    });
}

criterion_group!(benches, bench_unflatten);
criterion_main!(benches);
