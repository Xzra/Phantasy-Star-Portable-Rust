//! Geometry and skin influences from Classic PSO2 NIFL objects (0xC2A / 0xC31).
//! AFP packages use their first model/LOD; companion AQN/AQM data lives in `rig`.
use crate::Model;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct AquaError {
    pub offset: usize,
    pub message: &'static str,
}
impl fmt::Display for AquaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Aqua offset {:#x}: {}", self.offset, self.message)
    }
}
impl std::error::Error for AquaError {}
fn error(offset: usize, message: &'static str) -> AquaError {
    AquaError { offset, message }
}

pub(crate) struct Reader<'a>(pub &'a [u8]);
impl<'a> Reader<'a> {
    pub(crate) fn bytes(&self, offset: usize, size: usize) -> Result<&'a [u8], AquaError> {
        let end = offset
            .checked_add(size)
            .ok_or_else(|| error(offset, "range overflow"))?;
        self.0
            .get(offset..end)
            .ok_or_else(|| error(offset, "truncated or out-of-bounds data"))
    }
    pub(crate) fn word(&self, offset: usize) -> Result<usize, AquaError> {
        Ok(u32::from_le_bytes(self.bytes(offset, 4)?.try_into().unwrap()) as usize)
    }
    pub(crate) fn floats<const N: usize>(&self, offset: usize) -> Result<[f32; N], AquaError> {
        let bytes = self.bytes(offset, N * 4)?;
        let values = std::array::from_fn(|i| {
            f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap())
        });
        if values.iter().any(|v| !v.is_finite()) {
            return Err(error(offset, "non-finite vertex attribute"));
        }
        Ok(values)
    }
    pub(crate) fn table(
        &self,
        offset: usize,
        count: usize,
        stride: usize,
    ) -> Result<(), AquaError> {
        let size = count
            .checked_mul(stride)
            .ok_or_else(|| error(offset, "table size overflow"))?;
        self.bytes(offset, size)?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct VertexSet {
    stride: usize,
    count: usize,
    data: usize,
    position: usize,
    normal: Option<usize>,
    uv: Option<usize>,
    weights: Option<usize>,
    joints: Option<usize>,
    palette: usize,
    palette_count: usize,
}

fn vertex_set(reader: &Reader<'_>, offset: usize) -> Result<VertexSet, AquaError> {
    reader.bytes(offset, 52)?;
    let stride = reader.word(offset)?;
    let count = reader.word(offset + 12)?;
    let data = reader.word(offset + 16)?;
    if stride == 0 || stride > 4096 || count > 1_000_000 {
        return Err(error(offset, "unsupported vertex size or count"));
    }
    reader.table(data, count, stride)?;
    let layout_count = reader.word(offset + 4)?;
    let layout = reader.word(offset + 8)?;
    if layout_count > 64 {
        return Err(error(offset + 4, "too many vertex attributes"));
    }
    reader.table(layout, layout_count, 16)?;
    let mut position = None;
    let mut normal = None;
    let mut uv = None;
    let mut weights = None;
    let mut joints = None;
    for i in 0..layout_count {
        let field = layout + i * 16;
        let kind = reader.word(field)?;
        let format = reader.word(field + 4)?;
        let relative = reader.word(field + 8)?;
        let width = match kind {
            1 if format == 4 => 16,
            11 if format == 7 => 4,
            1 | 11 => return Err(error(field, "unsupported skin weight or joint format")),
            0 | 2 => {
                if format != 3 {
                    return Err(error(
                        field,
                        "only float32 position/normal attributes are supported",
                    ));
                }
                12
            }
            0x10 => {
                if format != 2 {
                    return Err(error(field, "only float32 primary UVs are supported"));
                }
                8
            }
            _ => continue,
        };
        if relative.checked_add(width).is_none_or(|end| end > stride) {
            return Err(error(field, "attribute extends beyond vertex stride"));
        }
        match kind {
            0 => position = Some(relative),
            2 => normal = Some(relative),
            0x10 => uv = Some(relative),
            1 => weights = Some(relative),
            11 => joints = Some(relative),
            _ => unreachable!(),
        }
    }
    let palette_count = reader.word(offset + 24)?;
    let palette = reader.word(offset + 28)?;
    if palette_count > 4096 || weights.is_some() != joints.is_some() {
        return Err(error(offset, "invalid skin layout or bone palette count"));
    }
    reader.table(palette, palette_count, 2)?;
    Ok(VertexSet {
        stride,
        count,
        data,
        position: position.ok_or_else(|| error(offset, "vertex layout has no positions"))?,
        normal,
        uv,
        weights,
        joints,
        palette,
        palette_count,
    })
}

impl Model {
    pub fn from_aqua(bytes: &[u8]) -> Result<Self, AquaError> {
        Ok(parse(bytes)?.0)
    }
}

pub fn parse(bytes: &[u8]) -> Result<(Model, Option<Vec<crate::rig::Influence>>), AquaError> {
    let outer = Reader(bytes);
    let bytes = match outer.bytes(0, 4)? {
        b"afp\0" => {
            if outer.word(4)? == 0 {
                return Err(error(4, "empty AFP package"));
            }
            outer.bytes(16, 48)?;
            let header_size = outer.word(16 + 36)?;
            let total = outer.word(16 + 40)?;
            if header_size != 48 || total < header_size {
                return Err(error(16, "unsupported AFP model envelope"));
            }
            outer.bytes(16 + header_size, total - header_size)?
        }
        b"aqo\0" => outer.bytes(4, bytes.len() - 4)?,
        b"NIFL" => bytes,
        _ => return Err(error(0, "expected an AFP package or NIFL Aqua object")),
    };
    let nifl = Reader(bytes);
    if nifl.bytes(0, 4)? != b"NIFL" || nifl.word(12)? != 32 {
        return Err(error(0, "unsupported NIFL header"));
    }
    if nifl.bytes(32, 4)? != b"REL0" {
        return Err(error(32, "missing REL0 chunk"));
    }
    let rel_size = nifl
        .word(36)?
        .checked_add(8)
        .ok_or_else(|| error(36, "REL0 size overflow"))?;
    let rel = Reader(nifl.bytes(32, rel_size)?);
    let root = rel.word(8)?;
    rel.bytes(root, 0xA4)?;
    let version = rel.word(root)?;
    if !matches!(version, 0xC2A | 0xC31) {
        return Err(error(
            root,
            "only Classic Aqua types 0xC2A and 0xC31 are supported",
        ));
    }
    let vertex_count = rel.word(root + 36)?;
    let vertices = rel.word(root + 40)?;
    let strip_count = rel.word(root + 44)?;
    let strips = rel.word(root + 48)?;
    let mesh_count = rel.word(root + 52)?;
    let meshes = rel.word(root + 56)?;
    if mesh_count > 10000 {
        return Err(error(root + 52, "too many meshes"));
    }
    rel.table(vertices, vertex_count, 52)?;
    rel.table(strips, strip_count, 24)?;
    rel.table(meshes, mesh_count, 48)?;
    let mut model = Model::default();
    let mut skin = Vec::new();
    let mut has_skin = false;
    for mesh in 0..mesh_count {
        let mesh_offset = meshes + mesh * 48;
        let vertex_index = rel.word(mesh_offset + 28)?;
        let strip_index = rel.word(mesh_offset + 32)?;
        if vertex_index >= vertex_count || strip_index >= strip_count {
            return Err(error(
                mesh_offset,
                "mesh references an invalid vertex or polygon set",
            ));
        }
        let set = vertex_set(&rel, vertices + vertex_index * 52)?;
        let base = model.positions.len();
        if base + set.count > 1_000_000 {
            return Err(error(mesh_offset, "model exceeds one million vertices"));
        }
        let normal_base = model.normals.len();
        let uv_base = model.texcoords.len();
        for i in 0..set.count {
            let offset = set.data + i * set.stride;
            model.positions.push(rel.floats(offset + set.position)?);
            let mut influence = crate::rig::Influence::default();
            if let (Some(weights), Some(joints)) = (set.weights, set.joints) {
                influence.weights = rel.floats(offset + weights)?;
                let indices = rel.bytes(offset + joints, 4)?;
                let sum: f32 = influence.weights.iter().sum();
                if influence.weights.iter().any(|v| *v < 0.0) || !sum.is_finite() || sum <= 0.000001
                {
                    return Err(error(offset + weights, "invalid skin weights"));
                }
                for (j, index) in indices.iter().enumerate() {
                    influence.weights[j] /= sum;
                    if influence.weights[j] == 0.0 {
                        continue;
                    }
                    if *index as usize >= set.palette_count {
                        return Err(error(offset + joints, "skin joint exceeds bone palette"));
                    }
                    influence.joints[j] = u16::from_le_bytes(
                        rel.bytes(set.palette + *index as usize * 2, 2)?
                            .try_into()
                            .unwrap(),
                    );
                }
                has_skin = true;
            }
            skin.push(influence);
            if let Some(normal) = set.normal {
                model.normals.push(rel.floats(offset + normal)?);
            }
            if let Some(uv) = set.uv {
                let [u, v] = rel.floats(offset + uv)?;
                // Aqua uses image-space V; normalize to the OBJ-style V-up convention.
                model.texcoords.push([u, 1.0 - v]);
            }
        }
        let polygon = strips + strip_index * 24;
        let tag = rel.word(polygon)?;
        let count_offset = rel.word(polygon + 8)?;
        let count = rel.word(count_offset)?;
        let indices = rel.word(polygon + 16)?;
        if count > 6_000_000 || count != rel.word(polygon + 12)? {
            return Err(error(polygon, "invalid polygon index count"));
        }
        rel.table(indices, count, 2)?;
        let values = rel
            .bytes(indices, count * 2)?
            .chunks_exact(2)
            .map(|v| u16::from_le_bytes(v.try_into().unwrap()) as usize)
            .collect::<Vec<_>>();
        let is_strip = tag == 0x2100;
        if !is_strip && tag != 0x1000 {
            return Err(error(polygon, "unsupported polygon topology"));
        }
        if !is_strip && count % 3 != 0 {
            return Err(error(
                polygon,
                "triangle list count is not divisible by three",
            ));
        }
        let step = if is_strip { 1 } else { 3 };
        for i in (0..count.saturating_sub(2)).step_by(step) {
            let mut triangle = [values[i], values[i + 1], values[i + 2]];
            if triangle.iter().any(|v| *v >= set.count) {
                return Err(error(indices + i * 2, "polygon index exceeds vertex set"));
            }
            if triangle[0] == triangle[1]
                || triangle[0] == triangle[2]
                || triangle[1] == triangle[2]
            {
                continue;
            }
            if is_strip && i % 2 == 1 {
                triangle.swap(1, 2);
            }
            if model.triangles.len() >= 2_000_000 {
                return Err(error(polygon, "model exceeds two million triangles"));
            }
            model.triangles.push(triangle.map(|v| base + v));
            model
                .triangle_normals
                .push(triangle.map(|v| set.normal.map(|_| normal_base + v)));
            model
                .triangle_uvs
                .push(triangle.map(|v| set.uv.map(|_| uv_base + v)));
            model.triangle_materials.push(None);
        }
    }
    if model.triangles.is_empty() {
        return Err(error(root, "model contains no drawable triangles"));
    }
    Ok((model, has_skin.then_some(skin)))
}

#[cfg(test)]
pub(crate) fn fixture() -> Vec<u8> {
    fn put(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    let mut data = vec![0; 32 + 0x220];
    data[..4].copy_from_slice(b"NIFL");
    put(&mut data, 4, 24);
    put(&mut data, 8, 1);
    put(&mut data, 12, 32);
    let rel = &mut data[32..];
    rel[..4].copy_from_slice(b"REL0");
    put(rel, 4, 0x218);
    put(rel, 8, 0x10);
    for (offset, value) in [
        (0x10, 0xC2A),
        (0x14, 0xA4),
        (0x10 + 36, 1),
        (0x10 + 40, 0xC0),
        (0x10 + 44, 1),
        (0x10 + 48, 0x1A0),
        (0x10 + 52, 1),
        (0x10 + 56, 0x1E0),
        (0xC0, 32),
        (0xC4, 3),
        (0xC8, 0x100),
        (0xCC, 3),
        (0xD0, 0x140),
        (0x100, 0),
        (0x104, 3),
        (0x108, 0),
        (0x110, 2),
        (0x114, 3),
        (0x118, 12),
        (0x120, 0x10),
        (0x124, 2),
        (0x128, 24),
        (0x1A0, 0x1000),
        (0x1A8, 0x1C0),
        (0x1AC, 3),
        (0x1B0, 0x1D0),
        (0x1C0, 3),
    ] {
        put(rel, offset, value);
    }
    for (i, position) in [[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
        .iter()
        .enumerate()
    {
        for (j, value) in position.iter().enumerate() {
            put(rel, 0x140 + i * 32 + j * 4, value.to_bits());
        }
        put(rel, 0x140 + i * 32 + 20, 1.0_f32.to_bits());
    }
    rel[0x1D0..0x1D6].copy_from_slice(&[0, 0, 1, 0, 2, 0]);
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    fn put(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn resolves_per_mesh_skin_palettes_and_rejects_invalid_weights() {
        let mut data = fixture();
        data.resize(32 + 0x340, 0);
        let r = &mut data[32..];
        let original = r[0x140..0x1a0].to_vec();
        for (offset, value) in [
            (4, 0x338),
            (0xc0, 52),
            (0xc4, 5),
            (0xd0, 0x240),
            (0xd8, 2),
            (0xdc, 0x320),
            (0x130, 1),
            (0x134, 4),
            (0x138, 32),
            (0x140, 11),
            (0x144, 7),
            (0x148, 48),
        ] {
            put(r, offset, value);
        }
        r[0x320..0x324].copy_from_slice(&[3, 0, 7, 0]);
        for i in 0..3 {
            r[0x240 + i * 52..0x240 + i * 52 + 32].copy_from_slice(&original[i * 32..i * 32 + 32]);
            put(r, 0x240 + i * 52 + 32, 0.25_f32.to_bits());
            put(r, 0x240 + i * 52 + 36, 0.75_f32.to_bits());
            r[0x240 + i * 52 + 49] = 1;
        }
        let (model, skin) = parse(&data).unwrap();
        let skin = skin.unwrap();
        assert_eq!(model.positions.len(), 3);
        assert_eq!(skin[0].joints, [3, 7, 0, 0]);
        assert_eq!(skin[0].weights, [0.25, 0.75, 0.0, 0.0]);
        let mut bad = data.clone();
        bad[32 + 0x240 + 48] = 2;
        assert!(parse(&bad).is_err());
        put(&mut data, 32 + 0x240 + 32, f32::NAN.to_bits());
        assert!(parse(&data).is_err());
    }
    #[test]
    fn reads_classic_geometry_and_first_afp_model() {
        let data = fixture();
        let model = Model::from_aqua(&data).unwrap();
        assert_eq!(model.positions.len(), 3);
        assert_eq!(model.triangles, vec![[0, 1, 2]]);
        assert_eq!(model.triangle_normals, vec![[Some(0), Some(1), Some(2)]]);
        assert_eq!(model.texcoords.len(), 3);
        let mut package = vec![0; 64];
        package[..4].copy_from_slice(b"afp\0");
        put(&mut package, 4, 1);
        put(&mut package, 52, 48);
        put(&mut package, 56, (data.len() + 48) as u32);
        package.extend_from_slice(&data);
        assert_eq!(Model::from_aqua(&package).unwrap(), model);
    }

    #[test]
    fn triangle_strips_keep_parity_across_degenerate_faces() {
        let mut data = fixture();
        put(&mut data, 32 + 0x1A0, 0x2100);
        put(&mut data, 32 + 0x1AC, 5);
        put(&mut data, 32 + 0x1C0, 5);
        data[32 + 0x1D0..32 + 0x1DA].copy_from_slice(&[0, 0, 0, 0, 1, 0, 2, 0, 0, 0]);
        assert_eq!(
            Model::from_aqua(&data).unwrap().triangles,
            vec![[0, 2, 1], [1, 2, 0]]
        );
    }

    #[test]
    fn rejects_truncated_attributes_bad_indices_and_unsupported_variants() {
        let data = fixture();
        for length in 0..data.len() {
            assert!(Model::from_aqua(&data[..length]).is_err());
        }
        for (offset, value) in [
            (32 + 0x10, 0xC33),
            (32 + 0xC0, 0),
            (32 + 0x108, 30),
            (32 + 0x104, 0x13),
            (32 + 0x140, f32::NAN.to_bits()),
            (32 + 0x1E0 + 28, 1),
            (32 + 0x1D0, 9),
            (32 + 0x1C0, 999),
            (32 + 0xC8, u32::MAX),
        ] {
            let mut bad = fixture();
            put(&mut bad, offset, value);
            assert!(Model::from_aqua(&bad).is_err(), "offset {offset}");
        }
    }
}
