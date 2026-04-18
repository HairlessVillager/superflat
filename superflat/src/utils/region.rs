use anyhow::{Context, Result};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use simdnbt::owned::{BaseNbt, NbtCompound, NbtList};
use simdnbt::{Deserialize, Serialize, borrow, owned};
use std::io::{Read, Seek, SeekFrom::Start as SeekStart, Write};

use super::palette::{dump_biome, dump_block, load_biome, load_block};

const SECTOR_SIZE: usize = 4096;

/// Parse a .mca region file into its timestamp header and chunks.
/// Returns None if the file is empty or has no chunks.
#[must_use]
pub fn read_region<B: Read + Seek>(
    mut buf: B,
    region_x: i32,
    region_z: i32,
) -> Result<Option<([u8; 4096], Vec<(i32, i32, Vec<u8>)>)>> {
    let mut locations = [0u8; 4096];
    if let Err(err) = buf.read_exact(&mut locations) {
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            return Ok(None);
        }
    }

    let mut timestamps = [0u8; 4096];
    buf.read_exact(&mut timestamps)
        .context("buffer's length < 8192")?;

    let mut compressed_chunks = Vec::new();

    for i in 0..1024usize {
        let loc = &locations[i * 4..(i + 1) * 4];
        let offset = u32::from_be_bytes([0, loc[0], loc[1], loc[2]]) as usize;
        let size = loc[3] as usize;

        if offset == 0 && size == 0 {
            continue;
        }

        let byte_offset = offset * SECTOR_SIZE;
        buf.seek(SeekStart(byte_offset as u64))
            .with_context(|| format!("at chunk #{i}: failed to seek {byte_offset}"))?;

        let mut header = [0u8; 5];
        buf.read_exact(&mut header)
            .with_context(|| format!("at chunk #{i}: failed to read chunk header"))?;

        let data_length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let compression_type = header[4];

        let compressed_len = data_length.saturating_sub(1);
        let mut compressed_data = vec![0u8; compressed_len];
        buf.read_exact(&mut compressed_data).with_context(|| {
            format!("at chunk #{i}: failed to read chunk body (length: {compressed_len})")
        })?;

        compressed_chunks.push((i, compression_type, compressed_data));
    }

    let chunks: Vec<(i32, i32, Vec<u8>)> = compressed_chunks
        .into_par_iter()
        .filter_map(|(i, compression_type, compressed)| {
            if compression_type == 2 {
                let mut decoder = ZlibDecoder::new(&compressed[..]);
                let mut nbt = Vec::new();
                if decoder.read_to_end(&mut nbt).is_ok() {
                    let local_x = (i % 32) as i32;
                    let local_z = (i / 32) as i32;
                    return Some((region_x * 32 + local_x, region_z * 32 + local_z, nbt));
                }
            } else {
                todo!("Support compression type {compression_type}")
            }
            None
        })
        .collect();

    if chunks.is_empty() {
        return Ok(None);
    }

    Ok(Some((timestamps, chunks)))
}

/// Reconstruct a .mca region file from a timestamp header and chunks.
pub fn write_region<B: Write + Seek>(
    region_x: i32,
    region_z: i32,
    timestamp_header: &[u8; 4096],
    chunks: impl IntoParallelIterator<Item = (i32, i32, impl AsRef<[u8]>)>,
    mut buf: B,
) -> Result<()> {
    buf.seek(SeekStart(4096)).context("failed to seek 4096")?;
    buf.write(timestamp_header)
        .context("failed to write timestamp header")?;

    let mut current_sector = 2usize;
    let chunks = chunks
        .into_par_iter()
        .map(|(chunk_x, chunk_z, nbt)| {
            let mut encoder = ZlibEncoder::new(Vec::with_capacity(8192), Compression::default());
            encoder.write_all(nbt.as_ref()).with_context(|| {
                format!("at chunk ({chunk_x}, {chunk_z}): failed to feed data into encoder")
            })?;
            let compressed = encoder.finish().with_context(|| {
                format!("at chunk ({chunk_x}, {chunk_z}): failed to finalize compression")
            })?;
            Ok((chunk_x, chunk_z, compressed))
        })
        .collect::<Result<Vec<_>>>()?;

    for (chunk_x, chunk_z, compressed) in chunks {
        let local_x = chunk_x - (region_x * 32);
        let local_z = chunk_z - (region_z * 32);
        let index = (local_x + local_z * 32) as usize;

        let content_length = compressed.len() + 1; // + 1 for the compression type byte
        let mut payload = Vec::with_capacity(4 + 1 + compressed.len());
        payload.extend_from_slice(&(content_length as u32).to_be_bytes());
        payload.push(2u8); // 2 means using zlib to compress
        payload.extend_from_slice(&compressed);

        let sectors_needed = payload.len().div_ceil(SECTOR_SIZE);
        if sectors_needed > 255 {
            todo!("Write big chunk (> 1020KiB) to .mcc file")
        }
        let padding = sectors_needed * SECTOR_SIZE - payload.len();

        let loc_offset = index * 4;
        let sector_bytes = (current_sector as u32).to_be_bytes();

        buf.seek(SeekStart(loc_offset as u64)).with_context(|| {
            format!("at chunk ({chunk_x}, {chunk_z}): failed to seek header@{loc_offset}")
        })?;
        buf.write(&[
            sector_bytes[1],
            sector_bytes[2],
            sector_bytes[3],
            sectors_needed as u8,
        ])
        .with_context(|| format!("at chunk ({chunk_x}, {chunk_z}): failed to write header"))?;

        buf.seek(SeekStart((current_sector * SECTOR_SIZE) as u64))
            .with_context(|| {
                format!(
                    "at chunk ({chunk_x}, {chunk_z}): failed to seek chunk@{}",
                    current_sector * SECTOR_SIZE
                )
            })?;
        buf.write(&payload)
            .with_context(|| format!("at chunk ({chunk_x}, {chunk_z}): failed to write payload"))?;

        buf.write(&std::iter::repeat_n(0u8, padding).collect::<Vec<u8>>())
            .with_context(|| format!("at chunk ({chunk_x}, {chunk_z}): failed to write padding"))?;

        current_sector += sectors_needed;
    }
    Ok(())
}

/// Parse (region_x, region_z) from a filename like "r.-1.2.mca".
#[must_use]
pub fn parse_xz(filename: &str) -> Result<(i32, i32)> {
    let parts: Vec<&str> = filename.split('.').collect();
    let x: i32 = parts[1]
        .parse()
        .with_context(|| format!("failed to parse {} as i32", parts[1]))?;
    let z: i32 = parts[2]
        .parse()
        .with_context(|| format!("failed to parse {} as i32", parts[1]))?;
    Ok((x, z))
}

fn to_nbt_tag_vec_u8(data: Vec<u8>) -> owned::NbtTag {
    owned::NbtTag::ByteArray(data)
}

fn from_nbt_tag_vec_u8(tag: borrow::NbtTag) -> Option<Vec<u8>> {
    tag.byte_array().map(|x| x.to_owned())
}

fn to_nbt_tag_vec_u16(data: Vec<u16>) -> owned::NbtTag {
    owned::NbtTag::List(NbtList::from(
        data.into_iter().map(|x| x as i16).collect::<Vec<_>>(),
    ))
}

fn from_nbt_tag_vec_u16(tag: borrow::NbtTag) -> Option<Vec<u16>> {
    Some(
        tag.list()?
            .shorts()?
            .into_iter()
            .map(|x| x as u16)
            .collect::<Vec<_>>(),
    )
}

pub struct Section {
    pub y: i8,
    pub biome: Vec<u8>,
    pub block_state: Vec<u16>,
}

impl simdnbt::Serialize for Section {
    fn to_compound(self) -> simdnbt::owned::NbtCompound {
        let mut nbt = simdnbt::owned::NbtCompound::new();
        if let Some(item) = simdnbt::ToNbtTag::to_optional_nbt_tag(self.y) {
            nbt.insert("y", item);
        }
        nbt.insert("biome", to_nbt_tag_vec_u8(self.biome));
        nbt.insert("block_state", to_nbt_tag_vec_u16(self.block_state));
        nbt
    }
}

impl simdnbt::Deserialize for Section {
    fn from_compound(nbt: simdnbt::borrow::NbtCompound) -> Result<Self, simdnbt::DeserializeError> {
        let value = Self {
            y: simdnbt::FromNbtTag::from_optional_nbt_tag(nbt.get("y"))?.ok_or(
                simdnbt::DeserializeError::MismatchedFieldType("Section::y".to_owned()),
            )?,
            biome: {
                let x = nbt
                    .get("biome")
                    .ok_or(simdnbt::DeserializeError::MissingField)?;
                let x = from_nbt_tag_vec_u8(x).ok_or(
                    simdnbt::DeserializeError::MismatchedFieldType("Section::biome".to_owned()),
                )?;
                Ok(x)
            }?,
            block_state: {
                let x = nbt
                    .get("block_state")
                    .ok_or(simdnbt::DeserializeError::MissingField)?;
                let x = from_nbt_tag_vec_u16(x).ok_or(simdnbt::DeserializeError::MissingField)?;
                Ok(x)
            }?,
        };
        Ok(value)
    }
}

#[derive(Serialize, Deserialize)]
pub struct SectionsDump {
    pub sections: Vec<Section>,
}

fn dump_sections(sections: &NbtList) -> Result<SectionsDump> {
    let sections = sections
        .compounds()
        .context("expect sections is a NBT compound list, got: {sections:#?}")?
        .iter()
        .enumerate()
        .map(|(idx, section)| {
            let y = section.byte("Y").with_context(|| {
                format!("missing NBT byte 'sections.{idx}.Y', got: {section:#?}")
            })?;
            let biome_dump = {
                let Some(biome) = section.compound("biomes") else {
                    log::warn!(
                        "missing 'sections.{idx}.biomes' (y={y}), all fields got: {:?}",
                        section.keys().collect::<Vec<_>>()
                    );
                    return Ok(None);
                };
                dump_biome(biome)?.as_flattened().as_flattened().into()
            };
            let block_dump = {
                let Some(block_states) = section.compound("block_states") else {
                    log::warn!(
                        "missing 'sections.{idx}.block_states' (y={y}), all fields got: {:?}",
                        section.keys().collect::<Vec<_>>()
                    );
                    return Ok(None);
                };
                dump_block(block_states)?
                    .as_flattened()
                    .as_flattened()
                    .into()
            };
            // TODO: extract block/sky light
            Ok(Some(Section {
                y,
                biome: biome_dump,
                block_state: block_dump,
            }))
        })
        .map(|e: Result<Option<Section>>| e.transpose())
        .filter_map(|e| e)
        .collect::<Result<_, _>>()?;
    Ok(SectionsDump { sections })
}

fn load_sections(dump: SectionsDump) -> Result<NbtList> {
    let list = dump
        .sections
        .into_iter()
        .map(|section| {
            let kvs = vec![
                ("Y".into(), owned::NbtTag::Byte(section.y)),
                (
                    "biomes".into(),
                    owned::NbtTag::Compound(load_biome(bytemuck::cast_box(
                        Box::<[u8; 64]>::try_from(section.biome.into_boxed_slice())
                            .map_err(|_| anyhow::anyhow!("vec length does not match S^3"))?,
                    ))?),
                ),
                (
                    "block_states".into(),
                    owned::NbtTag::Compound(load_block(bytemuck::cast_box(
                        Box::<[u16; 4096]>::try_from(section.block_state.into_boxed_slice())
                            .map_err(|_| anyhow::anyhow!("vec length does not match S^3"))?,
                    ))?),
                ),
            ];
            Ok(NbtCompound::from_values(kvs))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(NbtList::from(list))
}

/// Split a chunk nbt into (other_nbt, sections_dump)
pub fn split_chunk(nbt: BaseNbt) -> Result<(BaseNbt, SectionsDump)> {
    let name = nbt.name().to_owned();
    let mut nbt = nbt.as_compound();
    let sections_dump = {
        let sections = nbt.remove("sections").ok_or_else(|| {
            anyhow::anyhow!(format!(
                "missing 'sections', all fields: {:#?}",
                nbt.keys().collect::<Vec<_>>()
            ))
        })?;
        let sections = sections
            .list()
            .ok_or(anyhow::anyhow!("expect sections is a NBT list"))?;
        dump_sections(sections)?
    };

    // TODO: extract block/sky light
    if let Some(is_light_on_idx) = nbt.byte_mut("isLightOn") {
        *is_light_on_idx = i8::from(false);
    } else {
        anyhow::bail!(
            "missing 'isLightOn', all fields: {:#?}",
            nbt.keys().collect::<Vec<_>>()
        );
        // nbt.root_tag.put_bool("isLightOn", false);
    }

    Ok((BaseNbt::new(name, nbt), sections_dump))
}

/// Restore a chunk nbt from (other_nbt, sections_dump)
pub fn restore_chunk(other: BaseNbt, dump: SectionsDump) -> Result<BaseNbt> {
    let name = other.name().to_owned();
    let mut other = other.as_compound();
    other.insert("sections", load_sections(dump)?);
    Ok(BaseNbt::new(name, other))
}
