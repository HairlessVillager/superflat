// Copyright (c) 2026 HairlessVillager
//
// This file is derived from `pack.py` in the Dulwich project.
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

use std::collections::HashMap;

/// Variable-length size encoding used in git delta headers.
///
/// See https://git-scm.com/docs/gitformat-pack#_size_encoding
fn encode_size(mut size: usize) -> Vec<u8> {
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
#[allow(dead_code)] // TODO: remove allow
pub fn create_delta(src: &[u8], dst: &[u8]) -> Vec<u8> {
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
    delta.extend(encode_size(src.len()));
    delta.extend(encode_size(dst.len()));

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
