//! Original four-legged creature used when no external model is available.
use crate::{rig_geometry::box_part, scene::Scene};
use macroquad::prelude::*;
use psp_assets::{
    materials::MaterialLibrary,
    rig::{Bone, Rig, Skeleton, Transform},
    Model,
};

pub fn crawler() -> Scene {
    let mut bones: Vec<Bone> = Vec::new();
    let mut bone = |name: &str, parent: Option<usize>, position: Vec3, rotation: Quat| {
        let local_bind = Transform {
            translation: position,
            rotation,
            scale: Vec3::ONE,
        };
        let global = parent.map_or(local_bind.matrix(), |p| {
            bones[p].inverse_bind.inverse() * local_bind.matrix()
        });
        bones.push(Bone {
            name: name.to_owned(),
            parent,
            inverse_bind: global.inverse(),
            local_bind,
        });
        bones.len() - 1
    };
    bone("crawler_root", None, Vec3::ZERO, Quat::IDENTITY);
    bone("lumbar_efce", Some(0), vec3(0.0, 0.95, 0.0), Quat::IDENTITY);
    bone("spine", Some(1), vec3(0.3, 0.08, 0.0), Quat::IDENTITY);
    bone("head", Some(2), vec3(0.45, 0.06, 0.0), Quat::IDENTITY);
    bone("tail01", Some(1), vec3(-0.45, 0.02, 0.0), Quat::IDENTITY);
    bone(
        "drs-line_antennal_l",
        Some(3),
        vec3(0.12, 0.16, -0.15),
        Quat::from_rotation_y(0.3),
    );
    bone(
        "drs-line_antennal_r",
        Some(3),
        vec3(0.12, 0.16, 0.15),
        Quat::from_rotation_y(-0.3),
    );
    for (prefix, x, z, yaw) in [
        ("lf", 0.25, -0.22, 1.0),
        ("rf", 0.25, 0.22, -1.0),
        ("lr", -0.3, -0.22, 2.1),
        ("rr", -0.3, 0.22, -2.1),
    ] {
        let hip = bone(
            &format!("{prefix}_leg01"),
            Some(1),
            vec3(x, 0.0, z),
            Quat::from_rotation_y(yaw) * Quat::from_rotation_z(-0.2),
        );
        let knee = bone(
            &format!("{prefix}_leg02"),
            Some(hip),
            vec3(0.6, 0.0, 0.0),
            Quat::from_rotation_z(-0.9),
        );
        bone(
            &format!("{prefix}_leg03"),
            Some(knee),
            vec3(0.6, 0.0, 0.0),
            Quat::from_rotation_z(0.4),
        );
    }
    let skeleton = Skeleton { bones };
    let mut model = Model {
        material_names: vec!["shell".into(), "joint".into(), "core".into(), "edge".into()],
        ..Model::default()
    };
    let mut skin = Vec::new();
    let mut part = |b, center, size, material| {
        box_part(&mut model, &mut skin, &skeleton, b, center, size, material)
    };
    part(1, Vec3::ZERO, vec3(1.15, 0.5, 0.65), 0);
    part(1, vec3(0.0, 0.28, 0.0), vec3(0.7, 0.1, 0.5), 3);
    part(2, Vec3::ZERO, vec3(0.5, 0.38, 0.5), 0);
    part(2, vec3(0.0, 0.21, 0.0), vec3(0.28, 0.18, 0.3), 2);
    part(3, Vec3::ZERO, vec3(0.5, 0.35, 0.44), 0);
    for side in [-1.0, 1.0] {
        part(3, vec3(0.24, 0.08, side * 0.16), vec3(0.06, 0.08, 0.1), 2);
        part(3, vec3(0.33, -0.12, side * 0.16), vec3(0.35, 0.08, 0.07), 3);
    }
    part(4, vec3(-0.15, 0.03, 0.0), vec3(0.65, 0.36, 0.52), 0);
    for b in 5..7 {
        part(b, vec3(0.23, 0.07, 0.0), vec3(0.5, 0.06, 0.05), 3);
    }
    for hip in [7, 10, 13, 16] {
        part(hip, vec3(0.3, 0.0, 0.0), vec3(0.64, 0.16, 0.18), 0);
        part(hip + 1, vec3(0.3, 0.0, 0.0), vec3(0.64, 0.12, 0.13), 1);
        part(hip + 2, vec3(0.24, 0.0, 0.0), vec3(0.52, 0.07, 0.09), 3);
    }
    let materials=MaterialLibrary::from_mtl("newmtl shell\nKd 0.18 0.35 0.38\nnewmtl joint\nKd 0.08 0.12 0.16\nnewmtl core\nKd 1 0.18 0.08\nnewmtl edge\nKd 0.5 0.72 0.68").unwrap();
    let count = model.positions.len();
    let mut scene = Scene::from_model(model, None, &materials, Vec::new());
    scene.rig = Some(Rig::new(skeleton, skin, count).unwrap());
    scene
}
