//! Validated skeletal hierarchy, sampled local poses and linear blend skinning.
//! Matrices use column vectors; bind positions remain in source/model space.
use crate::aqua::{AquaError, Reader};
#[cfg(test)]
use glam::vec3;
use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Clone, Copy, Default)]
pub struct Influence {
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}
impl Transform {
    pub fn matrix(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }
    pub fn blend(self, other: Self, weight: f32) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, weight),
            rotation: self.rotation.slerp(other.rotation, weight),
            scale: self.scale.lerp(other.scale, weight),
        }
    }
}
#[derive(Debug)]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    pub inverse_bind: Mat4,
    pub local_bind: Transform,
}
#[derive(Debug)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}
fn fail(offset: usize, message: &'static str) -> AquaError {
    AquaError { offset, message }
}

/// Return only REL0 bytes; pointers may not escape the declared chunk.
fn rel0<'a>(bytes: &'a [u8], envelope: &[u8; 4]) -> Result<Reader<'a>, AquaError> {
    let bytes = if bytes.get(..4) == Some(envelope) {
        &bytes[4..]
    } else {
        bytes
    };
    let reader = Reader(bytes);
    if reader.bytes(0, 4)? != b"NIFL" || reader.word(12)? != 32 || reader.bytes(32, 4)? != b"REL0" {
        return Err(fail(0, "expected NIFL/REL0 skeletal data"));
    }
    let size = reader
        .word(36)?
        .checked_add(8)
        .ok_or_else(|| fail(36, "REL0 size overflow"))?;
    Ok(Reader(reader.bytes(32, size)?))
}
impl Skeleton {
    pub fn from_aqn(bytes: &[u8]) -> Result<Self, AquaError> {
        let r = rel0(bytes, b"aqn\0")?;
        let root = r.word(8)?;
        r.bytes(root, 32)?;
        let count = r.word(root)?;
        let table = r.word(root + 4)?;
        if count == 0 || count > 4096 {
            return Err(fail(root, "invalid skeleton bone count"));
        }
        r.table(table, count, 176)?;
        let mut bones: Vec<Bone> = Vec::with_capacity(count);
        for i in 0..count {
            let b = table + i * 176;
            let raw = r.word(b + 8)?;
            let parent = if raw == u32::MAX as usize {
                None
            } else {
                Some(raw)
            };
            if parent.is_some_and(|p| p >= i) {
                return Err(fail(
                    b + 8,
                    "bone parent must precede child; cycles unsupported",
                ));
            }
            let values = r.floats::<16>(b + 80)?;
            let inverse_bind = Mat4::from_cols_array(&values);
            if inverse_bind.determinant().abs() < 1e-8
                || !inverse_bind.inverse().is_finite()
                || inverse_bind.x_axis.w.abs() > 1e-5
                || inverse_bind.y_axis.w.abs() > 1e-5
                || inverse_bind.z_axis.w.abs() > 1e-5
                || (inverse_bind.w_axis.w - 1.0).abs() > 1e-5
            {
                return Err(fail(b + 80, "invalid inverse bind matrix"));
            }
            let global = inverse_bind.inverse();
            let local = parent.map_or(global, |p| bones[p].inverse_bind * global);
            let name = r.bytes(b + 144, 32)?;
            let name = std::str::from_utf8(name.split(|v| *v == 0).next().unwrap())
                .map_err(|_| fail(b + 144, "invalid bone name"))?
                .to_owned();
            if name.is_empty() || bones.iter().any(|b| b.name == name) {
                return Err(fail(b + 144, "empty or duplicate bone name"));
            }
            bones.push(Bone {
                name,
                parent,
                inverse_bind,
                local_bind: Transform::from_matrix(local),
            });
        }
        Ok(Self { bones })
    }
    pub fn bind_pose(&self) -> Vec<Transform> {
        self.bones.iter().map(|b| b.local_bind).collect()
    }
    pub fn globals(&self, pose: &[Transform]) -> Vec<Mat4> {
        assert_eq!(pose.len(), self.bones.len());
        let mut globals: Vec<Mat4> = Vec::with_capacity(pose.len());
        for (bone, local) in self.bones.iter().zip(pose) {
            let local = local.matrix();
            globals.push(bone.parent.map_or(local, |p| globals[p] * local));
        }
        globals
    }
    pub fn palette(&self, pose: &[Transform]) -> Vec<Mat4> {
        self.globals(pose)
            .iter()
            .zip(&self.bones)
            .map(|(global, bone)| *global * bone.inverse_bind)
            .collect()
    }
    pub fn validate_skin(&self, skin: &[Influence], vertices: usize) -> Result<(), AquaError> {
        if skin.len() != vertices {
            return Err(fail(0, "skin and geometry vertex counts differ"));
        }
        for influence in skin {
            let sum: f32 = influence.weights.iter().sum();
            if !sum.is_finite()
                || influence.weights.iter().any(|w| !w.is_finite() || *w < 0.0)
                || (sum != 0.0 && (sum - 1.0).abs() > 0.001)
            {
                return Err(fail(0, "skin weights must be zero or normalized"));
            }
            for (joint, weight) in influence.joints.iter().zip(influence.weights) {
                if weight > 0.0 && *joint as usize >= self.bones.len() {
                    return Err(fail(0, "skin references a missing skeleton bone"));
                }
            }
        }
        Ok(())
    }
}
/// Zero total weight explicitly denotes an unskinned vertex.
pub fn skin_point(point: Vec3, influence: &Influence, palette: &[Mat4]) -> Vec3 {
    if influence.weights.iter().all(|w| *w == 0.0) {
        return point;
    }
    let mut result = Vec3::ZERO;
    for (joint, weight) in influence.joints.iter().zip(influence.weights) {
        if weight > 0.0 {
            result += palette[*joint as usize].transform_point3(point) * weight;
        }
    }
    result
}

#[derive(Debug)]
pub struct Channel {
    pub times: Vec<f32>,
    pub values: Vec<[f32; 4]>,
}
impl Channel {
    fn sample(&self, time: f32, rotation: bool) -> [f32; 4] {
        let upper = self.times.partition_point(|t| *t <= time);
        if upper == 0 {
            return self.values[0];
        }
        if upper == self.values.len() {
            return *self.values.last().unwrap();
        }
        let lower = upper - 1;
        let blend = (time - self.times[lower]) / (self.times[upper] - self.times[lower]);
        if rotation {
            Quat::from_array(self.values[lower])
                .slerp(Quat::from_array(self.values[upper]), blend)
                .to_array()
        } else {
            std::array::from_fn(|i| {
                self.values[lower][i] + (self.values[upper][i] - self.values[lower][i]) * blend
            })
        }
    }
}
#[derive(Debug)]
pub struct BoneTrack {
    pub bone: usize,
    pub translation: Option<Channel>,
    pub rotation: Option<Channel>,
    pub scale: Option<Channel>,
}
#[derive(Debug)]
pub struct Clip {
    pub name: String,
    pub duration: f32,
    pub tracks: Vec<BoneTrack>,
}
impl Clip {
    /// Classic NIFL skeletal translation/quaternion/scale keys, mapped by bone name.
    pub fn from_aqm(name: String, bytes: &[u8], skeleton: &Skeleton) -> Result<Self, AquaError> {
        let r = rel0(bytes, b"aqm\0")?;
        let root = r.word(8)?;
        r.bytes(root, 64)?;
        if !matches!(r.word(root)?, 0x10002 | 0x10012) {
            return Err(fail(root, "unsupported motion variant"));
        }
        let end = r.word(root + 8)? as f32;
        let fps = r.floats::<1>(root + 12)?[0];
        if fps <= 0.0 || fps > 1000.0 || end <= 0.0 || end > 1_000_000.0 {
            return Err(fail(root, "invalid motion duration"));
        }
        let count = r.word(root + 20)?;
        let table = r.word(root + 24)?;
        if count > 4096 {
            return Err(fail(root, "too many motion tracks"));
        }
        r.table(table, count, 48)?;
        let mut tracks = Vec::new();
        let mut total = 0;
        for i in 0..count {
            let segment = table + i * 48;
            let raw_name = r.bytes(segment + 12, 32)?;
            let raw_name = raw_name.split(|v| *v == 0).next().unwrap();
            let Some(bone) = skeleton
                .bones
                .iter()
                .position(|b| b.name.as_bytes() == raw_name)
            else {
                continue;
            };
            if tracks.iter().any(|t: &BoneTrack| t.bone == bone) {
                return Err(fail(segment, "duplicate animation bone"));
            }
            let mut track = BoneTrack {
                bone,
                translation: None,
                rotation: None,
                scale: None,
            };
            let keys = r.word(segment + 4)?;
            let address = r.word(segment + 8)?;
            if keys > 64 {
                return Err(fail(segment, "too many animation channels"));
            }
            r.table(address, keys, 24)?;
            for k in 0..keys {
                let key = address + k * 24;
                let kind = r.word(key)?;
                let format = r.word(key + 4)?;
                if !(1..=3).contains(&kind) {
                    continue;
                }
                if !matches!((kind, format & !0x80), (1 | 3, 1) | (2, 3)) {
                    return Err(fail(key, "unsupported skeletal key format"));
                }
                let n = r.word(key + 12)?;
                total += n;
                if n == 0 || total > 2_000_000 {
                    return Err(fail(key, "invalid or excessive motion key count"));
                }
                let values_at = r.word(key + 16)?;
                let times_at = r.word(key + 20)?;
                r.table(values_at, n, 16)?;
                let mut values = Vec::with_capacity(n);
                let mut times = Vec::with_capacity(n);
                let time_width = if format & 0x80 != 0 { 4 } else { 2 };
                if n > 1 {
                    r.table(times_at, n, time_width)?;
                }
                for j in 0..n {
                    let mut v = r.floats::<4>(values_at + j * 16)?;
                    if kind == 2 {
                        let q = Quat::from_array(v);
                        if !q.length_squared().is_finite() || q.length_squared() < 1e-8 {
                            return Err(fail(key, "zero animation quaternion"));
                        }
                        v = q.normalize().to_array();
                    }
                    if kind == 1 && v[..3].iter().any(|v| v.abs() > 1_000_000.0)
                        || kind == 3 && v[..3].iter().any(|v| *v < 0.0001 || *v > 1000.0)
                    {
                        return Err(fail(key, "unsupported translation/scale magnitude"));
                    }
                    values.push(v);
                    let raw = if n == 1 {
                        0
                    } else if time_width == 4 {
                        r.word(times_at + j * 4)?
                    } else {
                        u16::from_le_bytes(r.bytes(times_at + j * 2, 2)?.try_into().unwrap())
                            as usize
                    };
                    let time = (raw >> 4) as f32 / fps;
                    if time > end / fps + 0.0001 || times.last().is_some_and(|t| time <= *t) {
                        return Err(fail(key, "motion times must increase within duration"));
                    }
                    times.push(time);
                }
                let channel = Channel { times, values };
                let target = match kind {
                    1 => &mut track.translation,
                    2 => &mut track.rotation,
                    _ => &mut track.scale,
                };
                if target.is_some() {
                    return Err(fail(key, "duplicate skeletal channel"));
                }
                *target = Some(channel);
            }
            tracks.push(track);
        }
        if tracks.is_empty() {
            return Err(fail(root, "motion has no matching skeleton bones"));
        }
        Ok(Self {
            name,
            duration: end / fps,
            tracks,
        })
    }
    pub fn sample(&self, skeleton: &Skeleton, time: f32, looping: bool) -> Vec<Transform> {
        let mut pose = skeleton.bind_pose();
        let time = if !time.is_finite() {
            0.0
        } else if looping {
            time.rem_euclid(self.duration)
        } else {
            time.clamp(0.0, self.duration)
        };
        for track in &self.tracks {
            let transform = &mut pose[track.bone];
            if let Some(c) = &track.translation {
                transform.translation = Vec3::from_slice(&c.sample(time, false)[..3]);
            }
            if let Some(c) = &track.rotation {
                transform.rotation = Quat::from_array(c.sample(time, true));
            }
            if let Some(c) = &track.scale {
                transform.scale = Vec3::from_slice(&c.sample(time, false)[..3]);
            }
        }
        pose
    }
}

#[derive(Debug)]
pub struct Rig {
    pub skeleton: Skeleton,
    pub skin: Vec<Influence>,
}
impl Rig {
    pub fn new(
        skeleton: Skeleton,
        skin: Vec<Influence>,
        vertices: usize,
    ) -> Result<Self, AquaError> {
        skeleton.validate_skin(&skin, vertices)?;
        Ok(Self { skeleton, skin })
    }
    pub fn load_clips(&self, data: &[u8]) -> Result<MotionLibrary, Box<dyn std::error::Error>> {
        let decoded = crate::ice_decode::decode_v4(data)?;
        let archive = crate::ice::IceArchive::parse(&decoded)?;
        let mut clips = Vec::new();
        let mut warnings = Vec::new();
        let mut total_keys = 0;
        for entry in archive
            .entries
            .iter()
            .filter(|e| e.name.to_ascii_lowercase().ends_with(".aqm"))
        {
            let r = rel0(entry.data, b"aqm\0")?;
            if !matches!(r.word(r.word(8)?)?, 0x10002 | 0x10012) {
                warnings.push(format!(
                    "Skipped unsupported motion variant: {}",
                    entry.name
                ));
                continue;
            }
            clips.push(
                Clip::from_aqm(entry.name.to_owned(), entry.data, &self.skeleton)
                    .map_err(|e| format!("{}: {e}", entry.name))?,
            );
            total_keys += clips
                .last()
                .unwrap()
                .tracks
                .iter()
                .map(|t| {
                    t.translation
                        .iter()
                        .chain(t.rotation.iter())
                        .chain(t.scale.iter())
                        .map(|c| c.values.len())
                        .sum::<usize>()
                })
                .sum::<usize>();
            if total_keys > 2_000_000 {
                return Err("motion library exceeds two million keys".into());
            }
        }
        if clips.is_empty() {
            return Err("archive contains no AQM motion clips".into());
        }
        Ok(MotionLibrary { clips, warnings })
    }
}

#[derive(Debug)]
pub struct MotionLibrary {
    pub clips: Vec<Clip>,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn put(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn floats(data: &mut [u8], offset: usize, values: &[f32]) {
        for (i, v) in values.iter().enumerate() {
            put(data, offset + i * 4, v.to_bits());
        }
    }
    fn nifl(size: usize) -> Vec<u8> {
        let mut bytes = vec![0; 32 + size];
        bytes[..4].copy_from_slice(b"NIFL");
        put(&mut bytes, 12, 32);
        bytes[32..36].copy_from_slice(b"REL0");
        put(&mut bytes, 36, (size - 8) as u32);
        put(&mut bytes, 40, 16);
        bytes
    }
    fn skeleton_bytes() -> Vec<u8> {
        let mut bytes = nifl(0x1a0);
        let r = &mut bytes[32..];
        put(r, 16, 2);
        put(r, 20, 48);
        for (i, (parent, position, name)) in [
            (u32::MAX, vec3(2.0, 0.0, 0.0), b"root".as_slice()),
            (0, vec3(2.0, 1.0, 0.0), b"child".as_slice()),
        ]
        .into_iter()
        .enumerate()
        {
            let b = 48 + i * 176;
            put(r, b + 8, parent);
            floats(
                r,
                b + 80,
                &Mat4::from_translation(position).inverse().to_cols_array(),
            );
            r[b + 144..b + 144 + name.len()].copy_from_slice(name);
        }
        bytes
    }
    fn motion_bytes(wide: bool) -> Vec<u8> {
        let mut bytes = nifl(0x120);
        let r = &mut bytes[32..];
        for (offset, value) in [
            (16, 0x10002),
            (24, 30),
            (28, 30_f32.to_bits()),
            (36, 1),
            (40, 80),
            (84, 2),
            (88, 128),
            (128, 1),
            (132, 1),
            (140, 2),
            (144, 192),
            (148, 256),
            (152, 2),
            (156, 3),
            (164, 2),
            (168, 224),
            (172, 264),
        ] {
            put(r, offset, value);
        }
        r[92..97].copy_from_slice(b"child");
        floats(r, 192, &[0.0, 1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
        floats(r, 224, &Quat::IDENTITY.to_array());
        floats(
            r,
            240,
            &Quat::from_rotation_z(std::f32::consts::FRAC_PI_2).to_array(),
        );
        for (format, at) in [(132, 256), (156, 264)] {
            if wide {
                let value = u32::from_le_bytes(r[format..format + 4].try_into().unwrap()) | 0x80;
                put(r, format, value);
                put(r, at, 1);
                put(r, at + 4, 30 * 16 + 2);
            } else {
                r[at..at + 2].copy_from_slice(&1_u16.to_le_bytes());
                r[at + 2..at + 4].copy_from_slice(&(30_u16 * 16 + 2).to_le_bytes());
            }
        }
        bytes
    }
    #[test]
    fn native_hierarchy_bind_pose_and_weighted_animation_use_source_space() {
        let skeleton = Skeleton::from_aqn(&skeleton_bytes()).unwrap();
        let influence = Influence {
            joints: [0, 1, 0, 0],
            weights: [0.5, 0.5, 0.0, 0.0],
        };
        skeleton.validate_skin(&[influence], 1).unwrap();
        let source = vec3(2.0, 2.0, 0.0);
        assert!(
            skin_point(source, &influence, &skeleton.palette(&skeleton.bind_pose()))
                .distance(source)
                < 1e-5
        );
        for wide in [false, true] {
            let clip =
                Clip::from_aqm("original fixture".into(), &motion_bytes(wide), &skeleton).unwrap();
            let middle = clip.sample(&skeleton, 0.5, false);
            assert!(middle[1].translation.distance(vec3(0.0, 1.5, 0.0)) < 1e-5);
            assert!(
                middle[1]
                    .rotation
                    .dot(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4))
                    .abs()
                    > 0.9999
            );
            let final_point = skin_point(
                source,
                &influence,
                &skeleton.palette(&clip.sample(&skeleton, 1.0, false)),
            );
            assert!(final_point.distance(vec3(1.5, 2.0, 0.0)) < 1e-5);
            assert!(
                clip.sample(&skeleton, 1.0, true)[1]
                    .translation
                    .distance(vec3(0.0, 1.0, 0.0))
                    < 1e-5
            );
            assert!(
                clip.sample(&skeleton, 99.0, false)[1]
                    .translation
                    .distance(vec3(0.0, 2.0, 0.0))
                    < 1e-5
            );
        }
    }
    #[test]
    fn malformed_skeletons_motions_and_skin_fail_without_panics() {
        let bytes = skeleton_bytes();
        let skeleton = Skeleton::from_aqn(&bytes).unwrap();
        for length in [0, 31, 45, bytes.len() - 1] {
            assert!(Skeleton::from_aqn(&bytes[..length]).is_err());
        }
        for (offset, value) in [
            (32 + 48 + 8, 1),
            (32 + 48 + 176 + 8, 1),
            (32 + 48 + 80, 0),
            (32 + 20, u32::MAX),
        ] {
            let mut bytes = bytes.clone();
            put(&mut bytes, offset, value);
            assert!(Skeleton::from_aqn(&bytes).is_err());
        }
        for (offset, value) in [
            (32 + 148, u32::MAX),
            (32 + 140, 2_000_001),
            (32 + 256, 0),
            (32 + 224, f32::NAN.to_bits()),
        ] {
            let mut motion = motion_bytes(false);
            put(&mut motion, offset, value);
            assert!(Clip::from_aqm("bad".into(), &motion, &skeleton).is_err());
        }
        let bad = Influence {
            joints: [9, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        };
        assert!(skeleton.validate_skin(&[bad], 1).is_err());
        let bad = Influence {
            weights: [f32::NAN, 0.0, 0.0, 0.0],
            ..Influence::default()
        };
        assert!(skeleton.validate_skin(&[bad], 1).is_err());
    }
    #[test]
    fn quaternion_interpolation_takes_shortest_path() {
        let q = Quat::from_rotation_y(0.4);
        let channel = Channel {
            times: vec![0.0, 1.0],
            values: vec![q.to_array(), (-q).to_array()],
        };
        assert!(Quat::from_array(channel.sample(0.5, true)).dot(q).abs() > 0.99999);
    }
}
