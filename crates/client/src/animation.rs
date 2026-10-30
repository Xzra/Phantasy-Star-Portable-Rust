//! Presentation-only native clip selection. Damage and quest rules remain in core.
use crate::scene::Scene;
use psp_assets::rig::{Clip, Transform};

#[derive(Default)]
pub struct ProjectionMotion {
    pub clips: Vec<Clip>,
    hit_at: Option<f32>,
    death_at: Option<f32>,
    last_time: f32,
    pose: Option<Vec<Transform>>,
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
    pub fn death_visible(&self, time: f32) -> bool {
        self.death_at.is_some_and(|start| {
            self.clip("31010_dead_a.aqm")
                .is_some_and(|clip| time - start < clip.duration)
        })
    }
    pub fn update(&mut self, scene: &mut Scene, time: f32, bones: bool) {
        let Some(rig) = &scene.rig else {
            return;
        };
        let delta = (time - self.last_time).clamp(0.0, 0.1);
        self.last_time = time;
        let hit = self.hit_at.map(|t| time - t).unwrap_or(f32::INFINITY);
        let (suffix, elapsed, looping) = if let Some(start) = self.death_at {
            ("31010_dead_a.aqm", time - start, false)
        } else if hit < 0.18 {
            ("30210_dmg_front.aqm", hit / 0.18, false)
        } else if hit < 0.5 {
            ("20010_atk_near.aqm", (hit - 0.18) / 0.32, false)
        } else {
            ("00010_idle.aqm", time, true)
        };
        let clip = self.clip(suffix).or_else(|| self.clip("00010_idle.aqm"));
        let mut pose = clip
            .map(|clip| {
                clip.sample(
                    &rig.skeleton,
                    if looping || self.death_at.is_some() {
                        elapsed
                    } else {
                        elapsed * clip.duration
                    },
                    looping,
                )
            })
            .unwrap_or_else(|| rig.skeleton.bind_pose());
        // The stationary training target must stay at its authoritative location.
        for (bone, transform) in rig.skeleton.bones.iter().zip(&mut pose) {
            if bone.parent.is_none() {
                transform.translation.x = bone.local_bind.translation.x;
                transform.translation.z = bone.local_bind.translation.z;
            }
        }
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
