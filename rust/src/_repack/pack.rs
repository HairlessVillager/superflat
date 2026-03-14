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
use std::{collections::HashMap, fs::File, io::Write};

pub const OFS_DELTA: u8 = 6;
pub const REF_DELTA: u8 = 7;

/// Encode pack object header bytes (type + variable-length size + optional delta base).
///
/// Mirrors dulwich's `pack_object_header()`.
///
/// Returns `Err` if the type requires a delta argument that was not supplied.
pub fn encode_pack_object_header(
    type_num: u8,
    size: usize,
    ofs_delta: Option<u64>,
    ref_delta: Option<&[u8]>,
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
pub fn compress_zlib(data: &[u8], level: i32) -> std::io::Result<Vec<u8>> {
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
pub struct HashingWriter {
    file: File,
    hasher: Sha1,
    pub offset: u64,
}

impl HashingWriter {
    pub fn create(path: &str) -> std::io::Result<Self> {
        Ok(Self {
            file: File::create(path)?,
            hasher: Sha1::new(),
            offset: 0,
        })
    }

    /// Write the final SHA-1 digest to the file and return it.
    /// The digest itself is not included in the hash.
    pub fn finish(mut self) -> std::io::Result<[u8; 20]> {
        let digest: [u8; 20] = self.hasher.finalize().into();
        self.file.write_all(&digest)?;
        Ok(digest)
    }
}

impl Write for HashingWriter {
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
pub struct PackEntry {
    pub sha: Vec<u8>,
    pub offset: u64,
    pub crc32: u32,
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
pub fn write_pack_data_raw(
    path: &str,
    records: impl Iterator<Item = (u8, Vec<u8>, Option<Vec<u8>>, Vec<u8>)>,
    num_records: usize,
    compression_level: i32,
) -> std::io::Result<(Vec<PackEntry>, [u8; 20])> {
    let mut writer = HashingWriter::create(path)?;
    // sha -> (pack_offset, crc32) for resolving delta bases.
    let mut entries_map: HashMap<Vec<u8>, (u64, u32)> = HashMap::with_capacity(num_records);
    let mut entries_list: Vec<PackEntry> = Vec::with_capacity(num_records);

    // Pack header: b"PACK" + version 2 (u32 BE) + object count (u32 BE).
    let mut hdr = [0u8; 12];
    hdr[..4].copy_from_slice(b"PACK");
    hdr[4..8].copy_from_slice(&2u32.to_be_bytes());
    hdr[8..].copy_from_slice(&(num_records as u32).to_be_bytes());
    writer.write_all(&hdr)?;

    let mut count = 0usize;
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

    let pack_checksum = writer.finish()?;
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
pub fn write_pack_index_v2_raw(
    path: &str,
    sorted_entries: &[PackEntry],
    pack_checksum: &[u8],
) -> std::io::Result<[u8; 20]> {
    debug_assert!(
        sorted_entries.windows(2).all(|w| w[0].sha <= w[1].sha),
        "sorted_entries must be sorted by SHA ascending"
    );

    let mut writer = HashingWriter::create(path)?;

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
    writer.finish()
}

/// Write both a pack file (`filename`.pack) and an index file (`filename`.idx).
///
/// Each record is `(pack_type_num, sha, delta_base_sha_or_none, raw_data)`.
///
/// Returns `(pack_checksum, index_checksum)`.
pub fn write_pack(
    filename: &str,
    records: impl Iterator<Item = (u8, Vec<u8>, Option<Vec<u8>>, Vec<u8>)>,
    num_records: usize,
    compression_level: i32,
) -> std::io::Result<([u8; 20], [u8; 20])> {
    let pack_path = format!("{filename}.pack");
    let idx_path = format!("{filename}.idx");

    let (mut entries_list, pack_checksum) =
        write_pack_data_raw(&pack_path, records, num_records, compression_level)?;

    entries_list.sort_by(|a, b| a.sha.cmp(&b.sha));

    let idx_checksum = write_pack_index_v2_raw(&idx_path, &entries_list, &pack_checksum)?;

    Ok((pack_checksum, idx_checksum))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::ZlibDecoder;
    use std::io::{Read, Write};
    use std::path::Path;
    use std::process::{Command, Stdio};

    const SOURCE: &[u8] = b"the quick brown fox jumps over the slow lazy dog";
    const TARGET: &[u8] = b"a swift auburn fox jumps over three dormant hounds";

    // ── Git helpers ──────────────────────────────────────────────────────────

    /// `git init` a new repo in `dir`.
    fn git_init(dir: &Path) {
        let status = Command::new("git")
            .args(["init", "-q", dir.to_str().unwrap()])
            .status()
            .expect("git init");
        assert!(status.success(), "git init failed");
    }

    /// Write `content` as a loose blob via `git hash-object -w --stdin`.
    /// Returns the 40-char hex SHA-1.
    fn git_hash_object(repo: &Path, content: &[u8]) -> String {
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
    fn read_loose_object(repo: &Path, sha_hex: &str) -> Vec<u8> {
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
    fn parse_object(data: &[u8]) -> (&[u8], &[u8]) {
        let nul = data.iter().position(|&b| b == 0).expect("NUL separator");
        (&data[..nul], &data[nul + 1..])
    }

    /// SHA-1 of a git object: `sha1(<header>\0<content>)`.
    fn object_sha1(header: &[u8], content: &[u8]) -> [u8; 20] {
        let mut h = Sha1::new();
        h.update(header);
        h.update(b"\0");
        h.update(content);
        h.finalize().into()
    }

    fn hex_encode(b: &[u8]) -> String {
        b.iter().map(|b| format!("{b:02x}")).collect()
    }

    // ── Delta encoder ────────────────────────────────────────────────────────

    /// Variable-length size encoding used in git delta headers.
    fn delta_varint(mut size: usize) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let byte = (size & 0x7F) as u8;
            size >>= 7;
            if size > 0 {
                out.push(byte | 0x80);
            } else {
                out.push(byte);
                break;
            }
        }
        out
    }

    /// Encode a delta COPY instruction (offset + size into src).
    fn delta_copy(offset: usize, size: usize) -> Vec<u8> {
        let mut instr = vec![0x80u8];
        for i in 0..4usize {
            let byte = ((offset >> (i * 8)) & 0xFF) as u8;
            if byte != 0 {
                instr.push(byte);
                instr[0] |= 1 << i;
            }
        }
        // size 0 encodes as 0x10000; we never emit that here (max copy 0xFFFF).
        for i in 0..2usize {
            let byte = ((size >> (i * 8)) & 0xFF) as u8;
            if byte != 0 {
                instr.push(byte);
                instr[0] |= 1 << (4 + i);
            }
        }
        instr
    }

    /// Simple block-hash delta encoder (16-byte blocks, first-match wins).
    /// Mirrors the logic of dulwich's `_create_delta_py` but in Rust.
    fn create_delta(src: &[u8], dst: &[u8]) -> Vec<u8> {
        const BLOCK: usize = 16;
        const MAX_COPY: usize = 0xFFFF;

        // Index every aligned 16-byte block in src (first occurrence wins).
        let mut block_map: HashMap<&[u8], usize> = HashMap::new();
        let mut p = 0;
        while p + BLOCK <= src.len() {
            block_map.entry(&src[p..p + BLOCK]).or_insert(p);
            p += BLOCK;
        }

        let mut delta = Vec::new();
        delta.extend(delta_varint(src.len()));
        delta.extend(delta_varint(dst.len()));

        let mut di = 0;
        while di < dst.len() {
            // Try to find a block match in src.
            if di + BLOCK <= dst.len() {
                if let Some(&si) = block_map.get(&dst[di..di + BLOCK]) {
                    // Extend match forward as far as possible.
                    let limit = (src.len() - si).min(dst.len() - di).min(MAX_COPY);
                    let mut copy_len = BLOCK;
                    while copy_len < limit && src[si + copy_len] == dst[di + copy_len] {
                        copy_len += 1;
                    }
                    delta.extend(delta_copy(si, copy_len));
                    di += copy_len;
                    continue;
                }
            }

            // No match — collect bytes into an INSERT instruction (max 127 bytes).
            let mut end = di + 1;
            while end < dst.len() && end - di < 127 {
                // Stop early if the next block would be a good copy candidate.
                if end + BLOCK <= dst.len() && block_map.contains_key(&dst[end..end + BLOCK]) {
                    break;
                }
                end += 1;
            }
            let insert_len = end - di;
            delta.push(insert_len as u8);
            delta.extend_from_slice(&dst[di..di + insert_len]);
            di += insert_len;
        }

        delta
    }

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
        assert_eq!(hex_encode(&sha1), sha_hex_1, "SHA-1 mismatch for SOURCE");
        assert_eq!(hex_encode(&sha2), sha_hex_2, "SHA-1 mismatch for TARGET");

        // Delta-encode TARGET relative to SOURCE.
        let delta = create_delta(content1, content2);

        // Pack: SOURCE as full blob, TARGET as OFS_DELTA relative to SOURCE.
        let pack_path = tmp.join("test.pack");
        let idx_path = tmp.join("test.idx");
        let records: Vec<(u8, Vec<u8>, Option<Vec<u8>>, Vec<u8>)> = vec![
            (3, sha1.to_vec(), None, content1.to_vec()),
            (3, sha2.to_vec(), Some(sha1.to_vec()), delta),
        ];

        let (mut entries, pack_checksum) =
            write_pack_data_raw(pack_path.to_str().unwrap(), records.into_iter(), 2, -1)
                .expect("write_pack_data_raw");
        entries.sort_by(|a, b| a.sha.cmp(&b.sha));
        write_pack_index_v2_raw(idx_path.to_str().unwrap(), &entries, &pack_checksum)
            .expect("write_pack_index_v2_raw");

        // Verify the pack is well-formed.
        let out = Command::new("git")
            .args(["verify-pack", "-v", pack_path.to_str().unwrap()])
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
}
