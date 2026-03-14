// Rust port of git's diff-delta.c
// Original: Nicolas Pitre <nico@fluxnic.net>, (C) 2005-2007, GPLv2
//
// SPDX-License-Identifier: GPL-2.0

use super::r#const::{T, U};

const HASH_LIMIT: usize = 64;
const RABIN_SHIFT: u32 = 23;
const RABIN_WINDOW: usize = 16;

// 5 (src size varint) + 5 (trg size varint) + 1 (inscnt) + RABIN_WINDOW + 7 (max copy op)
const MAX_OP_SIZE: usize = 5 + 5 + 1 + RABIN_WINDOW + 7;

/// Opaque index built from a source buffer, mirroring `struct delta_index`.
pub struct DeltaIndex {
    src: Vec<u8>,
    hash_mask: usize,
    /// packed_hash[i] is the start index into `entries` for bucket i;
    /// packed_hash[hsize] is a sentinel end marker.
    packed_hash: Vec<usize>,
    /// (offset_into_src, rabin_val) per entry.
    entries: Vec<(usize, u32)>,
}

impl DeltaIndex {
    /// Build a Rabin-hash index over `src`.  Returns `None` if `src` is empty.
    pub fn new(src: Vec<u8>) -> Option<Self> {
        let bufsize = src.len();
        if bufsize == 0 {
            return None;
        }

        // Skip the first byte (matches C's comment about Rabin init optimisation).
        let mut num_entries = (bufsize - 1) / RABIN_WINDOW;
        if bufsize >= 0xffff_ffff {
            num_entries = 0xffff_fffe / RABIN_WINDOW;
        }

        // Hash table size: smallest power-of-two >= num_entries/4, minimum 16.
        let hsize = {
            let target = (num_entries / 4).max(1);
            let mut s = 16usize;
            while s < target {
                s <<= 1;
            }
            s
        };
        let hmask = hsize - 1;

        // Linked-list hash buckets represented as parallel flat Vecs.
        let mut ev: Vec<u32> = Vec::with_capacity(num_entries); // rabin val
        let mut ep: Vec<usize> = Vec::with_capacity(num_entries); // offset into src
        let mut en: Vec<usize> = Vec::with_capacity(num_entries); // next in bucket (usize::MAX = nil)
        let mut bucket_head: Vec<usize> = vec![usize::MAX; hsize];
        let mut hash_count: Vec<usize> = vec![0; hsize];

        let mut prev_val: u32 = !0;
        let mut actual_entries = 0usize;

        // Iterate blocks from high address to low, matching the C loop direction.
        let mut block = num_entries; // 1-based; block `k` covers src[(k-1)*RW .. k*RW]
        while block > 0 {
            block -= 1;
            let pos = block * RABIN_WINDOW; // start of block in src

            // Hash window: src[pos+1 .. pos+RABIN_WINDOW] (16 bytes, skipping pos+0).
            let mut val: u32 = 0;
            for i in 1..=RABIN_WINDOW {
                val = (val << 8 | src[pos + i] as u32) ^ T[(val >> RABIN_SHIFT) as usize];
            }

            if val == prev_val {
                // Keep only the lowest (earliest) of consecutive identical blocks.
                *ep.last_mut().unwrap() = pos + RABIN_WINDOW;
            } else {
                prev_val = val;
                let b = (val as usize) & hmask;
                let idx = ev.len();
                ev.push(val);
                ep.push(pos + RABIN_WINDOW);
                en.push(bucket_head[b]);
                bucket_head[b] = idx;
                hash_count[b] += 1;
                actual_entries += 1;
            }
        }
        let mut num_entries = actual_entries;

        // Cull overfull buckets to guard against O(m*n) worst-case.
        for i in 0..hsize {
            if hash_count[i] <= HASH_LIMIT {
                continue;
            }
            num_entries -= hash_count[i] - HASH_LIMIT;

            let mut cur = bucket_head[i];
            let mut acc: isize = 0;
            loop {
                if cur == usize::MAX {
                    break;
                }
                acc += (hash_count[i] - HASH_LIMIT) as isize;
                if acc > 0 {
                    let keep = cur;
                    while acc > 0 {
                        cur = en[cur];
                        acc -= HASH_LIMIT as isize;
                    }
                    en[keep] = en[cur];
                }
                cur = en[cur];
            }
        }

        // Pack linked lists into flat arrays, one contiguous slice per bucket.
        let mut packed_hash: Vec<usize> = vec![0; hsize + 1];
        let mut entries: Vec<(usize, u32)> = Vec::with_capacity(num_entries);

        for i in 0..hsize {
            packed_hash[i] = entries.len();
            let mut cur = bucket_head[i];
            while cur != usize::MAX {
                entries.push((ep[cur], ev[cur]));
                cur = en[cur];
            }
        }
        packed_hash[hsize] = entries.len();

        Some(DeltaIndex {
            src,
            hash_mask: hmask,
            packed_hash,
            entries,
        })
    }

    /// Approximate memory footprint of the index.
    #[allow(dead_code)] // TODO: remove allow
    pub fn mem_size(&self) -> usize {
        self.src.len()
            + self.packed_hash.len() * std::mem::size_of::<usize>()
            + self.entries.len() * std::mem::size_of::<(usize, u32)>()
    }
}

fn encode_size(out: &mut Vec<u8>, mut l: usize) {
    while l >= 0x80 {
        out.push((l | 0x80) as u8);
        l >>= 7;
    }
    out.push(l as u8);
}

/// Create a delta from `index` (source) to `trg`.
///
/// If `max_size > 0` and the result exceeds it, returns `None`.
/// This mirrors `create_delta()` in diff-delta.c.
pub fn create_delta(index: &DeltaIndex, trg: &[u8], max_size: usize) -> Option<Vec<u8>> {
    let trg_size = trg.len();
    if trg_size == 0 {
        return None;
    }

    let mut out: Vec<u8> = Vec::with_capacity(8192);
    encode_size(&mut out, index.src.len());
    encode_size(&mut out, trg_size);

    let ref_data = index.src.as_slice();
    let ref_top = ref_data.len();

    // Reserve slot for first inscnt; fill initial Rabin window.
    out.push(0); // inscnt placeholder
    let mut val: u32 = 0;
    let mut di = 0usize;
    while di < RABIN_WINDOW && di < trg_size {
        out.push(trg[di]);
        val = (val << 8 | trg[di] as u32) ^ T[(val >> RABIN_SHIFT) as usize];
        di += 1;
    }
    let mut inscnt: isize = di as isize;

    let mut moff: usize = 0;
    let mut msize: usize = 0;

    'outer: while di < trg_size {
        // Search for a match only when the previous one is exhausted / small.
        if msize < 4096 {
            val ^= U[trg[di - RABIN_WINDOW] as usize];
            val = (val << 8 | trg[di] as u32) ^ T[(val >> RABIN_SHIFT) as usize];

            let b = (val as usize) & index.hash_mask;
            let start = index.packed_hash[b];
            let end = index.packed_hash[b + 1];

            for &(eptr, eval) in &index.entries[start..end] {
                if eval != val {
                    continue;
                }
                if eptr > ref_top {
                    continue;
                }

                let ref_avail = (ref_top - eptr).min(trg_size - di);
                if ref_avail <= msize {
                    break;
                }

                let matched = ref_data[eptr..]
                    .iter()
                    .zip(trg[di..].iter())
                    .take(ref_avail)
                    .take_while(|(a, b)| a == b)
                    .count();

                if matched > msize {
                    msize = matched;
                    moff = eptr;
                    if msize >= 4096 {
                        break;
                    }
                }
            }
        }

        if msize < 4 {
            // Emit an INSERT byte.
            if inscnt == 0 {
                out.push(0); // new inscnt placeholder
            }
            out.push(trg[di]);
            di += 1;
            inscnt += 1;
            if inscnt == 0x7f {
                let len = out.len();
                out[len - inscnt as usize - 1] = inscnt as u8;
                inscnt = 0;
            }
            msize = 0;
        } else {
            // Try to extend the match backward into the preceding insert run.
            if inscnt > 0 {
                while moff > 0 && ref_data[moff - 1] == trg[di - 1] {
                    msize += 1;
                    moff -= 1;
                    di -= 1;
                    out.pop(); // undo the last inserted byte
                    inscnt -= 1;
                    if inscnt == 0 {
                        out.pop(); // also remove the now-empty count slot
                        inscnt = -1;
                        break;
                    }
                }
                if inscnt >= 0 {
                    let len = out.len();
                    out[len - inscnt as usize - 1] = inscnt as u8;
                }
                inscnt = 0;
            }

            // Copy op is limited to 64 KB (pack v2 format).
            let left = if msize < 0x10000 { 0 } else { msize - 0x10000 };
            msize -= left;

            let op_pos = out.len();
            out.push(0); // op byte placeholder
            let mut op: u8 = 0x80;

            if moff & 0x0000_00ff != 0 {
                out.push((moff >> 0) as u8);
                op |= 0x01;
            }
            if moff & 0x0000_ff00 != 0 {
                out.push((moff >> 8) as u8);
                op |= 0x02;
            }
            if moff & 0x00ff_0000 != 0 {
                out.push((moff >> 16) as u8);
                op |= 0x04;
            }
            if moff & 0xff00_0000 != 0 {
                out.push((moff >> 24) as u8);
                op |= 0x08;
            }
            if msize & 0x00ff != 0 {
                out.push((msize >> 0) as u8);
                op |= 0x10;
            }
            if msize & 0xff00 != 0 {
                out.push((msize >> 8) as u8);
                op |= 0x20;
            }
            out[op_pos] = op;

            di += msize;
            moff += msize;
            msize = left;

            if moff > 0xffff_ffff {
                msize = 0;
            }

            // Rebuild Rabin val over the new trailing RABIN_WINDOW bytes.
            if msize < 4096 {
                val = 0;
                for k in 0..RABIN_WINDOW {
                    let byte = trg[di - RABIN_WINDOW + k];
                    val = (val << 8 | byte as u32) ^ T[(val >> RABIN_SHIFT) as usize];
                }
            }
        }

        if max_size > 0 && out.len() > max_size + MAX_OP_SIZE {
            break 'outer;
        }
    }

    if inscnt > 0 {
        let len = out.len();
        out[len - inscnt as usize - 1] = inscnt as u8;
    }

    if max_size > 0 && out.len() > max_size {
        return None;
    }

    Some(out)
}

/// One-shot helper: build index from `src` then compute delta to `trg`.
#[allow(dead_code)]
pub fn diff_delta(src: Vec<u8>, trg: &[u8], max_size: usize) -> Option<Vec<u8>> {
    let index = DeltaIndex::new(src)?;
    create_delta(&index, trg, max_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &[u8] = b"the quick brown fox jumps over the slow lazy dog";
    const TARGET: &[u8] = b"a swift auburn fox jumps over three dormant hounds";

    // ── delta applicator (for test verification) ─────────────────────────────

    fn decode_size(data: &[u8], pos: &mut usize) -> usize {
        let mut size = 0usize;
        let mut shift = 0u32;
        loop {
            let b = data[*pos];
            *pos += 1;
            size |= ((b & 0x7f) as usize) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                break;
            }
        }
        size
    }

    fn apply_delta(src: &[u8], delta: &[u8]) -> Vec<u8> {
        let mut pos = 0;
        let src_size = decode_size(delta, &mut pos);
        let dst_size = decode_size(delta, &mut pos);
        assert_eq!(src_size, src.len(), "delta src size header mismatch");

        let mut out = Vec::with_capacity(dst_size);
        while pos < delta.len() {
            let cmd = delta[pos];
            pos += 1;
            if cmd & 0x80 != 0 {
                let mut offset = 0usize;
                let mut size = 0usize;
                if cmd & 0x01 != 0 {
                    offset |= (delta[pos] as usize) << 0;
                    pos += 1;
                }
                if cmd & 0x02 != 0 {
                    offset |= (delta[pos] as usize) << 8;
                    pos += 1;
                }
                if cmd & 0x04 != 0 {
                    offset |= (delta[pos] as usize) << 16;
                    pos += 1;
                }
                if cmd & 0x08 != 0 {
                    offset |= (delta[pos] as usize) << 24;
                    pos += 1;
                }
                if cmd & 0x10 != 0 {
                    size |= (delta[pos] as usize) << 0;
                    pos += 1;
                }
                if cmd & 0x20 != 0 {
                    size |= (delta[pos] as usize) << 8;
                    pos += 1;
                }
                if size == 0 {
                    size = 0x10000;
                }
                out.extend_from_slice(&src[offset..offset + size]);
            } else if cmd != 0 {
                let n = cmd as usize;
                out.extend_from_slice(&delta[pos..pos + n]);
                pos += n;
            }
        }
        assert_eq!(out.len(), dst_size, "reconstructed size mismatch");
        out
    }

    // ── correctness tests ─────────────────────────────────────────────────────

    #[test]
    fn test_roundtrip_basic() {
        let delta = diff_delta(SOURCE.to_vec(), TARGET, 0).expect("diff_delta None");
        assert_eq!(apply_delta(SOURCE, &delta), TARGET);
    }

    #[test]
    fn test_identical_buffers() {
        // Repeated data is highly compressible; the delta should be tiny.
        let data = b"hello world hello world hello world hello world!!".as_ref();
        let delta = diff_delta(data.to_vec(), data, 0).expect("diff_delta None");
        assert_eq!(apply_delta(data, &delta).as_slice(), data);
        assert!(
            delta.len() < data.len(),
            "delta should be smaller than identical input"
        );
    }

    #[test]
    fn test_small_edit() {
        let src = b"abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnop";
        let trg = b"abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOP";
        let delta = diff_delta(src.to_vec(), trg, 0).expect("diff_delta None");
        assert_eq!(apply_delta(src, &delta).as_slice(), trg.as_slice());
    }

    #[test]
    fn test_max_size_limit() {
        let result = diff_delta(SOURCE.to_vec(), TARGET, 4);
        assert!(result.is_none(), "expected None when max_size too small");
    }

    #[test]
    fn test_empty_target_returns_none() {
        assert!(diff_delta(SOURCE.to_vec(), b"", 0).is_none());
    }

    #[test]
    fn test_multiblock_copy() {
        let base = b"the quick brown fox jumps over the lazy dog, ".repeat(200);
        let mut trg = base.clone();
        let mid = base.len() / 2;
        trg[mid..mid + 10].copy_from_slice(b"0123456789");
        let delta = diff_delta(base.to_vec(), &trg, 0).expect("diff_delta None");
        assert_eq!(apply_delta(&base, &delta), trg);
        assert!(
            delta.len() < trg.len() / 4,
            "delta ({}) should be << target ({})",
            delta.len(),
            trg.len()
        );
    }

    // ── space-efficiency benchmark ────────────────────────────────────────────

    /// Run with: `cargo test test_git_delta_space_efficiency -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn test_git_delta_space_efficiency() {
        struct Lcg(u64);
        impl Lcg {
            fn next(&mut self) -> u64 {
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
        const WORDS: &[u8] = b"the quick brown fox jumps over the lazy dog\n";

        let mut rng = Lcg(SEED);
        let mut base: Vec<u8> = Vec::with_capacity(BASE_SIZE + 64);
        while base.len() < BASE_SIZE {
            let start = rng.next_usize(WORDS.len() - 4);
            let max_len = (WORDS.len() - start).min(20);
            let len = (4 + rng.next_usize(max_len.max(1))).min(WORDS.len() - start);
            base.extend_from_slice(&WORDS[start..start + len]);
        }
        base.truncate(BASE_SIZE);

        let mut versions: Vec<Vec<u8>> = vec![base];
        for _ in 1..NUM_VERSIONS {
            let mut doc = versions.last().unwrap().clone();
            for _ in 0..EDITS_PER_VERSION {
                if doc.is_empty() {
                    break;
                }
                match rng.next_usize(3) {
                    0 => {
                        let pos = rng.next_usize(doc.len() + 1);
                        let len = 4 + rng.next_usize(32);
                        let snippet: Vec<u8> = (0..len)
                            .map(|_| WORDS[rng.next_usize(WORDS.len())])
                            .collect();
                        doc.splice(pos..pos, snippet);
                    }
                    1 => {
                        let pos = rng.next_usize(doc.len());
                        let len = (1 + rng.next_usize(32)).min(doc.len() - pos);
                        doc.drain(pos..pos + len);
                    }
                    _ => {
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

        println!("\n=== git_delta space-efficiency test (seed={SEED}) ===");
        println!(
            "{:<10} {:>10} {:>10} {:>12} {:>8}  {:>12}",
            "pair", "src_bytes", "dst_bytes", "delta_bytes", "ratio%", "index_mem"
        );
        println!("{}", "-".repeat(68));

        for i in 0..versions.len() - 1 {
            let src = versions[i].clone();
            let dst = &versions[i + 1];
            let index = DeltaIndex::new(src.clone()).unwrap();
            let mem = index.mem_size();
            let delta = create_delta(&index, dst, 0).expect("create_delta None");

            // Verify correctness inline.
            let got = apply_delta(&src, &delta);
            assert_eq!(&got, dst, "roundtrip failed at pair {}→{}", i, i + 1);

            let ratio = delta.len() as f64 / dst.len() as f64 * 100.0;
            println!(
                "{:<10} {:>10} {:>10} {:>12} {:>7.1}%  {:>12}",
                format!("{}→{}", i, i + 1),
                src.len(),
                dst.len(),
                delta.len(),
                ratio,
                mem,
            );
        }
    }
}
