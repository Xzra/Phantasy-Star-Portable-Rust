//! Monster presentation. Native clips and original procedural poses share skinning.
use crate::scene::Scene;
use macroquad::prelude::Quat;
use psp_assets::rig::{Clip, Skeleton, Transform};
use psp_core::world::{EnemyAction, World};
const BASIC_DEATH_DURATION: f32 = 1.25;

#[derive(Default)]
pub struct ProjectionMotion {
    pub clips: Vec<Clip>,
    hit_at: Option<f32>,
    death_at: Option<f32>,
    last_time: f32,
    pose: Option<Vec<Transform>>,
    last_position: Option<psp_core::world::Position>,
    travel: f32,
}
#[derive(Clone, Copy)]
struct Frame {
    action: EnemyAction,
    elapsed: f32,
    time: f32,
    travel: f32,
    moving: bool,
    hit_age: f32,
    death_age: Option<f32>,
    windup: f32,
}
impl ProjectionMotion {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self {
            clips,
            ..Self::default()
        }
    }
    pub fn reset(&mut self) {
        self.hit_at = None;
        self.death_at = None;
        self.pose = None;
        self.last_position = None;
        self.travel = 0.0;
    }
    pub fn hit(&mut self, time: f32) {
        self.hit_at = Some(time);
    }
    pub fn die(&mut self, time: f32) {
        self.death_at = Some(time);
    }
    fn clip(&self, suffix: &str) -> Option<&Clip> {
        self.clips.iter().find(|c| c.name.ends_with(suffix))
    }
    pub fn has_native_motions(&self) -> bool {
        [
            "10010_mov_f.aqm",
            "20010_atk_near.aqm",
            "30210_dmg_front.aqm",
            "31010_dead_a.aqm",
        ]
        .iter()
        .all(|name| self.clip(name).is_some())
    }
    pub fn status(&self, external: bool, rigged: bool) -> &'static str {
        if !external {
            "Original creature motions active"
        } else if !rigged {
            "Custom model / no supported skeleton"
        } else if self.has_native_motions() {
            "Full monster motion pack active"
        } else {
            "Basic motions / Play-Dagan.cmd"
        }
    }
    pub fn death_age(&self, time: f32) -> Option<f32> {
        self.death_at.map(|start| (time - start).max(0.0))
    }
    pub fn death_visible(&self, time: f32) -> bool {
        self.death_at.is_some_and(|start| {
            time - start
                < self
                    .clip("31010_dead_a.aqm")
                    .map_or(BASIC_DEATH_DURATION, |c| c.duration)
        })
    }
    fn sample(&self, skeleton: &Skeleton, frame: Frame) -> Vec<Transform> {
        let (suffix, elapsed, looping) = if let Some(age) = frame.death_age {
            ("31010_dead_a.aqm", age, false)
        } else if frame.hit_age < 0.2 || frame.action == EnemyAction::Stagger {
            ("30210_dmg_front.aqm", frame.hit_age.min(0.65), false)
        } else {
            match frame.action {
                EnemyAction::Approach if frame.moving => {
                    ("10010_mov_f.aqm", frame.travel / 2.8, true)
                }
                EnemyAction::Windup => (
                    "20010_atk_near.aqm",
                    frame.elapsed / frame.windup * 0.4,
                    false,
                ),
                EnemyAction::Strike => {
                    ("20010_atk_near.aqm", 0.4 + frame.elapsed / 0.2 * 0.2, false)
                }
                EnemyAction::Recover => (
                    "20010_atk_near.aqm",
                    0.6 + frame.elapsed / 0.85 * 0.4,
                    false,
                ),
                _ => ("00010_idle.aqm", frame.time, true),
            }
        };
        let Some(clip) = self.clip(suffix) else {
            return basic_pose(skeleton, frame);
        };
        let time = if looping || frame.death_age.is_some() || suffix == "30210_dmg_front.aqm" {
            elapsed
        } else {
            elapsed * clip.duration
        };
        let mut pose = clip.sample(skeleton, time, looping);
        // Collision and navigation stay authoritative in the world.
        for (bone, transform) in skeleton.bones.iter().zip(&mut pose) {
            if bone.parent.is_none() {
                transform.translation.x = bone.local_bind.translation.x;
                transform.translation.z = bone.local_bind.translation.z;
            }
        }
        pose
    }
    pub fn update(&mut self, scene: &mut Scene, world: &World, bones: bool) {
        let Some(rig) = &scene.rig else {
            return;
        };
        let time = world.time();
        let delta = (time - self.last_time).clamp(0.0, 0.1);
        self.last_time = time;
        let moved = self
            .last_position
            .map_or(0.0, |p| p.distance(world.enemy_position()));
        self.last_position = Some(world.enemy_position());
        self.travel += moved;
        let frame = Frame {
            action: world.enemy_action(),
            elapsed: world.enemy_elapsed(),
            time,
            travel: self.travel,
            moving: moved > 0.0001,
            hit_age: self.hit_at.map_or(f32::INFINITY, |t| time - t),
            death_age: self.death_at.map(|t| time - t),
            windup: world.enemy_windup_duration(),
        };
        let mut pose = self.sample(&rig.skeleton, frame);
        if let Some(previous) = &self.pose {
            let weight = 1.0 - (-28.0 * delta).exp();
            for (current, previous) in pose.iter_mut().zip(previous) {
                *current = previous.blend(*current, weight);
            }
        }
        scene.animate(&pose);
        if bones {
            scene.draw_skeleton(&pose);
        }
        self.pose = Some(pose);
    }
}

/// Original joint motion for the Dagan rig and the distributable crawler.
/// Always start at bind pose; animation never accumulates onto last frame's vertices.
fn basic_pose(skeleton: &Skeleton, frame: Frame) -> Vec<Transform> {
    let mut pose = skeleton.bind_pose();
    let windup = (frame.elapsed / frame.windup).clamp(0.0, 1.0);
    let strike = (frame.elapsed / 0.2).clamp(0.0, 1.0);
    let recovery = 1.0 - (frame.elapsed / 0.85).clamp(0.0, 1.0);
    let hit = if frame.hit_age < 0.4 {
        (frame.hit_age / 0.4 * std::f32::consts::PI).sin()
    } else {
        0.0
    };
    let death = frame
        .death_age
        .map(|t| (t / BASIC_DEATH_DURATION).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    for (i, bone) in skeleton.bones.iter().enumerate() {
        let name = bone.name.as_str();
        let moving = frame.action == EnemyAction::Approach && frame.moving;
        let phase = frame.travel * 5.5
            + if name.starts_with("rf_") || name.starts_with("lr_") {
                std::f32::consts::PI
            } else {
                0.0
            };
        let step = if moving {
            phase.sin()
        } else {
            (frame.time * 2.8).sin() * 0.06
        };
        let mut angle = 0.0;
        if name.contains("_leg") {
            angle = if name.ends_with("01") {
                step * 0.28
            } else if name.ends_with("02") {
                -step.max(0.0) * 0.5
            } else if name.ends_with("03") {
                step * 0.26
            } else {
                step * 0.12
            };
            if frame.action == EnemyAction::Windup {
                angle -= windup * if name.contains("f_leg") { 0.38 } else { 0.12 };
            }
            if frame.action == EnemyAction::Strike {
                angle += (1.0 - strike) * if name.contains("f_leg") { 0.65 } else { -0.2 };
            }
            if frame.action == EnemyAction::Recover {
                angle += recovery * 0.12;
            }
            angle += death * if name.ends_with("02") { 1.15 } else { -0.3 };
        }
        if name == "lumbar_efce" {
            pose[i].translation.y += (frame.time * 3.0).sin() * 0.025;
            if moving {
                pose[i].translation.y += (frame.travel * 11.0).sin().abs() * 0.045;
            }
            if frame.action == EnemyAction::Windup {
                pose[i].translation.y -= windup * 0.13;
            }
            if frame.action == EnemyAction::Strike {
                pose[i].translation.y += (1.0 - strike) * 0.12;
            }
            pose[i].translation.y -= death * 0.2;
        }
        if name == "spine" || name == "head" {
            angle += (frame.time * 2.5).sin() * 0.025 - hit * 0.3;
            angle += match frame.action {
                EnemyAction::Windup => -windup * 0.3,
                EnemyAction::Strike => (1.0 - strike) * 0.5,
                EnemyAction::Recover => recovery * 0.15,
                _ => 0.0,
            };
        }
        if name.contains("antennal") {
            angle += (frame.time * 4.0 + i as f32).sin() * 0.07;
        }
        pose[i].rotation *= Quat::from_rotation_z(angle);
        if bone.parent.is_none() {
            pose[i].translation.y += death * 0.35;
            pose[i].rotation *= Quat::from_rotation_x(death * 1.45);
            // A small recoil reads even on rigs without named spine bones.
            pose[i].rotation *= Quat::from_rotation_z(-hit * 0.06);
        }
    }
    pose
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::prelude::Vec3;
    use psp_assets::rig::skin_point;
    fn frame(action: EnemyAction, elapsed: f32) -> Frame {
        Frame {
            action,
            elapsed,
            time: elapsed,
            travel: elapsed * 2.8,
            moving: true,
            hit_age: f32::INFINITY,
            death_age: None,
            windup: 0.65,
        }
    }
    #[test]
    fn missing_motion_pack_moves_joints_and_vertices_for_each_action() {
        let scene = crate::monster::crawler();
        let rig = scene.rig.as_ref().unwrap();
        let animator = ProjectionMotion::default();
        let bind = rig.skeleton.palette(&rig.skeleton.bind_pose());
        for action in [
            EnemyAction::Approach,
            EnemyAction::Windup,
            EnemyAction::Strike,
            EnemyAction::Recover,
            EnemyAction::Stagger,
            EnemyAction::Dead,
        ] {
            let mut f = frame(action, 0.12);
            if action == EnemyAction::Stagger {
                f.hit_age = 0.15;
            }
            if action == EnemyAction::Dead {
                f.death_age = Some(0.8);
            }
            let palette = rig.skeleton.palette(&animator.sample(&rig.skeleton, f));
            let mut changed = 0;
            for (p, skin) in scene.model.positions.iter().zip(&rig.skin) {
                let source = Vec3::from_array(*p);
                let a = skin_point(source, skin, &palette);
                let b = skin_point(source, skin, &bind);
                assert!(a.is_finite());
                if a.distance(b) > 0.001 {
                    changed += 1;
                }
            }
            assert!(changed > 16, "{action:?} did not deform the creature");
        }
    }
    #[test]
    fn procedural_pose_is_repeatable_and_death_stays_visible_without_archive() {
        let scene = crate::monster::crawler();
        let skeleton = &scene.rig.as_ref().unwrap().skeleton;
        let mut animator = ProjectionMotion::default();
        let f = frame(EnemyAction::Approach, 0.24);
        let a = animator.sample(skeleton, f);
        let b = animator.sample(skeleton, f);
        for (a, b) in a.iter().zip(b) {
            assert_eq!(a.matrix(), b.matrix());
        }
        animator.die(4.0);
        assert!(animator.death_visible(4.8));
        assert!(!animator.death_visible(5.3));
        assert!(!animator.has_native_motions());
        animator.reset();
        assert!(!animator.death_visible(4.0));
    }
}
