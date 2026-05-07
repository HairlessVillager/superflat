use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{Context, anyhow};
use gix::{
    bstr::ByteSlice,
    hash::{self, ObjectId},
    interrupt,
    odb::pack,
    parallel::InOrderIter,
    prelude::Finalize,
};

pub fn repack(git_dir: impl AsRef<Path>, target: ObjectId, source: ObjectId) -> anyhow::Result<()> {
    let git_dir = git_dir.as_ref();
    let repo = gix::open(git_dir.to_path_buf())?.into_sync();
    let loose_objects = {
        let objects_dir = repo.objects_dir().to_path_buf();
        let object_hash = repo.objects.object_hash();
        let loose = gix::odb::loose::Store::at(&objects_dir, object_hash);
        let oids = loose.iter().collect::<Result<HashSet<_>, _>>()?;
        if oids.is_empty() {
            return Ok(());
        }
        oids
    };

    let analysis_repo = repo.to_thread_local();

    let target_blobs = {
        let target_commit = analysis_repo.find_commit(target)?;
        let target_tree = analysis_repo.find_tree(target_commit.tree_id()?)?;
        let mut tb = HashMap::new();
        walk_tree(&target_tree, "", &loose_objects, &HashMap::new(), &mut tb)?;
        tb
    };

    let source_blobs = {
        let source_commit = analysis_repo.find_commit(source)?;
        let source_tree = analysis_repo.find_tree(source_commit.tree_id()?)?;
        let mut sb = HashMap::new();
        walk_tree(&source_tree, "", &loose_objects, &HashMap::new(), &mut sb)?;
        sb
    };

    let mut skip = HashSet::new();
    let mut topo = HashMap::new();
    for (path, &target_blob) in &target_blobs {
        match source_blobs.get(path) {
            Some(&source_blob) if source_blob == target_blob => {
                skip.insert(target_blob);
            }
            Some(&source_blob) => {
                topo.insert(target_blob, source_blob);
            }
            None => {}
        }
    }

    let counts = loose_objects
        .iter()
        .filter(|oid| !skip.contains(*oid))
        .map(|oid| pack::data::output::Count {
            id: oid.to_owned(),
            entry_pack_location: pack::data::output::count::PackLocation::NotLookedUp,
        })
        .collect::<Vec<_>>();

    if counts.is_empty() {
        return Ok(());
    }

    let num_objects = counts.len();
    let thread_limit = None;
    let chunk_size = 1000;
    drop(analysis_repo);
    let mut handle = repo.objects.into_shared_arc().to_cache_arc();
    handle.prevent_pack_unload();
    handle.ignore_replacements = true;
    let mut in_order_entries = {
        let progress = gix::progress::Discard;
        InOrderIter::from(iter_from_counts::iter_from_counts(
            counts,
            topo,
            handle.clone(),
            Box::new(progress),
            iter_from_counts::Options {
                thread_limit,
                chunk_size,
                version: Default::default(),
            },
        ))
    };

    let pack_dir = git_dir.join("objects").join("pack");
    let mut named_tempfile = tempfile::NamedTempFile::new_in(&pack_dir)?;
    let make_cancellation_err = || anyhow!("Cancelled by user");
    let mut interruptible_output_iter = interrupt::Iter::new(
        pack::data::output::bytes::FromEntriesIter::new(
            in_order_entries.by_ref(),
            &mut named_tempfile,
            num_objects as u32,
            pack::data::Version::default(),
            hash::Kind::default(),
        ),
        make_cancellation_err,
    );
    for io_res in interruptible_output_iter.by_ref() {
        io_res??;
    }

    let hash = interruptible_output_iter
        .into_inner()
        .digest()
        .expect("iteration is done");
    let pack_name = format!("{hash}.pack");
    let pack_path = pack_dir.join(&pack_name);
    named_tempfile.persist(&pack_path)?;
    let _ = in_order_entries.inner.finalize()?;

    std::process::Command::new("git") // TODO: use gitoxide to index pack
        .args(["index-pack", pack_path.to_str().unwrap()])
        .output()
        .context("failed to run git index-pack")?;

    for oid in &loose_objects {
        if skip.contains(oid) {
            continue;
        }
        let hex = oid.to_hex();
        let hex_str = hex.to_string();
        let obj_path = git_dir
            .join("objects")
            .join(&hex_str[..2])
            .join(&hex_str[2..]);
        let _ = std::fs::remove_file(obj_path);
    }

    Ok(())
}

fn walk_tree(
    tree: &gix::Tree<'_>,
    prefix: &str,
    loose: &std::collections::HashSet<ObjectId>,
    trees: &HashMap<ObjectId, HashMap<String, ObjectId>>,
    out: &mut HashMap<String, ObjectId>,
) -> anyhow::Result<()> {
    for entry in tree.iter() {
        let entry = entry?;
        let name = entry.filename().to_str_lossy().to_string();
        let path = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        if entry.mode().is_tree() {
            let sub_oid = entry.oid().to_owned();
            if let Some(sub_map) = trees.get(&sub_oid) {
                for (p, b) in sub_map {
                    out.insert(p.clone(), *b);
                }
            } else {
                let sub_tree = tree.repo.find_tree(sub_oid)?;
                walk_tree(&sub_tree, &path, loose, trees, out)?;
            }
        } else if entry.mode().is_blob() {
            let blob_oid = entry.oid().to_owned();
            if loose.contains(&blob_oid) {
                out.insert(path, blob_oid);
            }
        }
    }
    Ok(())
}

mod iter_from_counts {
    use std::{cmp::Ordering, collections::HashMap, io::Write, sync::Arc};

    use gix::{
        Progress,
        hash::ObjectId,
        odb::pack,
        parallel::{self, SequenceId},
        progress::{self, Count, DynNestedProgress, UNKNOWN},
    };

    use pack::data::output::{
        self,
        entry::iter_from_counts::{Error, Outcome, ProgressId},
    };

    #[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
    pub struct Options {
        pub thread_limit: Option<usize>,
        pub chunk_size: usize,
        pub version: pack::data::Version,
    }

    pub fn iter_from_counts<Find>(
        mut counts: Vec<output::Count>,
        topo: HashMap<ObjectId, ObjectId>,
        db: Find,
        mut progress: Box<dyn DynNestedProgress + 'static>,
        Options {
            thread_limit,
            chunk_size,
            version,
        }: Options,
    ) -> impl Iterator<Item = Result<(SequenceId, Vec<output::Entry>), Error>>
    + parallel::reduce::Finalize<Reduce = reduce::Statistics<Error>>
    where
        Find: pack::Find + Send + Clone + 'static,
    {
        assert!(
            matches!(version, pack::data::Version::V2),
            "currently we can only write version 2"
        );
        let (chunk_size, thread_limit, _) = parallel::optimize_chunk_size_and_thread_limit(
            chunk_size,
            Some(counts.len()),
            thread_limit,
            None,
        );
        {
            let progress = Arc::new(parking_lot::Mutex::new(
                progress.add_child_with_id("resolving".into(), ProgressId::ResolveCounts.into()),
            ));
            progress.lock().init(None, progress::count("counts"));
            let enough_counts_present = counts.len() > 4_000;
            let start = std::time::Instant::now();
            parallel::in_parallel_if(
                || enough_counts_present,
                counts.chunks_mut(chunk_size),
                thread_limit,
                |_n| Vec::<u8>::new(),
                {
                    let progress = Arc::clone(&progress);
                    let db = db.clone();
                    move |chunk, buf| {
                        let chunk_size = chunk.len();
                        for count in chunk {
                            use pack::data::output::count::PackLocation::*;
                            match count.entry_pack_location {
                                LookedUp(_) => continue,
                                NotLookedUp => {
                                    count.entry_pack_location =
                                        LookedUp(db.location_by_oid(&count.id, buf))
                                }
                            }
                        }
                        progress.lock().inc_by(chunk_size);
                        Ok::<_, ()>(())
                    }
                },
                parallel::reduce::IdentityWithResult::<(), ()>::default(),
            )
            .expect("infallible - we ignore none-existing objects");
            progress.lock().show_throughput(start);
        }

        let sorted_counts = {
            topo_sort(counts.as_mut_slice(), &topo).expect("no loop in delta topo");
            Arc::new(counts)
        };
        let progress = Arc::new(parking_lot::Mutex::new(progress));
        let chunks = util::ChunkRanges::new(chunk_size, sorted_counts.len());

        let oid_index_mapping = Arc::new(
            sorted_counts
                .iter()
                .enumerate()
                .map(|(index, count)| (count.id, index))
                .collect::<std::collections::HashMap<_, _>>(),
        );
        parallel::reduce::Stepwise::new(
            chunks.enumerate(),
            thread_limit,
            {
                let progress = Arc::clone(&progress);
                move |n| {
                    (
                        std::collections::HashMap::<u32, Vec<(pack::data::Offset, ObjectId)>>::new(
                        ),
                        Vec::new(),
                        Vec::new(),
                        progress
                            .lock()
                            .add_child_with_id(format!("thread {n}"), UNKNOWN),
                    )
                }
            },
            {
                let sorted_counts = Arc::clone(&sorted_counts);
                let oid_index_mapping = Arc::clone(&oid_index_mapping);
                move |(chunk_id, chunk_range): (SequenceId, std::ops::Range<usize>),
                      (pack_index_cache, buf_t, buf_s, progress)| {
                    let mut out = Vec::new();
                    let chunk = &sorted_counts[chunk_range];
                    let mut stats = Outcome::default();
                    progress.init(Some(chunk.len()), progress::count("objects"));

                    for count in chunk.iter() {
                        let oid = count.id;
                        let db_find_cached = |oid, buf| db.try_find(oid, buf).map_err(Error::Find);
                        let entry = if let Some(source_oid) = topo.get(&oid) {
                            let mut find_existing_delta = || -> Option<_> {
                                let (compressed_data, decompressed_size) = find_delta(
                                    count,
                                    &db,
                                    source_oid,
                                    |pack_id, base_offset| {
                                        let offsets_oid_mapping =
                                            pack_index_cache.entry(pack_id).or_insert_with(|| {
                                                db.pack_offsets_and_oid(pack_id)
                                                    .map(|mut v| {
                                                        v.sort_by_key(|e| e.0);
                                                        v
                                                    })
                                                    .expect(
                                                        "pack used for counts is still available",
                                                    )
                                            });
                                        offsets_oid_mapping
                                            .binary_search_by_key(&base_offset, |e| e.0)
                                            .ok()
                                            .map(|idx| offsets_oid_mapping[idx].1)
                                    },
                                    version,
                                )?;
                                Some(Ok(output::Entry {
                                    id: oid.to_owned(),
                                    kind: output::entry::Kind::DeltaRef {
                                        object_index: *oid_index_mapping.get(source_oid).expect(
                                            "all target and source objects should in ONE pack",
                                        ),
                                    },
                                    decompressed_size,
                                    compressed_data,
                                }))
                            };
                            if let Some(entry) = find_existing_delta() {
                                stats.objects_copied_from_pack += 1;
                                entry
                            } else if let Some((target, _)) = db_find_cached(&oid, buf_t)? {
                                if let Some((source, _)) = db_find_cached(source_oid, buf_s)? {
                                    let delta_data = delta_diff::diff(source.data, target.data)
                                        .expect("delta diff algorithm should valid");
                                    let mut deflate =
                                        gix::features::zlib::stream::deflate::Write::new(Vec::new());
                                    std::io::copy(&mut delta_data.as_slice(), &mut deflate)
                                        .map_err(|e| Error::NewEntry(e.into()))?;
                                    deflate.flush().map_err(|e| Error::NewEntry(e.into()))?;
                                    let compressed_delta = deflate.into_inner();
                                    Ok(output::Entry {
                                        id: oid.to_owned(),
                                        kind: output::entry::Kind::DeltaRef {
                                            object_index: *oid_index_mapping
                                                .get(source_oid)
                                                .expect("all target and source objects should in ONE pack"),
                                        },
                                        decompressed_size: delta_data.len(),
                                        compressed_data: compressed_delta,
                                    })
                                } else {
                                    Ok(output::Entry::invalid())
                                }
                            } else {
                                Ok(output::Entry::invalid())
                            }
                        } else if let Some((data, _)) = db_find_cached(&oid, buf_t)? {
                            output::Entry::from_data(count, &data)
                        } else {
                            Ok(output::Entry::invalid())
                        }?;
                        out.push(entry);
                        progress.inc();
                    }
                    Ok((chunk_id, out, stats))
                }
            },
            reduce::Statistics::default(),
        )
    }

    fn topo_sort(
        counts: &mut [output::Count],
        to_parent: &std::collections::HashMap<ObjectId, ObjectId>,
    ) -> Result<(), usize> {
        use std::collections::HashMap;

        type CountIndex = usize;

        let n = counts.len();
        if n == 0 {
            return Ok(());
        }

        let oid_to_idx: HashMap<ObjectId, CountIndex> = counts
            .iter()
            .enumerate()
            .map(|(idx, c)| (c.id.to_owned(), idx))
            .collect();
        let mut idx_to_child_count: HashMap<CountIndex, usize> = (0..n).map(|c| (c, 0)).collect();
        for (child, parent) in to_parent {
            let child = oid_to_idx
                .get(child)
                .expect("child ObjectId in to_parent should exist in counts");
            let parent = oid_to_idx
                .get(parent)
                .expect("parent ObjectId in to_parent should exist in counts");
            if idx_to_child_count.contains_key(child) {
                if let Some(count) = idx_to_child_count.get_mut(parent) {
                    *count += 1;
                }
            }
        }

        let mut stack: Vec<CountIndex> = idx_to_child_count
            .iter()
            .filter_map(|(&c, count)| (*count == 0).then_some(c))
            .collect();
        let mut sorted = Vec::with_capacity(n);
        while let Some(curr) = stack.pop() {
            if let Some(parent) = to_parent.get(&counts[curr].id) {
                let parent = oid_to_idx.get(parent).unwrap();
                if let Some(count) = idx_to_child_count.get_mut(parent) {
                    *count -= 1;
                    if *count == 0 {
                        stack.push(*parent);
                    }
                }
            }
            sorted.push(curr);
        }

        match sorted.len().cmp(&n) {
            Ordering::Less => Err(n - sorted.len()),
            Ordering::Equal => {
                sorted.reverse();
                util::apply_permutation(counts, &sorted);
                Ok(())
            }
            Ordering::Greater => {
                unreachable!("sorted counts should less or equal than all counts")
            }
        }
    }

    fn find_delta(
        count: &output::Count,
        db: &impl pack::Find,
        source_oid: &ObjectId,
        mut pack_offset_to_oid: impl FnMut(u32, u64) -> Option<ObjectId>,
        target_version: pack::data::Version,
    ) -> Option<(Vec<u8>, usize)> {
        let entry = count
            .entry_pack_location
            .as_ref()
            .and_then(|l| db.entry_by_location(l))?;

        if entry.version != target_version {
            return None;
        }

        let pack_offset_must_be_zero = 0;
        let pack_entry = pack::data::Entry::from_bytes(
            &entry.data,
            pack_offset_must_be_zero,
            count.id.as_slice().len(),
        )
        .ok()?;

        use pack::data::entry::Header::*;
        let source_matches = match pack_entry.header {
            OfsDelta { base_distance } => {
                let pack_location = count.entry_pack_location.as_ref().expect("packed");
                let base_offset = pack_location
                    .pack_offset
                    .checked_sub(base_distance)
                    .expect("pack-offset - distance is firmly within the pack");
                pack_offset_to_oid(pack_location.pack_id, base_offset)
            }
            RefDelta { base_id } => Some(base_id),
            _ => None,
        }
        .filter(|id| id == source_oid);

        if source_matches.is_none() {
            return None;
        }

        let compressed = entry.data[pack_entry.data_offset as usize..].to_vec();
        Some((compressed, pack_entry.decompressed_size as usize))
    }

    mod util {
        #[derive(Clone)]
        pub struct ChunkRanges {
            cursor: usize,
            size: usize,
            len: usize,
        }

        impl ChunkRanges {
            pub fn new(size: usize, total: usize) -> Self {
                ChunkRanges {
                    cursor: 0,
                    size,
                    len: total,
                }
            }
        }

        impl Iterator for ChunkRanges {
            type Item = std::ops::Range<usize>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.cursor >= self.len {
                    None
                } else {
                    let upper = (self.cursor + self.size).min(self.len);
                    let range = self.cursor..upper;
                    self.cursor = upper;
                    Some(range)
                }
            }
        }

        pub fn apply_permutation<T>(data: &mut [T], indices: &[usize]) {
            let n = data.len();

            let mut inv = vec![0; n];
            for (i, &j) in indices.iter().enumerate() {
                inv[j] = i;
            }

            for i in 0..n {
                while inv[i] != i {
                    let target = inv[i];
                    data.swap(i, target);
                    inv.swap(i, target);
                }
            }
        }
    }

    mod reduce {
        use std::marker::PhantomData;

        use super::Outcome;
        use super::pack::data::output;
        use super::{parallel, parallel::SequenceId};

        pub struct Statistics<E> {
            total: Outcome,
            _err: PhantomData<E>,
        }

        impl<E> Default for Statistics<E> {
            fn default() -> Self {
                Statistics {
                    total: Default::default(),
                    _err: PhantomData,
                }
            }
        }

        impl<Error> parallel::Reduce for Statistics<Error> {
            type Input = Result<(SequenceId, Vec<output::Entry>, Outcome), Error>;
            type FeedProduce = (SequenceId, Vec<output::Entry>);
            type Output = Outcome;
            type Error = Error;

            fn feed(&mut self, item: Self::Input) -> Result<Self::FeedProduce, Self::Error> {
                item.map(|(cid, entries, stats)| {
                    self.total.decoded_and_recompressed_objects +=
                        stats.decoded_and_recompressed_objects;
                    self.total.missing_objects += stats.missing_objects;
                    self.total.objects_copied_from_pack += stats.objects_copied_from_pack;
                    self.total.ref_delta_objects += stats.ref_delta_objects;
                    (cid, entries)
                })
            }

            fn finalize(self) -> Result<Self::Output, Self::Error> {
                Ok(self.total)
            }
        }
    }

    mod delta_diff {
        use std::io::Write;

        #[derive(thiserror::Error, Debug)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error("Failed to write bytes: {0}")]
            IOError(std::io::Error),
            #[error("Too large offset in Copy instruction, should <= 0xffffffff, got {0}")]
            TooLargeOffset(usize),
            #[error("Too large size in Copy instruction, should <= 0x00ffffff, got {0}")]
            TooLargeSize(usize),
            #[error("Too large data in Add instruction, length should <= 127, got {0}")]
            TooLargeData(usize),
        }

        #[derive(Debug)]
        pub enum Instruction<'a> {
            Copy { offset: usize, size: usize },
            Add { data: &'a [u8] },
        }

        impl Instruction<'_> {
            pub fn encode(self, mut writer: impl Write) -> Result<(), Error> {
                match self {
                    Self::Copy { offset, mut size } => {
                        let mut header = 0x80u8;
                        let mut buf = [0u8; 7];
                        let mut n = 0;

                        if size == 0x10000 {
                            size = 0;
                        } else if size > 0x00ffffff {
                            return Err(Error::TooLargeSize(size));
                        }
                        if offset > 0xffffffff {
                            return Err(Error::TooLargeOffset(offset));
                        }

                        for i in 0..4 {
                            let byte = (offset >> (i * 8)) as u8;
                            if byte != 0 {
                                header |= 1 << i;
                                buf[n] = byte;
                                n += 1;
                            }
                        }
                        for i in 0..3 {
                            let byte = (size >> (i * 8)) as u8;
                            if byte != 0 {
                                header |= 1 << (4 + i);
                                buf[n] = byte;
                                n += 1;
                            }
                        }

                        writer.write_all(&[header]).map_err(Error::IOError)?;
                        writer.write_all(&buf[..n]).map_err(Error::IOError)?;
                        Ok(())
                    }
                    Self::Add { data } => {
                        if data.len() > 127 {
                            return Err(Error::TooLargeData(data.len()));
                        }

                        let header = data.len() as u8;
                        writer.write_all(&[header]).map_err(Error::IOError)?;
                        writer.write_all(data).map_err(Error::IOError)?;
                        Ok(())
                    }
                }
            }
        }

        fn encode_delta_varint(mut value: usize, buf: &mut impl Write) -> Result<(), Error> {
            loop {
                let mut byte = (value & 0x7f) as u8;
                value >>= 7;
                if value > 0 {
                    byte |= 0x80;
                }
                buf.write_all(&[byte]).map_err(Error::IOError)?;
                if value == 0 {
                    break;
                }
            }
            Ok(())
        }

        fn compute_delta<'a>(source: &[u8], target: &'a [u8]) -> Vec<Instruction<'a>> {
            let mut insts = Vec::new();

            let mut common_prefix_len: usize = 0;
            for (s, t) in source.iter().zip(target) {
                if s == t {
                    common_prefix_len += 1;
                } else {
                    break;
                }
            }
            if common_prefix_len > 0 {
                insts.push(Instruction::Copy {
                    offset: 0,
                    size: common_prefix_len,
                });
            }

            for chunk in target[common_prefix_len..].chunks(127) {
                insts.push(Instruction::Add { data: chunk });
            }
            insts
        }

        pub fn diff(source: &[u8], target: &[u8]) -> Result<Vec<u8>, Error> {
            let mut delta_data = Vec::new();
            encode_delta_varint(source.len(), &mut delta_data)?;
            encode_delta_varint(target.len(), &mut delta_data)?;
            for inst in compute_delta(source, target) {
                inst.encode(&mut delta_data)?;
            }
            Ok(delta_data)
        }
    }
}
