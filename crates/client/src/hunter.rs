//! Original rigid-weighted Hunter mesh and authored local-space keyframe clips.
use crate::{rig_geometry::box_part, scene::Scene};
use macroquad::prelude::*;
use psp_assets::{
    materials::MaterialLibrary,
    rig::{Bone, BoneTrack, Channel, Clip, Rig, Skeleton, Transform},
    Model,
};
use psp_core::world::World;

pub struct Hunter {
    pub scene: Scene,
    pub pose: Vec<Transform>,
    clips: Vec<Clip>,
    last_distance: f32,
    last_time: f32,
    gait: f32,
    movement_blend: f32,
    run_blend: f32,
    pub blade: [Vec3; 2],
}
const BONES: [(&str, Option<usize>, [f32; 3]); 17] = [
    ("hips", None, [0.0, 0.88, 0.0]),
    ("spine", Some(0), [0.0, 0.2, 0.0]),
    ("chest", Some(1), [0.0, 0.25, 0.0]),
    ("head", Some(2), [0.0, 0.31, 0.0]),
    ("left_shoulder", Some(2), [-0.4, 0.13, 0.0]),
    ("left_elbow", Some(4), [0.0, -0.32, 0.0]),
    ("left_hand", Some(5), [0.0, -0.28, 0.0]),
    ("right_shoulder", Some(2), [0.4, 0.13, 0.0]),
    ("right_elbow", Some(7), [0.0, -0.32, 0.0]),
    ("right_hand", Some(8), [0.0, -0.28, 0.0]),
    ("left_hip", Some(0), [-0.18, -0.05, 0.0]),
    ("left_knee", Some(10), [0.0, -0.38, 0.0]),
    ("left_foot", Some(11), [0.0, -0.36, 0.07]),
    ("right_hip", Some(0), [0.18, -0.05, 0.0]),
    ("right_knee", Some(13), [0.0, -0.38, 0.0]),
    ("right_foot", Some(14), [0.0, -0.36, 0.07]),
    ("photon_blade", Some(9), [0.0, -0.05, 0.12]),
];
fn make_skeleton() -> Skeleton {
    let mut bones: Vec<Bone> = Vec::new();
    for (name, parent, position) in BONES {
        let local_bind = Transform {
            translation: position.into(),
            rotation: Quat::IDENTITY,
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
    }
    Skeleton { bones }
}

fn authored_pose(skeleton: &Skeleton, state: usize, t: f32) -> Vec<Transform> {
    let mut pose = skeleton.bind_pose();
    let phase = t * std::f32::consts::TAU;
    let idle = (phase).sin();
    pose[0].translation.y += idle * 0.012;
    pose[1].rotation = Quat::from_rotation_x(idle * 0.018);
    pose[3].rotation = Quat::from_rotation_y(idle * 0.04);
    pose[4].rotation = Quat::from_rotation_z(-0.12);
    pose[7].rotation = Quat::from_rotation_z(0.12);
    pose[5].rotation = Quat::from_rotation_x(-0.12);
    pose[8].rotation = Quat::from_rotation_x(-0.22);
    if state == 1 || state == 2 {
        let run = state == 2;
        let swing = phase.sin() * if run { 0.95 } else { 0.48 };
        pose[0].translation.y += (phase * 2.0).cos() * if run { 0.045 } else { 0.022 };
        pose[1].rotation = Quat::from_rotation_x(if run { 0.18 } else { 0.05 })
            * Quat::from_rotation_y(phase.sin() * 0.08);
        pose[10].rotation = Quat::from_rotation_x(swing);
        pose[13].rotation = Quat::from_rotation_x(-swing);
        pose[11].rotation =
            Quat::from_rotation_x(-(-phase.sin()).max(0.0) * if run { 1.2 } else { 0.55 });
        pose[14].rotation =
            Quat::from_rotation_x(-(phase.sin()).max(0.0) * if run { 1.2 } else { 0.55 });
        pose[4].rotation = Quat::from_rotation_x(-swing * 0.7) * Quat::from_rotation_z(-0.12);
        pose[7].rotation = Quat::from_rotation_x(swing * 0.6) * Quat::from_rotation_z(0.12);
        pose[5].rotation = Quat::from_rotation_x(if run { -0.9 } else { -0.2 });
        pose[8].rotation = Quat::from_rotation_x(if run { -0.9 } else { -0.35 });
    }
    if state >= 3 {
        // Explicit wind-up, cut, follow-through, recovery keys.
        let (shoulder, twist, elbow) = if t < 0.25 {
            let k = t / 0.25;
            (-0.2 - 1.4 * k, -0.7 * k, -0.2 - 0.8 * k)
        } else if t < 0.55 {
            let k = (t - 0.25) / 0.3;
            (-1.6 + 2.8 * k, -0.7 + 1.5 * k, -1.0 + 0.85 * k)
        } else {
            let k = (t - 0.55) / 0.45;
            (1.2 * (1.0 - k), 0.8 * (1.0 - k), -0.15 - 0.07 * k)
        };
        pose[1].rotation = Quat::from_rotation_y(twist * 0.5);
        pose[2].rotation = Quat::from_rotation_y(twist * 0.5);
        pose[7].rotation = Quat::from_euler(EulerRot::YXZ, -0.45, shoulder, 0.35);
        pose[8].rotation = Quat::from_rotation_x(elbow);
        pose[4].rotation = Quat::from_rotation_x(-0.45) * Quat::from_rotation_z(-0.4);
        pose[5].rotation = Quat::from_rotation_x(-0.8);
    }
    if state == 4 {
        pose[1].rotation = pose[1].rotation.inverse();
        pose[2].rotation = pose[2].rotation.inverse();
        pose[7].rotation = Quat::from_rotation_y(0.7) * pose[7].rotation;
    }
    if state == 5 {
        let cut = if t < 0.4 {
            -2.5 * t / 0.4
        } else {
            -2.5 + 3.5 * (t - 0.4) / 0.6
        };
        pose[7].rotation = Quat::from_rotation_x(cut) * Quat::from_rotation_z(-0.25);
        pose[8].rotation = Quat::from_rotation_x(-0.3);
        pose[1].rotation = Quat::from_rotation_x(cut * 0.1);
    }
    if state == 6 {
        pose[1].rotation = Quat::from_rotation_y(t * std::f32::consts::TAU);
        pose[7].rotation = Quat::from_euler(EulerRot::XYZ, -1.3, 0.0, 0.5);
        pose[8].rotation = Quat::from_rotation_x(-0.2);
    }
    pose
}
fn make_clips(skeleton: &Skeleton) -> Vec<Clip> {
    [
        ("Hunter idle", 2.4, 0),
        ("Hunter walk", 0.8, 1),
        ("Hunter run", 0.52, 2),
        ("Hunter slash I", 0.5, 3),
        ("Hunter slash II", 0.52, 4),
        ("Hunter cleave III", 0.68, 5),
        ("Photon sweep", 0.85, 6),
    ]
    .into_iter()
    .map(|(name, duration, state)| {
        let samples: Vec<f32> = if state >= 3 {
            vec![0.0, 0.125, 0.25, 0.4, 0.55, 0.75, 1.0]
        } else {
            (0..=16).map(|i| i as f32 / 16.0).collect()
        };
        let poses: Vec<_> = samples
            .iter()
            .map(|t| authored_pose(skeleton, state, *t))
            .collect();
        let times: Vec<_> = samples.iter().map(|t| t * duration).collect();
        let tracks = (0..skeleton.bones.len())
            .map(|bone| BoneTrack {
                bone,
                translation: Some(Channel {
                    times: times.clone(),
                    values: poses
                        .iter()
                        .map(|p| p[bone].translation.extend(0.0).to_array())
                        .collect(),
                }),
                rotation: Some(Channel {
                    times: times.clone(),
                    values: poses.iter().map(|p| p[bone].rotation.to_array()).collect(),
                }),
                scale: None,
            })
            .collect();
        Clip {
            name: name.to_owned(),
            duration,
            tracks,
        }
    })
    .collect()
}
impl Default for Hunter {
    fn default() -> Self {
        let skeleton = make_skeleton();
        let mut model = Model {
            material_names: vec![
                "armor".into(),
                "joint".into(),
                "light".into(),
                "trim".into(),
            ],
            ..Model::default()
        };
        let mut skin = Vec::new();
        let mut part = |bone, center, size, material| {
            box_part(
                &mut model, &mut skin, &skeleton, bone, center, size, material,
            )
        };
        part(0, vec3(0.0, 0.03, 0.0), vec3(0.48, 0.2, 0.32), 1);
        part(1, vec3(0.0, 0.03, 0.0), vec3(0.4, 0.25, 0.3), 1);
        part(2, vec3(0.0, -0.02, 0.0), vec3(0.62, 0.38, 0.38), 0);
        part(2, vec3(0.0, 0.02, 0.205), vec3(0.22, 0.12, 0.04), 2);
        part(2, vec3(0.0, 0.04, -0.23), vec3(0.4, 0.3, 0.14), 3);
        part(3, vec3(0.0, 0.08, 0.0), vec3(0.36, 0.34, 0.34), 3);
        part(3, vec3(0.0, 0.1, 0.18), vec3(0.3, 0.1, 0.035), 2);
        for (shoulder, elbow, hand) in [(4, 5, 6), (7, 8, 9)] {
            part(shoulder, vec3(0.0, -0.02, 0.0), vec3(0.3, 0.22, 0.34), 0);
            part(shoulder, vec3(0.0, -0.19, 0.0), vec3(0.18, 0.28, 0.2), 1);
            part(elbow, Vec3::ZERO, Vec3::splat(0.18), 3);
            part(elbow, vec3(0.0, -0.16, 0.0), vec3(0.22, 0.24, 0.24), 0);
            part(hand, vec3(0.0, -0.02, 0.0), vec3(0.18, 0.16, 0.2), 1);
        }
        for (hip, knee, foot) in [(10, 11, 12), (13, 14, 15)] {
            part(hip, vec3(0.0, -0.2, 0.0), vec3(0.25, 0.34, 0.28), 0);
            part(knee, Vec3::ZERO, vec3(0.2, 0.16, 0.23), 3);
            part(knee, vec3(0.0, -0.18, 0.0), vec3(0.22, 0.28, 0.25), 1);
            part(foot, vec3(0.0, -0.01, 0.08), vec3(0.26, 0.14, 0.4), 3);
        }
        part(16, vec3(0.0, 0.0, 0.0), vec3(0.09, 0.24, 0.08), 1);
        part(16, vec3(0.0, 0.13, 0.0), vec3(0.28, 0.06, 0.1), 3);
        part(16, vec3(0.0, 0.67, 0.0), vec3(0.1, 1.0, 0.04), 2);
        let materials=MaterialLibrary::from_mtl("newmtl armor\nKd 0.12 0.55 0.68\nnewmtl joint\nKd 0.09 0.14 0.2\nnewmtl light\nKd 0.3 0.95 1\nnewmtl trim\nKd 0.6 0.72 0.8").unwrap();
        let clips = make_clips(&skeleton);
        let pose = skeleton.bind_pose();
        let vertices = model.positions.len();
        let mut scene = Scene::from_model(model, None, &materials, Vec::new());
        scene.rig = Some(Rig::new(skeleton, skin, vertices).unwrap());
        Self {
            scene,
            pose,
            clips,
            last_distance: 0.0,
            last_time: 0.0,
            gait: 0.0,
            movement_blend: 0.0,
            run_blend: 0.0,
            blade: [Vec3::ZERO; 2],
        }
    }
}
impl Hunter {
    pub fn update(&mut self, world: &World) {
        let delta = (world.time() - self.last_time).clamp(0.0, 0.1);
        let distance = (world.distance_walked() - self.last_distance).max(0.0);
        let speed = if delta > 0.0 { distance / delta } else { 0.0 };
        self.last_time = world.time();
        self.last_distance = world.distance_walked();
        self.gait += distance / 2.4;
        let blend = 1.0 - (-14.0 * delta).exp();
        self.movement_blend += (f32::from(speed > 0.1) - self.movement_blend) * blend;
        self.run_blend += (((speed - 4.0) / 2.5).clamp(0.0, 1.0) - self.run_blend) * blend;
        let skeleton = &self.scene.rig.as_ref().unwrap().skeleton;
        let idle = self.clips[0].sample(skeleton, world.time(), true);
        let walk = self.clips[1].sample(skeleton, self.gait * self.clips[1].duration, true);
        let run = self.clips[2].sample(skeleton, self.gait * self.clips[2].duration, true);
        self.pose = idle
            .iter()
            .zip(walk.iter().zip(&run))
            .map(|(idle, (walk, run))| {
                idle.blend(walk.blend(*run, self.run_blend), self.movement_blend)
            })
            .collect();
        if let Some(attack) = world.attack() {
            let index = if attack.photon {
                6
            } else {
                2 + attack.stage as usize
            };
            let pose = self.clips[index].sample(skeleton, attack.elapsed, false);
            let weight =
                (attack.progress() / 0.1).min(1.0) * ((1.0 - attack.progress()) / 0.15).min(1.0);
            for i in (1..=9).chain(std::iter::once(16)) {
                self.pose[i] = self.pose[i].blend(pose[i], weight);
            }
        }
        if world.height() > 0.0 {
            self.pose[10].rotation = Quat::from_rotation_x(-0.65);
            self.pose[11].rotation = Quat::from_rotation_x(-0.75);
            self.pose[13].rotation = Quat::from_rotation_x(0.2);
            self.pose[14].rotation = Quat::from_rotation_x(-1.1);
        }
        if world.dodging() {
            self.pose[1].rotation = Quat::from_rotation_x(0.45);
            self.pose[0].translation.y -= 0.12;
        }
        if world.hurt() {
            self.pose[1].rotation = Quat::from_rotation_x(-0.3);
        }
        if world.defeated() {
            self.pose[0].rotation = Quat::from_rotation_x(-1.4);
            self.pose[0].translation.y = 0.18;
        }

        let p = world.position();
        let transform = Mat4::from_rotation_translation(
            Quat::from_rotation_y(world.facing()),
            vec3(p.x, world.height(), p.z),
        );
        let globals = skeleton.globals(&self.pose);
        self.blade = [vec3(0.0, 0.2, 0.0), vec3(0.0, 1.18, 0.0)]
            .map(|v| transform.transform_point3(globals[16].transform_point3(v)));
        self.scene.set_model_transform(transform);
        self.scene.animate(&self.pose);
    }
    pub fn draw(&self, bones: bool) {
        self.scene.draw(false);
        if bones {
            self.scene.draw_skeleton(&self.pose);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use psp_core::world::{Input, Position, FIXED_STEP};
    #[test]
    fn beveled_armor_faces_have_outward_normals() {
        let skeleton = Skeleton {
            bones: vec![Bone {
                name: "fixture".into(),
                parent: None,
                inverse_bind: Mat4::IDENTITY,
                local_bind: Transform::from_matrix(Mat4::IDENTITY),
            }],
        };
        let mut model = Model::default();
        let mut skin = Vec::new();
        box_part(
            &mut model,
            &mut skin,
            &skeleton,
            0,
            Vec3::ZERO,
            Vec3::ONE,
            0,
        );
        for triangle in &model.triangles {
            let [a, b, c] = triangle.map(|i| Vec3::from_array(model.positions[i]));
            assert!((b - a).cross(c - a).dot((a + b + c) / 3.0) > 0.0);
        }
    }
    #[test]
    fn rigged_mesh_keeps_bind_data_and_changes_pose_without_accumulation() {
        let mut hunter = Hunter::default();
        let mut world = World::default();
        let source = hunter.scene.model.positions.clone();
        hunter.update(&world);
        let idle = hunter.scene.positions.clone();
        for _ in 0..16 {
            world.step(
                Input {
                    movement: Position { x: 0.0, z: -1.0 },
                    sprint: true,
                    ..Input::default()
                },
                FIXED_STEP,
            );
            hunter.update(&world);
        }
        assert!(hunter.run_blend > 0.8);
        assert!(hunter.pose[10].rotation.angle_between(Quat::IDENTITY) > 0.2);
        assert_ne!(hunter.scene.positions, idle);
        world.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        hunter.update(&world);
        let attack_pose = hunter.pose.clone();
        let attack_vertices = hunter.scene.positions.clone();
        hunter.update(&world); // No simulation time passes while paused.
        assert_eq!(hunter.scene.positions, attack_vertices);
        assert!(
            hunter.pose[7]
                .rotation
                .angle_between(attack_pose[7].rotation)
                < 1e-5
        );
        assert_eq!(hunter.scene.model.positions, source);
        assert!(hunter.scene.positions.iter().all(|p| p.is_finite()));
        let skeleton = &hunter.scene.rig.as_ref().unwrap().skeleton;
        let bind = skeleton.bind_pose();
        hunter.scene.set_model_transform(Mat4::IDENTITY);
        hunter.scene.animate(&bind);
        hunter.scene.animate(&bind);
        for (position, source) in hunter.scene.positions.iter().zip(source) {
            assert!(position.distance(source.into()) < 1e-5);
        }
        for (mesh, triangles) in hunter
            .scene
            .solid
            .iter()
            .zip(hunter.scene.model.triangles.chunks(1500))
        {
            for (vertex, index) in mesh.vertices.iter().zip(triangles.iter().flatten()) {
                assert_eq!(vertex.position, hunter.scene.positions[*index]);
            }
        }
    }
}
