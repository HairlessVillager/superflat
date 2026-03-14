// Copyright (c) 2026 HairlessVillager
//
// This file is a Rust implementation derived from `pack.py` in the Dulwich project.
//
// Original Python implementation:
// Copyright (C) 2007 James Westby <jw+debian@jameswestby.net>
// Copyright (C) 2008-2013 Jelmer Vernooij <jelmer@jelmer.uk>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use flate2::{Compression, write::ZlibEncoder};
use sha1::{Digest, Sha1};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

use super::git_helper::ObjID;

const OFS_DELTA: u8 = 6;
const REF_DELTA: u8 = 7;

/// Encode pack object header bytes (type + variable-length size + optional delta base).
///
/// Mirrors dulwich's `pack_object_header()`.
///
/// Returns `Err` if the type requires a delta argument that was not supplied.
fn encode_pack_object_header(
    type_num: u8,
    size: usize,
    ofs_delta: Option<u64>,
    ref_delta: Option<&ObjID>,
) -> Result<Vec<u8>, String> {
    let mut header = Vec::new();

    // First byte: bits 6-4 = type, bits 3-0 = lower 4 bits of size.
    let mut c = ((type_num as usize) << 4) | (size & 0x0F);
    let mut sz = size >> 4;
    while sz > 0 {
        header.push((c | 0x80) as u8);
        c = sz & 0x7F;
        sz >>= 7;
    }
    header.push(c as u8);

    if type_num == OFS_DELTA {
        // Encode delta offset with a variable-length negative-offset format.
        let mut d =
            ofs_delta.ok_or_else(|| "OFS_DELTA requires an ofs_delta offset".to_string())?;
        let mut ret = Vec::new();
        ret.push((d & 0x7F) as u8);
        d >>= 7;
        while d > 0 {
            d -= 1;
            ret.insert(0, 0x80 | ((d & 0x7F) as u8));
            d >>= 7;
        }
        header.extend_from_slice(&ret);
    } else if type_num == REF_DELTA {
        // Append raw SHA bytes of the base object.
        let sha = ref_delta.ok_or_else(|| "REF_DELTA requires a ref_delta SHA".to_string())?;
        header.extend_from_slice(sha);
    }

    Ok(header)
}

/// Compress `data` with zlib at the given level (-1 = default).
fn compress_zlib(data: &[u8], level: i32) -> std::io::Result<Vec<u8>> {
    let compression = if level < 0 {
        Compression::default()
    } else {
        Compression::new(level.clamp(0, 9) as u32)
    };
    let mut encoder = ZlibEncoder::new(Vec::new(), compression);
    encoder.write_all(data)?;
    encoder.finish()
}

/// File writer that computes a running SHA-1 over all written bytes.
struct HashingWriter<W> {
    file: W,
    hasher: Sha1, // TODO: use sha256 if possible
    offset: u64,
}

impl<W: Write> HashingWriter<W> {
    /// Write the final SHA-1 digest to the file and return it.
    /// The digest itself is not included in the hash.
    fn finalize(mut self) -> std::io::Result<(W, ObjID)> {
        let digest: ObjID = self.hasher.finalize().into(); // TODO: use sha256 if possible
        self.file.write_all(&digest)?;
        Ok((self.file, digest))
    }
}

impl<W: Write> From<W> for HashingWriter<W> {
    fn from(value: W) -> Self {
        Self {
            file: value,
            hasher: Sha1::new(),
            offset: 0,
        }
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.file.write(buf)?;
        self.hasher.update(&buf[..n]);
        self.offset += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

/// One entry in the resulting pack (sha, pack offset, crc32).
struct PackEntry {
    sha: ObjID,
    offset: u64,
    crc32: u32,
}

/// Write a pack data file at `path`.
///
/// Each record is `(pack_type_num, sha, delta_base_sha_or_none, raw_data)`.
/// - For non-delta objects `delta_base` is `None` and `raw_data` is the raw object bytes.
/// - For delta objects `delta_base` is the SHA of the base object and `raw_data` is
///   the already-encoded delta instructions.
///
/// The `size` field in the pack object header encodes:
/// - For non-delta objects: the length of the raw (uncompressed) object content.
/// - For delta objects (OFS_DELTA / REF_DELTA): the length of the delta instructions.
/// This matches dulwich's `PackChunkGenerator._pack_data_chunks` behaviour.
///
/// When the base object has already been written we emit an OFS_DELTA; otherwise REF_DELTA.
///
/// Returns `(pack_entries, pack_checksum)`.
fn write_pack_data_raw(
    packfile: impl Write,
    records: impl Iterator<Item = (u8, ObjID, Option<ObjID>, Vec<u8>)>,
    num_records: usize,
    compression_level: i32,
) -> std::io::Result<(Vec<PackEntry>, ObjID)> {
    let mut writer = HashingWriter::from(packfile);
    // sha -> (pack_offset, crc32) for resolving delta bases.
    let mut entries_map: HashMap<ObjID, (u64, u32)> = HashMap::with_capacity(num_records);
    let mut entries_list: Vec<PackEntry> = Vec::with_capacity(num_records);

    // Pack header: b"PACK" + version 2 (u32 BE) + object count (u32 BE).
    let mut hdr = [0u8; 12];
    hdr[..4].copy_from_slice(b"PACK");
    hdr[4..8].copy_from_slice(&2u32.to_be_bytes());
    hdr[8..].copy_from_slice(&(num_records as u32).to_be_bytes());
    writer.write_all(&hdr)?;

    let mut count: usize = 0;
    for (pack_type_num, sha, delta_base, raw_data) in records {
        let obj_offset = writer.offset;

        let obj_header = match &delta_base {
            Some(base_sha) => {
                if let Some(&(base_offset, _)) = entries_map.get(base_sha) {
                    // Base already written — use relative OFS_DELTA.
                    let delta = obj_offset - base_offset;
                    encode_pack_object_header(OFS_DELTA, raw_data.len(), Some(delta), None)
                } else {
                    // Base not in this pack yet — use absolute REF_DELTA.
                    encode_pack_object_header(REF_DELTA, raw_data.len(), None, Some(base_sha))
                }
            }
            None => encode_pack_object_header(pack_type_num, raw_data.len(), None, None),
        }
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        let compressed = compress_zlib(&raw_data, compression_level)?;

        let mut crc_hasher = crc32fast::Hasher::new();
        crc_hasher.update(&obj_header);
        crc_hasher.update(&compressed);
        let crc32 = crc_hasher.finalize();

        writer.write_all(&obj_header)?;
        writer.write_all(&compressed)?;

        entries_map.insert(sha.clone(), (obj_offset, crc32));
        entries_list.push(PackEntry {
            sha,
            offset: obj_offset,
            crc32,
        });
        count += 1;
    }

    if count != num_records {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("actual records written differs: {count} != {num_records}"),
        ));
    }

    let (_, pack_checksum) = writer.finalize()?;
    Ok((entries_list, pack_checksum))
}

/// Write a pack index v2 file at `path`.
///
/// `sorted_entries` must be sorted by SHA ascending. Returns the index checksum.
///
/// Format (mirrors dulwich's `write_pack_index_v2`):
///   magic(4) + version(4) + fan_out(1024) + shas(20*N) +
///   crc32s(4*N) + offsets(4*N) + [large_offsets(8*M)] +
///   pack_checksum(20) + index_checksum(20)
fn write_pack_index_v2_raw(
    idxfile: impl Write,
    sorted_entries: &[PackEntry],
    pack_checksum: &[u8],
) -> std::io::Result<ObjID> {
    debug_assert!(
        sorted_entries.windows(2).all(|w| w[0].sha <= w[1].sha),
        "sorted_entries must be sorted by SHA ascending"
    );

    let mut writer = HashingWriter::from(idxfile);

    // Magic 0xFF744F63 and version 2.
    writer.write_all(b"\xfftOc")?;
    writer.write_all(&2u32.to_be_bytes())?;

    // Fan-out table: fan_out[i] = cumulative count of entries with sha[0] <= i.
    let mut fan_out = [0u32; 256];
    for e in sorted_entries {
        fan_out[e.sha[0] as usize] += 1;
    }
    for i in 1..256usize {
        fan_out[i] += fan_out[i - 1];
    }
    for v in fan_out {
        writer.write_all(&v.to_be_bytes())?;
    }

    // SHA entries.
    for e in sorted_entries {
        writer.write_all(&e.sha)?;
    }

    // CRC32 entries.
    for e in sorted_entries {
        writer.write_all(&e.crc32.to_be_bytes())?;
    }

    // Offset entries — large offsets (>= 2^31) go into a separate table.
    let mut large_offsets: Vec<u64> = Vec::new();
    for e in sorted_entries {
        if e.offset < (1u64 << 31) {
            writer.write_all(&(e.offset as u32).to_be_bytes())?;
        } else {
            let idx = (large_offsets.len() as u32) | (1u32 << 31);
            writer.write_all(&idx.to_be_bytes())?;
            large_offsets.push(e.offset);
        }
    }

    // Large offset table.
    for lo in &large_offsets {
        writer.write_all(&lo.to_be_bytes())?;
    }

    // Pack checksum (included in the index hash).
    writer.write_all(pack_checksum)?;

    // Index checksum (SHA-1 of all preceding index bytes).
    let (_, checksum) = writer.finalize()?;
    Ok(checksum)
}

/// Write both a pack file (`filename`.pack) and an index file (`filename`.idx).
///
/// Each record is `(pack_type_num, sha, delta_base_sha_or_none, raw_data)`.
///
/// You should provide delta instructions as raw_data when delta_base_sha is not None
///
/// Returns `(pack_checksum, index_checksum)`.
#[allow(dead_code)] // TODO: remove allow
pub fn write_pack(
    pack_dir: &PathBuf,
    basename: &str,
    records: impl Iterator<Item = (u8, ObjID, Option<ObjID>, Vec<u8>)>,
    num_records: usize,
    compression_level: i32,
) -> std::io::Result<(ObjID, ObjID)> {
    let mut pack_buf: Vec<u8> = Vec::new();
    let mut idx_buf: Vec<u8> = Vec::new();

    let (mut entries_list, pack_checksum) =
        write_pack_data_raw(&mut pack_buf, records, num_records, compression_level)?;

    entries_list.sort_by(|a, b| a.sha.cmp(&b.sha));
    let idx_checksum = write_pack_index_v2_raw(&mut idx_buf, &entries_list, &pack_checksum)?;

    let write_file = |buf, ext| -> io::Result<()> {
        let mut file_path = pack_dir.clone();
        file_path.push(Path::new(&format!(
            "{}-{}",
            basename,
            hex::encode(&pack_checksum)
        )));
        file_path.set_extension(ext);
        let mut file = File::create_new(file_path)?;
        file.write_all(buf)?;
        Ok(())
    };
    write_file(&pack_buf, "pack")?;
    write_file(&idx_buf, "idx")?;

    Ok((pack_checksum, idx_checksum))
}

#[cfg(test)]
mod tests {
    use super::super::delta;
    use super::super::git_helper::*;
    use super::*;
    use std::process::Command;

    const SOURCE: &[u8] = b"the quick brown fox jumps over the slow lazy dog";
    const TARGET: &[u8] = b"a swift auburn fox jumps over three dormant hounds";

    // ── Test ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_pack_with_delta() {
        // Set up a temporary git repo.
        let tmp =
            std::env::temp_dir().join(format!("superflat-repack-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        git_init(&tmp);

        // Write the two blobs and get their git SHA-1s.
        let sha_hex_1 = git_hash_object(&tmp, SOURCE);
        let sha_hex_2 = git_hash_object(&tmp, TARGET);

        // Read back from the object store, parse, and verify content.
        let raw1 = read_loose_object(&tmp, &sha_hex_1);
        let raw2 = read_loose_object(&tmp, &sha_hex_2);
        let (hdr1, content1) = parse_object(&raw1);
        let (hdr2, content2) = parse_object(&raw2);
        assert_eq!(content1, SOURCE, "round-trip mismatch for SOURCE");
        assert_eq!(content2, TARGET, "round-trip mismatch for TARGET");

        // Verify our SHA-1 computation agrees with git.
        let sha1 = object_sha1(hdr1, content1);
        let sha2 = object_sha1(hdr2, content2);
        assert_eq!(hex::encode(&sha1), sha_hex_1, "SHA-1 mismatch for SOURCE");
        assert_eq!(hex::encode(&sha2), sha_hex_2, "SHA-1 mismatch for TARGET");

        // Delta-encode TARGET relative to SOURCE.
        let d = delta::dulwich::create_delta(content1, content2);

        // Pack: SOURCE as full blob, TARGET as OFS_DELTA relative to SOURCE.
        let pack_path = tmp.join("test.pack");
        let idx_path = tmp.join("test.idx");
        let records: Vec<(u8, ObjID, Option<ObjID>, Vec<u8>)> =
            vec![(3, sha1, None, content1.to_vec()), (3, sha2, Some(sha1), d)];

        let (mut entries, pack_checksum) = write_pack_data_raw(
            File::create_new(&pack_path).unwrap(),
            records.into_iter(),
            2,
            -1,
        )
        .expect("write_pack_data_raw");
        entries.sort_by(|a, b| a.sha.cmp(&b.sha));
        write_pack_index_v2_raw(
            File::create_new(&idx_path).unwrap(),
            &entries,
            &pack_checksum,
        )
        .expect("write_pack_index_v2_raw");

        // Verify the pack is well-formed.
        let out = Command::new("git")
            .args(["verify-pack", "-v", &pack_path.to_str().unwrap()])
            .output()
            .expect("git verify-pack");
        let stdout = String::from_utf8_lossy(&out.stdout);
        if !out.status.success() {
            panic!(
                "git verify-pack failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        assert!(
            stdout.contains(&sha_hex_1),
            "SOURCE sha not in verify output"
        );
        assert!(
            stdout.contains(&sha_hex_2),
            "TARGET sha not in verify output"
        );

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// Space-efficiency benchmark for `create_delta`.
    ///
    /// Simulates a realistic document-editing workflow: a base document is
    /// mutated version-by-version (insertions, deletions, and in-place edits)
    /// according to a deterministic seed. Adjacent versions are delta-encoded
    /// and the resulting sizes are printed.
    ///
    /// Run with: `cargo test test_delta_space_efficiency -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn test_delta_space_efficiency() {
        /// Minimal LCG PRNG — avoids pulling in the `rand` crate.
        struct Lcg(u64);
        impl Lcg {
            fn next(&mut self) -> u64 {
                // Knuth multiplicative hash constants.
                self.0 = self
                    .0
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                self.0
            }
            fn next_usize(&mut self, max: usize) -> usize {
                (self.next() % max as u64) as usize
            }
        }

        const SEED: u64 = 42;
        const NUM_VERSIONS: usize = 20;
        const BASE_SIZE: usize = 4096;
        const EDITS_PER_VERSION: usize = 10;

        // "Word soup" that produces text-like, compressible content.
        const WORDS: &[u8] = b"the quick brown fox jumps over the lazy dog\n";

        let mut rng = Lcg(SEED);

        // Build a base document by repeating random word-like slices.
        let mut base: Vec<u8> = Vec::with_capacity(BASE_SIZE + 64);
        while base.len() < BASE_SIZE {
            let start = rng.next_usize(WORDS.len() - 4);
            let max_len = (WORDS.len() - start).min(20);
            let len = 4 + rng.next_usize(max_len.max(1));
            let len = len.min(WORDS.len() - start);
            base.extend_from_slice(&WORDS[start..start + len]);
        }
        base.truncate(BASE_SIZE);

        // Evolve the document through NUM_VERSIONS generations.
        let mut versions: Vec<Vec<u8>> = vec![base];
        for _ in 1..NUM_VERSIONS {
            let mut doc = versions.last().unwrap().clone();
            for _ in 0..EDITS_PER_VERSION {
                if doc.is_empty() {
                    break;
                }
                match rng.next_usize(3) {
                    0 => {
                        // Insert a short snippet at a random position.
                        let pos = rng.next_usize(doc.len() + 1);
                        let snippet_len = 4 + rng.next_usize(32);
                        let snippet: Vec<u8> = (0..snippet_len)
                            .map(|_| WORDS[rng.next_usize(WORDS.len())])
                            .collect();
                        doc.splice(pos..pos, snippet);
                    }
                    1 => {
                        // Delete a short run.
                        let pos = rng.next_usize(doc.len());
                        let len = (1 + rng.next_usize(32)).min(doc.len() - pos);
                        doc.drain(pos..pos + len);
                    }
                    _ => {
                        // Overwrite a short run with fresh word-soup bytes.
                        let pos = rng.next_usize(doc.len());
                        let len = (4 + rng.next_usize(16)).min(doc.len() - pos);
                        for i in 0..len {
                            doc[pos + i] = WORDS[rng.next_usize(WORDS.len())];
                        }
                    }
                }
            }
            versions.push(doc);
        }

        // Delta-encode every adjacent pair and report sizes.
        println!("\n=== Delta space-efficiency test (seed={SEED}) ===");
        println!(
            "{:<10} {:>10} {:>10} {:>12} {:>8}",
            "pair", "src_bytes", "dst_bytes", "delta_bytes", "ratio%"
        );
        println!("{}", "-".repeat(54));

        for i in 0..versions.len() - 1 {
            let src = &versions[i];
            let dst = &versions[i + 1];
            let d = delta::dulwich::create_delta(src, dst);
            let ratio = d.len() as f64 / dst.len() as f64 * 100.0;
            println!(
                "{:<10} {:>10} {:>10} {:>12} {:>7.1}%",
                format!("{}→{}", i, i + 1),
                src.len(),
                dst.len(),
                d.len(),
                ratio,
            );
        }
    }
}
