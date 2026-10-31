//! Original beveled primitives attached to a skeletal bind pose.
use macroquad::prelude::*;
use psp_assets::{
    rig::{Influence, Skeleton},
    Model,
};

pub(crate) fn box_part(
    model: &mut Model,
    skin: &mut Vec<Influence>,
    skeleton: &Skeleton,
    bone: usize,
    center: Vec3,
    size: Vec3,
    material: usize,
) {
    let start = model.positions.len();
    let global = skeleton.bones[bone].inverse_bind.inverse();
    let outline = [
        [-1.0, -0.65],
        [-0.65, -1.0],
        [0.65, -1.0],
        [1.0, -0.65],
        [1.0, 0.65],
        [0.65, 1.0],
        [-0.65, 1.0],
        [-1.0, 0.65],
    ];
    for y in [-1.0, 1.0] {
        for [x, z] in outline {
            let v = vec3(x, y, z);
            model
                .positions
                .push(global.transform_point3(center + v * size * 0.5).to_array());
            skin.push(Influence {
                joints: [bone as u16, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            });
        }
    }
    let mut triangles = Vec::new();
    for i in 1..7 {
        triangles.push([0, i + 1, i]);
        triangles.push([8, 8 + i, 8 + i + 1]);
    }
    for i in 0..8 {
        let j = (i + 1) % 8;
        triangles.push([i, j, j + 8]);
        triangles.push([i, j + 8, i + 8]);
    }
    for t in triangles {
        model
            .triangles
            .push([start + t[0], start + t[2], start + t[1]]);
        model.triangle_uvs.push([None; 3]);
        model.triangle_normals.push([None; 3]);
        model.triangle_materials.push(Some(material));
    }
}
