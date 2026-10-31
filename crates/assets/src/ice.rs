//! Bounded, borrowing reader for plaintext, uncompressed ICE v4 containers.
//! Encrypted and compressed archives fail explicitly instead of guessing a codec.
use std::fmt;

const GROUP_HEADERS: usize = 0x120;
const GROUP_DATA: usize = 0x150;
const ENTRY_HEADER: usize = 0x40;
const MAX_ENTRIES: usize = 100_000;

#[derive(Debug, PartialEq, Eq)]
pub struct IceError {
    pub offset: usize,
    pub message: &'static str,
}

impl fmt::Display for IceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ICE offset {:#x}: {}", self.offset, self.message)
    }
}
impl std::error::Error for IceError {}

fn error(offset: usize, message: &'static str) -> IceError {
    IceError { offset, message }
}

fn range(data: &[u8], start: usize, size: usize) -> Result<&[u8], IceError> {
    let end = start
        .checked_add(size)
        .ok_or_else(|| error(start, "range overflow"))?;
    data.get(start..end)
        .ok_or_else(|| error(start, "truncated data"))
}

fn word(data: &[u8], offset: usize) -> Result<u32, IceError> {
    Ok(u32::from_le_bytes(
        range(data, offset, 4)?.try_into().unwrap(),
    ))
}

#[derive(Debug)]
pub struct IceEntry<'a> {
    pub group: u8,
    pub name: &'a str,
    pub extension: &'a str,
    pub offset: usize,
    pub data: &'a [u8],
}

#[derive(Debug)]
pub struct IceArchive<'a> {
    pub entries: Vec<IceEntry<'a>>,
}

impl<'a> IceArchive<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, IceError> {
        if range(data, 0, 4)? != b"ICE\0" {
            return Err(error(0, "invalid magic"));
        }
        if word(data, 8)? != 4 {
            return Err(error(8, "only ICE version 4 is supported"));
        }
        if word(data, 4)? != 0 || word(data, 12)? != 0x80 {
            return Err(error(4, "unsupported header layout"));
        }
        if word(data, 24)? != 0 {
            return Err(error(
                24,
                "encrypted or flagged ICE archives are not supported",
            ));
        }
        if word(data, 28)? as usize != data.len() {
            return Err(error(28, "declared size does not match input"));
        }
        range(data, 0, GROUP_DATA)?;
        let mut archive = Self {
            entries: Vec::new(),
        };
        let mut group_start = GROUP_DATA;
        for group in 0..2 {
            let descriptor = GROUP_HEADERS + group * 16;
            let size = word(data, descriptor)? as usize;
            if word(data, descriptor + 4)? != 0 {
                return Err(error(
                    descriptor + 4,
                    "compressed ICE groups are not supported",
                ));
            }
            let count = word(data, descriptor + 8)? as usize;
            if count > MAX_ENTRIES || count > size / ENTRY_HEADER {
                return Err(error(descriptor + 8, "entry count exceeds group bounds"));
            }
            let bytes = range(data, group_start, size)?;
            let mut cursor = 0;
            for _ in 0..count {
                let header = range(bytes, cursor, ENTRY_HEADER).map_err(|mut error| {
                    error.offset += group_start;
                    error
                })?;
                let entry_size = word(header, 4)? as usize;
                let payload_size = word(header, 8)? as usize;
                let header_size = word(header, 12)? as usize;
                let name_size = word(header, 16)? as usize;
                if header_size < ENTRY_HEADER || header_size > entry_size {
                    return Err(error(group_start + cursor, "invalid entry header size"));
                }
                let entry = range(bytes, cursor, entry_size).map_err(|mut error| {
                    error.offset += group_start;
                    error
                })?;
                if payload_size > entry_size - header_size {
                    return Err(error(group_start + cursor, "payload extends beyond entry"));
                }
                if name_size == 0 || name_size > header_size - ENTRY_HEADER {
                    return Err(error(group_start + cursor, "invalid filename length"));
                }
                let name = range(entry, ENTRY_HEADER, name_size)?;
                if name.last() != Some(&0) || name[..name_size - 1].contains(&0) {
                    return Err(error(
                        group_start + cursor,
                        "filename is not null terminated",
                    ));
                }
                let name = std::str::from_utf8(&name[..name_size - 1])
                    .map_err(|_| error(group_start + cursor, "filename is not UTF-8"))?;
                let extension_end = header[..4].iter().position(|byte| *byte == 0).unwrap_or(4);
                let extension = std::str::from_utf8(&header[..extension_end])
                    .map_err(|_| error(group_start + cursor, "extension is not UTF-8"))?;
                archive.entries.push(IceEntry {
                    group: group as u8 + 1,
                    name,
                    extension,
                    offset: group_start + cursor + header_size,
                    data: range(entry, header_size, payload_size)?,
                });
                cursor += entry_size;
            }
            if cursor != size {
                return Err(error(group_start + cursor, "unaccounted bytes in group"));
            }
            group_start += size;
        }
        if group_start != data.len() {
            return Err(error(group_start, "unaccounted bytes after groups"));
        }
        Ok(archive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn fixture() -> Vec<u8> {
        let mut data = vec![0; GROUP_DATA + 96];
        data[..4].copy_from_slice(b"ICE\0");
        put(&mut data, 8, 4);
        put(&mut data, 12, 0x80);
        let length = data.len() as u32;
        put(&mut data, 28, length);
        put(&mut data, GROUP_HEADERS + 16, 96);
        put(&mut data, GROUP_HEADERS + 24, 1);
        let entry = &mut data[GROUP_DATA..];
        entry[..4].copy_from_slice(b"text");
        put(entry, 4, 96);
        put(entry, 8, 4);
        put(entry, 12, 80);
        put(entry, 16, 12);
        entry[64..76].copy_from_slice(b"sample.text\0");
        entry[80..84].copy_from_slice(b"TEST");
        data
    }

    #[test]
    fn lists_borrowed_entries_with_empty_first_group() {
        let data = fixture();
        let archive = IceArchive::parse(&data).unwrap();
        let entry = &archive.entries[0];
        assert_eq!(
            (entry.group, entry.name, entry.extension),
            (2, "sample.text", "text")
        );
        assert_eq!(entry.data, b"TEST");
        assert_eq!(entry.offset, GROUP_DATA + 80);
    }

    #[test]
    fn truncated_archives_and_bad_entry_bounds_return_errors() {
        let data = fixture();
        for length in 0..data.len() {
            assert!(IceArchive::parse(&data[..length]).is_err());
        }
        for (offset, value) in [
            (GROUP_DATA + 4, 0),
            (GROUP_DATA + 8, 1000),
            (GROUP_DATA + 12, 63),
            (GROUP_DATA + 12, 1000),
            (GROUP_DATA + 16, 0),
            (GROUP_DATA + 16, 1000),
            (GROUP_HEADERS + 24, 100_001),
        ] {
            let mut bad = fixture();
            put(&mut bad, offset, value);
            assert!(IceArchive::parse(&bad).is_err(), "offset {offset}");
        }
    }

    #[test]
    fn rejects_compression_encryption_unknown_versions_and_bad_names() {
        for (offset, value) in [(8, 5), (24, 1), (GROUP_HEADERS + 20, 64)] {
            let mut data = fixture();
            put(&mut data, offset, value);
            assert!(IceArchive::parse(&data).is_err());
        }
        let mut data = fixture();
        data[GROUP_DATA + 75] = b'x';
        assert!(IceArchive::parse(&data).is_err());
    }

    #[test]
    fn preserves_both_groups_and_rejects_unaccounted_data() {
        let mut data = fixture();
        data.extend_from_within(GROUP_DATA..);
        let length = data.len() as u32;
        put(&mut data, 28, length);
        put(&mut data, GROUP_HEADERS, 96);
        put(&mut data, GROUP_HEADERS + 8, 1);
        let archive = IceArchive::parse(&data).unwrap();
        assert_eq!(archive.entries.len(), 2);
        assert_eq!(archive.entries[0].group, 1);
        assert_eq!(archive.entries[1].group, 2);
        assert_eq!(archive.entries[1].offset - archive.entries[0].offset, 96);
        put(&mut data, GROUP_HEADERS + 24, 0);
        assert!(IceArchive::parse(&data).is_err());
    }
}
