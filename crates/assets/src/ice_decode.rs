//! ICE v4 Blowfish/PRS decoding with bounded input, groups, and expanded output.
//! Key derivation adapted from ages-ice-archive (MIT); see THIRD_PARTY_NOTICES.md.
use crate::ice::{IceArchive, IceError};
use ages_prs::ModernPrsDecoder;
use blowfish::{
    cipher::{Block, BlockDecrypt, KeyInit},
    BlowfishLE,
};
use std::io::{Cursor, Read};

const HEADER: usize = 0x150;
const LIMIT: usize = 128 * 1024 * 1024;
const GROUP_LIMIT: usize = 64 * 1024 * 1024;
fn error(offset: usize, message: &'static str) -> IceError {
    IceError { offset, message }
}
fn word(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}
fn put(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn decrypt(data: &mut [u8], key: u32) {
    let cipher = BlowfishLE::new_from_slice(&key.to_le_bytes())
        .expect("Blowfish accepts four-byte ICE keys");
    for chunk in data.chunks_exact_mut(8) {
        let array: [u8; 8] = (&*chunk).try_into().unwrap();
        let mut block: Block<BlowfishLE> = array.into();
        cipher.decrypt_block(&mut block);
        chunk.copy_from_slice(&block);
    }
}

fn key2(key: u32, table: &[u8]) -> u32 {
    let positions = [0x5D_usize, 0x3F, 0x45, 0xC6];
    let shifts = [8, 24, 0, 16];
    let mut result = 0;
    for i in 0..4 {
        let byte = table[(((key >> (i * 8)) as usize) + positions[i]) & 255];
        result |= (byte.rotate_left((positions[i] & 7) as u32) as u32) << shifts[i];
    }
    result
}

fn keys(size: u32, table: &[u8]) -> (u32, u32, u32) {
    let key1 = word(table, 0x6C) ^ size ^ crc32fast::hash(&table[0x7C..0xDC]) ^ 0x4352F5C2;
    let mut key = key2(key1, table) ^ 0x4352F5C2 ^ 0xCD50379E;
    let rounds = key % 7 + 2;
    for _ in 0..rounds {
        key = key2(key, table);
    }
    let key3 = key ^ 0x4352F5C2 ^ 0xCD50379E;
    (key3.rotate_left(13), key3, key2(key3, table))
}

fn decrypt_group(data: &mut [u8], first: u32, second: u32) {
    let mask = (first ^ (first >> 16)) as u8;
    for byte in data.iter_mut() {
        if *byte != 0 && *byte != mask {
            *byte ^= mask;
        }
    }
    decrypt(data, first);
    if data.len() <= 0x19000 {
        decrypt(data, second);
    }
}

/// Returns a canonical plaintext/uncompressed v4 archive, validated by the entry reader.
pub fn decode_v4(data: &[u8]) -> Result<Vec<u8>, IceError> {
    if data.len() < HEADER || data.len() > LIMIT {
        return Err(error(0, "ICE input must be between 336 bytes and 128 MiB"));
    }
    if &data[..4] != b"ICE\0" || word(data, 8) != 4 || word(data, 4) != 0 || word(data, 12) != 0x80
    {
        return Err(error(0, "only standard ICE v4 archives are supported"));
    }
    if word(data, 28) as usize != data.len() {
        return Err(error(28, "declared ICE size does not match input"));
    }
    let flags = word(data, 24);
    if flags & !1 != 0 {
        return Err(error(24, "unsupported ICE flags or Oodle compression"));
    }
    let mut descriptors = data[0x120..HEADER].to_vec();
    let (_, first, second) = keys(word(data, 28), &data[32..0x120]);
    if flags & 1 != 0 {
        decrypt(&mut descriptors, first.rotate_left(13));
    }
    let mut output = data[..HEADER].to_vec();
    put(&mut output, 24, 0);
    let mut cursor = HEADER;
    for group in 0..2 {
        let descriptor = group * 16;
        let expanded = word(&descriptors, descriptor) as usize;
        let compressed = word(&descriptors, descriptor + 4) as usize;
        let stored = if compressed == 0 {
            expanded
        } else {
            compressed
        };
        if expanded > GROUP_LIMIT
            || stored > GROUP_LIMIT
            || word(&descriptors, descriptor + 8) > 100_000
        {
            return Err(error(
                0x120 + descriptor,
                "ICE group exceeds decoding limits",
            ));
        }
        let end = cursor
            .checked_add(stored)
            .filter(|end| *end <= data.len())
            .ok_or_else(|| error(cursor, "ICE group extends beyond input"))?;
        let mut bytes = data[cursor..end].to_vec();
        cursor = end;
        if flags & 1 != 0 {
            let rotation = if group == 0 { 0 } else { 17 };
            decrypt_group(
                &mut bytes,
                first.rotate_left(rotation),
                second.rotate_left(rotation),
            );
        }
        if compressed != 0 {
            for byte in &mut bytes {
                *byte ^= 0x95;
            }
            let decoder = ModernPrsDecoder::new(Cursor::new(bytes));
            bytes = Vec::new();
            decoder
                .take(expanded as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| error(cursor - stored, "invalid PRS compressed group"))?;
            if bytes.len() != expanded {
                return Err(error(
                    cursor - stored,
                    "expanded group size does not match header",
                ));
            }
        }
        output.extend_from_slice(&bytes);
        put(&mut descriptors, descriptor + 4, 0);
    }
    if cursor != data.len() {
        return Err(error(cursor, "unexpected trailing ICE data"));
    }
    output[0x120..HEADER].copy_from_slice(&descriptors);
    let size = output.len() as u32;
    put(&mut output, 28, size);
    IceArchive::parse(&output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blowfish::cipher::BlockEncrypt;
    use std::io::Write;

    fn encrypt(data: &mut [u8], key: u32) {
        let cipher = BlowfishLE::new_from_slice(&key.to_le_bytes()).unwrap();
        for chunk in data.chunks_exact_mut(8) {
            let array: [u8; 8] = (&*chunk).try_into().unwrap();
            let mut block: Block<BlowfishLE> = array.into();
            cipher.encrypt_block(&mut block);
            chunk.copy_from_slice(&block);
        }
    }

    pub(crate) fn archive(
        payload: &[u8],
        name: &str,
        encrypted: bool,
        compressed: bool,
    ) -> Vec<u8> {
        let header_size = (64 + name.len() + 1 + 15) & !15;
        let entry_size = (header_size + payload.len() + 15) & !15;
        let mut group = vec![0; entry_size];
        group[..4].copy_from_slice(b"aqp\0");
        put(&mut group, 4, entry_size as u32);
        put(&mut group, 8, payload.len() as u32);
        put(&mut group, 12, header_size as u32);
        put(&mut group, 16, (name.len() + 1) as u32);
        group[64..64 + name.len()].copy_from_slice(name.as_bytes());
        group[header_size..header_size + payload.len()].copy_from_slice(payload);
        if compressed {
            let mut encoder = ages_prs::ModernPrsEncoder::new(Vec::new());
            encoder.write_all(&group).unwrap();
            group = encoder.into_inner().unwrap();
            for byte in &mut group {
                *byte ^= 0x95;
            }
        }
        let mut data = vec![0; HEADER];
        data[..4].copy_from_slice(b"ICE\0");
        put(&mut data, 8, 4);
        put(&mut data, 12, 0x80);
        put(&mut data, 24, u32::from(encrypted));
        put(&mut data, 28, (HEADER + group.len()) as u32);
        for (i, byte) in data[32..0x120].iter_mut().enumerate() {
            *byte = i as u8;
        }
        put(&mut data, 0x130, entry_size as u32);
        put(&mut data, 0x138, 1);
        if compressed {
            put(&mut data, 0x134, group.len() as u32);
        }
        if encrypted {
            let (header, first, second) = keys(word(&data, 28), &data[32..0x120]);
            let a = first.rotate_left(17);
            let b = second.rotate_left(17);
            if group.len() <= 0x19000 {
                encrypt(&mut group, b);
            }
            encrypt(&mut group, a);
            let mask = (a ^ (a >> 16)) as u8;
            for byte in &mut group {
                if *byte != 0 && *byte != mask {
                    *byte ^= mask;
                }
            }
            encrypt(&mut data[0x120..HEADER], header);
        }
        data.extend_from_slice(&group);
        data
    }

    #[test]
    fn decrypts_and_decompresses_bounded_synthetic_archives() {
        for encrypted in [false, true] {
            for compressed in [false, true] {
                let data = archive(
                    b"original synthetic payload",
                    "sample.aqp",
                    encrypted,
                    compressed,
                );
                let decoded = decode_v4(&data).unwrap();
                let parsed = IceArchive::parse(&decoded).unwrap();
                assert_eq!(parsed.entries[0].data, b"original synthetic payload");
            }
        }
    }

    #[test]
    fn decodes_synthetic_model_through_common_import_entrypoint() {
        let data = archive(&crate::aqua::fixture(), "sample.aqp", true, true);
        let imported = crate::import::ImportedModel::parse(&data).unwrap();
        assert_eq!(imported.model.triangles, vec![[0, 1, 2]]);
    }

    #[test]
    fn rejects_unsupported_and_oversized_groups() {
        let data = archive(b"test", "sample.aqp", false, false);
        for (offset, value) in [
            (24, 8),
            (0x130, (GROUP_LIMIT + 1) as u32),
            (0x134, u32::MAX),
            (0x138, 100001),
            (28, 0),
        ] {
            let mut bad = data.clone();
            put(&mut bad, offset, value);
            assert!(decode_v4(&bad).is_err());
        }
        for length in 0..HEADER {
            assert!(decode_v4(&data[..length]).is_err());
        }
    }
}
