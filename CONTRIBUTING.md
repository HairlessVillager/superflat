## Profiling Guide

Use samply to profile, for example, `sf checkout`:

```sh
cargo build --profile profiling
samply record ./target/profiling/sf checkout /tmp/save /tmp/sf.git -c aa0183a927fc4e6bbca578ecc10e3f1b639cdb90
```
