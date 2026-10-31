//! Classic LAT stage tile metadata. This describes layouts, not rendered terrain.
use crate::aqua::{AquaError, Reader};

#[derive(Debug, PartialEq, Eq)]
pub struct LandCell {
    pub piece_id: u16,
    pub rotation_count: u16,
    pub variant: u16,
    pub object_variant: u16,
    pub category: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LandTemplate {
    pub width: usize,
    pub height: usize,
    /// File order: rows of `width` cells. World axes and spacing are not inferred.
    pub cells: Vec<LandCell>,
}

fn fail(offset: usize, message: &'static str) -> AquaError {
    AquaError { offset, message }
}

impl LandTemplate {
    pub fn parse(bytes: &[u8]) -> Result<Self, AquaError> {
        let bytes = bytes.strip_prefix(b"lat\0").unwrap_or(bytes);
        let outer = Reader(bytes);
        if outer.bytes(0, 4)? != b"NIFL" || outer.word(12)? != 32 || outer.bytes(32, 4)? != b"REL0"
        {
            return Err(fail(0, "expected NIFL/REL0 land template"));
        }
        let length = outer
            .word(36)?
            .checked_add(8)
            .ok_or_else(|| fail(36, "REL0 size overflow"))?;
        let r = Reader(outer.bytes(32, length)?);
        let root = r.word(8)?;
        r.bytes(root, 24)?;
        let height = r.word(root)?;
        let width = r.word(root + 4)?;
        if width == 0 || height == 0 || width > 512 || height > 512 {
            return Err(fail(root, "unsupported land grid dimensions"));
        }
        let count = width
            .checked_mul(height)
            .filter(|n| *n <= 65_536)
            .ok_or_else(|| fail(root, "land grid exceeds 65536 cells"))?;
        let categories = r.word(root + 12)?;
        let cells = r.word(root + 16)?;
        if categories < 16 || cells < 16 {
            return Err(fail(root, "null or header-relative land table pointer"));
        }
        r.table(categories, count, 4)?;
        r.table(cells, count, 36)?;
        let mut output = Vec::with_capacity(count);
        for i in 0..count {
            let data = r.bytes(cells + i * 36, 36)?;
            let word = |offset| u16::from_le_bytes([data[offset], data[offset + 1]]);
            output.push(LandCell {
                piece_id: word(0),
                rotation_count: word(4),
                variant: word(10),
                object_variant: word(12),
                category: r.bytes(categories + i * 4, 1)?[0],
            });
        }
        Ok(Self {
            width,
            height,
            cells: output,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut bytes = vec![0; 32 + 144];
        bytes[..4].copy_from_slice(b"NIFL");
        bytes[32..36].copy_from_slice(b"REL0");
        for (offset, value) in [
            (12, 32_u32),
            (36, 136),
            (40, 16),
            (48, 1),
            (52, 2),
            (60, 40),
            (64, 48),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[72] = 3;
        bytes[80..82].copy_from_slice(b"cg");
        bytes[84..86].copy_from_slice(&2_u16.to_le_bytes());
        bytes[90..92].copy_from_slice(&u16::MAX.to_le_bytes());
        bytes[92..94].copy_from_slice(&5_u16.to_le_bytes());
        bytes
    }

    #[test]
    fn reads_tile_ids_variants_rotation_and_empty_cells() {
        let template = LandTemplate::parse(&fixture()).unwrap();
        assert_eq!((template.width, template.height), (2, 1));
        assert_eq!(
            template.cells[0],
            LandCell {
                piece_id: u16::from_le_bytes(*b"cg"),
                rotation_count: 2,
                variant: u16::MAX,
                object_variant: 5,
                category: 3
            }
        );
        assert_eq!(template.cells[1].piece_id, 0);
        assert_eq!(
            LandTemplate::parse(&[b"lat\0".as_slice(), &fixture()].concat()).unwrap(),
            template
        );
    }

    #[test]
    fn rejects_truncated_oversized_and_escaping_tables() {
        let source = fixture();
        for end in 0..source.len() - 24 {
            assert!(LandTemplate::parse(&source[..end]).is_err(), "length {end}");
        }
        for (offset, value) in [
            (48, 0),
            (52, 513),
            (48, u32::MAX),
            (60, 0),
            (64, 0),
            (64, 136),
        ] {
            let mut bytes = source.clone();
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(LandTemplate::parse(&bytes).is_err());
        }
    }
}
