//! Validated project settings shared by the runtime and visual editor.
use crate::{
    weapon::WeaponClass,
    world::{Position, ENEMY, RETURN_PAD, SHOPKEEPER, SHOP_NPC, TERMINAL},
    Location,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponBalance {
    pub power: f32,
    pub reach: f32,
    pub speed: f32,
}
impl Default for WeaponBalance {
    fn default() -> Self {
        Self {
            power: 1.0,
            reach: 1.0,
            speed: 1.0,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct GameSettings {
    pub walk_speed: f32,
    pub sprint_speed: f32,
    pub jump_speed: f32,
    pub gravity: f32,
    pub pp_recovery: f32,
    pub enemy_speed: f32,
    pub enemy_damage: f32,
    pub animation_energy: f32,
    pub sword: WeaponBalance,
    pub partisan: WeaponBalance,
    pub knuckles: WeaponBalance,
}
impl Default for GameSettings {
    fn default() -> Self {
        Self {
            walk_speed: 4.0,
            sprint_speed: 6.5,
            jump_speed: 7.0,
            gravity: 20.0,
            pp_recovery: 5.0,
            enemy_speed: 2.8,
            enemy_damage: 1.0,
            animation_energy: 1.0,
            sword: WeaponBalance::default(),
            partisan: WeaponBalance::default(),
            knuckles: WeaponBalance::default(),
        }
    }
}
impl GameSettings {
    pub fn weapon(&self, class: WeaponClass) -> WeaponBalance {
        match class {
            WeaponClass::Sword => self.sword,
            WeaponClass::Partisan => self.partisan,
            WeaponClass::Knuckles => self.knuckles,
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        for (v, min, max) in [
            (self.walk_speed, 1.0, 8.0),
            (self.sprint_speed, 2.0, 12.0),
            (self.jump_speed, 3.0, 12.0),
            (self.gravity, 8.0, 35.0),
            (self.pp_recovery, 0.0, 20.0),
            (self.enemy_speed, 0.5, 6.0),
            (self.enemy_damage, 0.25, 3.0),
            (self.animation_energy, 0.6, 1.4),
        ] {
            if !v.is_finite() || !(min..=max).contains(&v) {
                return Err("Project setting is outside its supported range.");
            }
        }
        if self.sprint_speed < self.walk_speed {
            return Err("Sprint speed must be at least walk speed.");
        }
        for weapon in [self.sword, self.partisan, self.knuckles] {
            for v in [weapon.power, weapon.reach, weapon.speed] {
                if !v.is_finite() || !(0.5..=2.0).contains(&v) {
                    return Err("Weapon multipliers must be between 0.5 and 2.0.");
                }
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct SceneProp {
    pub location: Location,
    pub center: Position,
    pub size: [f32; 3],
    pub color: [u8; 3],
}
impl SceneProp {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.center.x.is_finite()
            || !self.center.z.is_finite()
            || self
                .size
                .iter()
                .any(|v| !v.is_finite() || !(0.3..=4.0).contains(v))
        {
            return Err("Prop dimensions must be finite and between 0.3 and 4 units.");
        }
        let bound = if self.location == Location::Lobby {
            11.0
        } else {
            14.0
        };
        if self.center.x.abs() + self.size[0] * 0.5 > bound
            || self.center.z.abs() + self.size[2] * 0.5 > bound
        {
            return Err("Prop must fit inside the scene boundary.");
        }
        let protected = if self.location == Location::Lobby {
            vec![
                (TERMINAL, 2.0),
                (SHOPKEEPER, 2.0),
                (SHOP_NPC, 1.4),
                (Position { x: 0.0, z: 5.5 }, 2.0),
            ]
        } else {
            vec![
                (ENEMY, 3.0),
                (RETURN_PAD, 2.0),
                (Position { x: 0.0, z: 6.0 }, 2.0),
            ]
        };
        for (p, radius) in protected {
            if (self.center.x - p.x).abs() < self.size[0] * 0.5 + radius
                && (self.center.z - p.z).abs() < self.size[2] * 0.5 + radius
            {
                return Err(
                    "Prop overlaps a protected spawn, mission terminal, return pad or shop.",
                );
            }
        }
        Ok(())
    }
}
