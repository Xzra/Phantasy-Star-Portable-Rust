//! Solo world simulation. Input, collision, interaction ranges and attack cadence
//! live here so presentation cannot bypass the gameplay rules.
use crate::{Command, Event, Location, QuestState, Session};

pub const FIXED_STEP: f32 = 1.0 / 60.0;
pub const PLAYER_RADIUS: f32 = 0.38;
pub const ATTACK_RANGE: f32 = 2.6;
pub const TERMINAL: Position = Position { x: 0.0, z: -7.5 };
pub const ENEMY: Position = Position { x: 0.0, z: -4.0 };
pub const RETURN_PAD: Position = Position { x: 0.0, z: 9.0 };

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f32,
    pub z: f32,
}
impl Position {
    pub fn distance(self, other: Self) -> f32 {
        ((self.x - other.x).powi(2) + (self.z - other.z).powi(2)).sqrt()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Obstacle {
    pub center: Position,
    pub half: Position,
}
pub const LOBBY_OBSTACLES: [Obstacle; 4] = [
    Obstacle {
        center: Position { x: -6.0, z: -2.0 },
        half: Position { x: 0.8, z: 0.8 },
    },
    Obstacle {
        center: Position { x: 6.0, z: -2.0 },
        half: Position { x: 0.8, z: 0.8 },
    },
    Obstacle {
        center: Position { x: -6.0, z: 5.0 },
        half: Position { x: 1.8, z: 0.65 },
    },
    Obstacle {
        center: Position { x: 6.0, z: 5.0 },
        half: Position { x: 1.8, z: 0.65 },
    },
];
pub const FIELD_OBSTACLES: [Obstacle; 4] = [
    Obstacle {
        center: Position { x: -7.0, z: 0.0 },
        half: Position { x: 1.5, z: 1.5 },
    },
    Obstacle {
        center: Position { x: 7.0, z: 0.0 },
        half: Position { x: 1.5, z: 1.5 },
    },
    Obstacle {
        center: Position { x: -6.0, z: -9.0 },
        half: Position { x: 2.0, z: 0.8 },
    },
    Obstacle {
        center: Position { x: 6.0, z: -9.0 },
        half: Position { x: 2.0, z: 0.8 },
    },
];

#[derive(Debug, Default, Clone, Copy)]
pub struct Input {
    /// World-space X/Z intent. Normalized by the simulation, including diagonal input.
    pub movement: Position,
    pub sprint: bool,
    pub interact: bool,
    pub attack: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldEvent {
    Swing,
    Gameplay(Event),
    Message(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    session: Session,
    position: Position,
    facing: f32,
    attack_cooldown: f32,
    time: f32,
    distance_walked: f32,
}
impl Default for World {
    fn default() -> Self {
        Self {
            session: Session::default(),
            position: Position { x: 0.0, z: 5.5 },
            facing: std::f32::consts::PI,
            attack_cooldown: 0.0,
            time: 0.0,
            distance_walked: 0.0,
        }
    }
}

impl World {
    pub fn session(&self) -> &Session {
        &self.session
    }
    pub fn position(&self) -> Position {
        self.position
    }
    pub fn facing(&self) -> f32 {
        self.facing
    }
    pub fn time(&self) -> f32 {
        self.time
    }
    pub fn distance_walked(&self) -> f32 {
        self.distance_walked
    }
    pub fn attack_cooldown(&self) -> f32 {
        self.attack_cooldown
    }
    pub fn half_extent(&self) -> f32 {
        if self.session.location() == Location::Lobby {
            12.0
        } else {
            15.0
        }
    }
    pub fn obstacles(&self) -> &'static [Obstacle] {
        if self.session.location() == Location::Lobby {
            &LOBBY_OBSTACLES
        } else {
            &FIELD_OBSTACLES
        }
    }
    pub fn interaction_prompt(&self) -> Option<&'static str> {
        match self.session.location() {
            Location::Lobby if self.position.distance(TERMINAL) <= 2.2 => {
                Some(if self.session.quest() == QuestState::Available {
                    "E  /  Begin Hunter training"
                } else {
                    "Training complete - start a new session from the pause menu"
                })
            }
            Location::TrainingField if self.position.distance(RETURN_PAD) <= 2.2 => {
                Some(if self.session.quest() == QuestState::Complete {
                    "E  /  Return to the lobby"
                } else {
                    "Defeat the training projection to unlock the return pad"
                })
            }
            _ => None,
        }
    }
    fn free(&self, position: Position) -> bool {
        let edge = self.half_extent() - PLAYER_RADIUS;
        if position.x.abs() > edge || position.z.abs() > edge {
            return false;
        }
        if self.session.location() == Location::Lobby
            && position.distance(TERMINAL) < 0.8 + PLAYER_RADIUS
        {
            return false;
        }
        for obstacle in self.obstacles() {
            let nearest = Position {
                x: position.x.clamp(
                    obstacle.center.x - obstacle.half.x,
                    obstacle.center.x + obstacle.half.x,
                ),
                z: position.z.clamp(
                    obstacle.center.z - obstacle.half.z,
                    obstacle.center.z + obstacle.half.z,
                ),
            };
            if position.distance(nearest) < PLAYER_RADIUS {
                return false;
            }
        }
        if self.session.location() == Location::TrainingField
            && self.session.quest() == QuestState::Active
            && position.distance(ENEMY) < 1.1 + PLAYER_RADIUS
        {
            return false;
        }
        true
    }
    pub fn step(&mut self, input: Input, delta: f32) -> Vec<WorldEvent> {
        if !delta.is_finite()
            || delta <= 0.0
            || !input.movement.x.is_finite()
            || !input.movement.z.is_finite()
        {
            return Vec::new();
        }
        let delta = delta.min(0.1);
        self.time += delta;
        self.attack_cooldown = (self.attack_cooldown - delta).max(0.0);
        // Scale first so even extreme finite input cannot overflow its length.
        let scale = input.movement.x.abs().max(input.movement.z.abs()).max(1.0);
        let mut movement = Position {
            x: input.movement.x / scale,
            z: input.movement.z / scale,
        };
        let length = (movement.x * movement.x + movement.z * movement.z)
            .sqrt()
            .max(1.0);
        movement.x /= length;
        movement.z /= length;
        if movement.x != 0.0 || movement.z != 0.0 {
            self.facing = movement.x.atan2(movement.z);
        }
        let speed = if input.sprint { 6.5 } else { 4.0 };
        let before = self.position;
        let x = Position {
            x: self.position.x + movement.x * speed * delta,
            ..self.position
        };
        if self.free(x) {
            self.position = x;
        }
        let z = Position {
            z: self.position.z + movement.z * speed * delta,
            ..self.position
        };
        if self.free(z) {
            self.position = z;
        }
        self.distance_walked += before.distance(self.position);
        let mut events = Vec::new();
        if input.interact {
            let command = match self.session.location() {
                Location::Lobby if self.position.distance(TERMINAL) <= 2.2 => {
                    Some(Command::StartQuest)
                }
                Location::TrainingField if self.position.distance(RETURN_PAD) <= 2.2 => {
                    Some(Command::ReturnToLobby)
                }
                _ => None,
            };
            if let Some(command) = command {
                match self.session.execute(command) {
                    Ok(result) => {
                        events.extend(result.into_iter().map(WorldEvent::Gameplay));
                        self.position = if self.session.location() == Location::Lobby {
                            Position { x: 0.0, z: -4.0 }
                        } else {
                            Position { x: 0.0, z: 6.0 }
                        };
                        self.facing = std::f32::consts::PI;
                        self.attack_cooldown = 0.0;
                    }
                    Err(message) => events.push(WorldEvent::Message(message)),
                }
            }
        }
        if input.attack && self.attack_cooldown == 0.0 {
            self.attack_cooldown = 0.55;
            events.push(WorldEvent::Swing);
            if self.session.location() == Location::TrainingField
                && self.session.quest() == QuestState::Active
                && self.position.distance(ENEMY) <= ATTACK_RANGE
            {
                let dx = ENEMY.x - self.position.x;
                let dz = ENEMY.z - self.position.z;
                let distance = (dx * dx + dz * dz).sqrt();
                let alignment =
                    (self.facing.sin() * dx + self.facing.cos() * dz) / distance.max(0.001);
                if alignment >= 0.35 {
                    if let Ok(result) = self.session.execute(Command::Attack) {
                        events.extend(result.into_iter().map(WorldEvent::Gameplay));
                    }
                }
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagonal_speed_and_invalid_frames_cannot_bypass_movement_limits() {
        let mut straight = World::default();
        let mut diagonal = straight.clone();
        straight.step(
            Input {
                movement: Position { x: 1.0, z: 0.0 },
                ..Input::default()
            },
            FIXED_STEP,
        );
        diagonal.step(
            Input {
                movement: Position { x: 1.0, z: 1.0 },
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert!((straight.distance_walked() - diagonal.distance_walked()).abs() < 0.0001);
        let original = straight.clone();
        straight.step(
            Input {
                movement: Position {
                    x: f32::NAN,
                    z: 0.0,
                },
                ..Input::default()
            },
            FIXED_STEP,
        );
        straight.step(Input::default(), f32::INFINITY);
        assert_eq!(straight, original);
    }
    #[test]
    fn boundaries_and_obstacles_block_player_motion() {
        let mut world = World {
            position: Position { x: 11.5, z: 0.0 },
            ..World::default()
        };
        world.step(
            Input {
                movement: Position { x: 1.0, z: 0.0 },
                sprint: true,
                ..Input::default()
            },
            0.1,
        );
        assert!(world.position.x <= 12.0 - PLAYER_RADIUS);
        world.position = Position { x: -6.0, z: 0.0 };
        for _ in 0..60 {
            world.step(
                Input {
                    movement: Position { x: 0.0, z: -1.0 },
                    ..Input::default()
                },
                FIXED_STEP,
            );
        }
        assert!(world.position.z >= -0.82);
    }
    #[test]
    fn quest_requires_proximity_and_combat_requires_range_facing_and_cooldown() {
        let mut world = World::default();
        world.step(
            Input {
                interact: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert_eq!(world.session.location(), Location::Lobby);
        world.position = TERMINAL;
        world.step(
            Input {
                interact: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert_eq!(world.session.location(), Location::TrainingField);
        world.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert_eq!(world.session.enemy_health(), 36);
        world.position = Position { x: 0.0, z: -2.0 };
        world.facing = 0.0;
        world.attack_cooldown = 0.0;
        world.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert_eq!(world.session.enemy_health(), 36);
        world.facing = std::f32::consts::PI;
        world.attack_cooldown = 0.0;
        world.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        world.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert_eq!(world.session.enemy_health(), 24);
        for _ in 0..70 {
            world.step(
                Input {
                    attack: true,
                    ..Input::default()
                },
                FIXED_STEP,
            );
        }
        assert_eq!(world.session.quest(), QuestState::Complete);
        assert_eq!(world.session.hunter().level, 2);
        world.position = RETURN_PAD;
        world.step(
            Input {
                interact: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert_eq!(world.session.location(), Location::Lobby);
        assert_eq!(world.session.hunter().health, 120);
    }
}
