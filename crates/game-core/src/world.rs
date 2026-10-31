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
    pub movement: Position,
    pub sprint: bool,
    pub interact: bool,
    pub attack: bool,
    /// Edge preserved by the client until the next fixed tick.
    pub attack_pressed: bool,
    pub dodge: bool,
    pub jump: bool,
    pub lock_on: bool,
    pub photon_art: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyAction {
    Approach,
    Windup,
    Strike,
    Recover,
    Stagger,
    Dead,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attack {
    pub stage: u8,
    pub elapsed: f32,
    pub duration: f32,
    pub perfect: bool,
    pub photon: bool,
    hit: bool,
}
impl Attack {
    pub fn progress(self) -> f32 {
        self.elapsed / self.duration
    }
    pub fn timing_window(self) -> bool {
        !self.photon && (0.64..=0.88).contains(&self.progress())
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum WorldEvent {
    Swing,
    Step,
    Jump,
    PerfectAttack,
    EnemyWindup,
    EnemyStrike,
    Evaded,
    Gameplay(Event),
    Message(&'static str),
}
#[derive(Debug, Clone, PartialEq)]
pub struct World {
    session: Session,
    position: Position,
    facing: f32,
    time: f32,
    distance_walked: f32,
    enemy_position: Position,
    enemy_facing: f32,
    enemy_action: EnemyAction,
    enemy_elapsed: f32,
    strike_direction: Position,
    strike_hit: bool,
    attack: Option<Attack>,
    combo_stage: u8,
    combo_window: f32,
    buffered: bool,
    buffered_perfect: bool,
    buffered_photon: bool,
    queued_dodge: bool,
    queued_jump: bool,
    last_attack: bool,
    locked: bool,
    pp: f32,
    dodge_remaining: f32,
    dodge_cooldown: f32,
    dodge_direction: Position,
    height: f32,
    vertical_speed: f32,
    hurt_remaining: f32,
    invulnerable: f32,
    hit_stop: f32,
}
impl Default for World {
    fn default() -> Self {
        Self {
            session: Session::default(),
            position: Position { x: 0.0, z: 5.5 },
            facing: std::f32::consts::PI,
            time: 0.0,
            distance_walked: 0.0,
            enemy_position: ENEMY,
            enemy_facing: 0.0,
            enemy_action: EnemyAction::Approach,
            enemy_elapsed: 0.0,
            strike_direction: Position::default(),
            strike_hit: false,
            attack: None,
            combo_stage: 0,
            combo_window: 0.0,
            buffered: false,
            buffered_perfect: false,
            buffered_photon: false,
            queued_dodge: false,
            queued_jump: false,
            last_attack: false,
            locked: false,
            pp: 100.0,
            dodge_remaining: 0.0,
            dodge_cooldown: 0.0,
            dodge_direction: Position::default(),
            height: 0.0,
            vertical_speed: 0.0,
            hurt_remaining: 0.0,
            invulnerable: 0.0,
            hit_stop: 0.0,
        }
    }
}
fn direction(from: Position, to: Position) -> Position {
    let d = from.distance(to).max(0.001);
    Position {
        x: (to.x - from.x) / d,
        z: (to.z - from.z) / d,
    }
}
fn normalized(input: Position) -> Position {
    let scale = input.x.abs().max(input.z.abs()).max(1.0);
    let x = input.x / scale;
    let z = input.z / scale;
    let length = (x * x + z * z).sqrt().max(1.0);
    Position {
        x: x / length,
        z: z / length,
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
    pub fn attack(&self) -> Option<Attack> {
        self.attack
    }
    pub fn attack_cooldown(&self) -> f32 {
        self.attack.map_or(0.0, |a| a.duration - a.elapsed)
    }
    pub fn enemy_position(&self) -> Position {
        self.enemy_position
    }
    pub fn enemy_facing(&self) -> f32 {
        self.enemy_facing
    }
    pub fn enemy_action(&self) -> EnemyAction {
        self.enemy_action
    }
    pub fn enemy_elapsed(&self) -> f32 {
        self.enemy_elapsed
    }
    pub fn locked_on(&self) -> bool {
        self.locked
    }
    pub fn pp(&self) -> f32 {
        self.pp
    }
    pub fn height(&self) -> f32 {
        self.height
    }
    pub fn dodging(&self) -> bool {
        self.dodge_remaining > 0.0
    }
    pub fn dodge_ready(&self) -> bool {
        self.dodge_cooldown == 0.0 && self.height == 0.0
    }
    pub fn hurt(&self) -> bool {
        self.hurt_remaining > 0.0
    }
    pub fn defeated(&self) -> bool {
        self.session.hunter().health == 0
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
        if self.defeated() {
            return Some("E / Retry the encounter");
        }
        match self.session.location() {
            Location::Lobby if self.position.distance(TERMINAL) <= 2.2 => {
                Some(if self.session.quest() == QuestState::Available {
                    "E / Begin Hunter combat trial"
                } else {
                    "Trial complete / Esc > New session"
                })
            }
            Location::TrainingField if self.position.distance(RETURN_PAD) <= 2.2 => {
                Some(if self.session.quest() == QuestState::Complete {
                    "E / Return to the lobby"
                } else {
                    "Defeat the enemy to unlock the return pad"
                })
            }
            _ => None,
        }
    }
    fn terrain_free(&self, position: Position, radius: f32) -> bool {
        let edge = self.half_extent() - radius;
        if position.x.abs() > edge || position.z.abs() > edge {
            return false;
        }
        if self.session.location() == Location::Lobby && position.distance(TERMINAL) < 0.8 + radius
        {
            return false;
        }
        self.obstacles().iter().all(|b| {
            let nearest = Position {
                x: position
                    .x
                    .clamp(b.center.x - b.half.x, b.center.x + b.half.x),
                z: position
                    .z
                    .clamp(b.center.z - b.half.z, b.center.z + b.half.z),
            };
            position.distance(nearest) >= radius
        })
    }
    fn free(&self, position: Position) -> bool {
        self.terrain_free(position, PLAYER_RADIUS)
            && (self.session.location() != Location::TrainingField
                || self.session.quest() != QuestState::Active
                || position.distance(self.enemy_position) >= 1.1 + PLAYER_RADIUS)
    }
    fn move_player(&mut self, movement: Position, speed: f32, delta: f32) {
        let before = self.position;
        // Substeps keep dodge/lunge movement from crossing thin colliders.
        let steps = (speed * delta / 0.15).ceil().max(1.0) as usize;
        for _ in 0..steps {
            let x = Position {
                x: self.position.x + movement.x * speed * delta / steps as f32,
                ..self.position
            };
            if self.free(x) {
                self.position = x;
            }
            let z = Position {
                z: self.position.z + movement.z * speed * delta / steps as f32,
                ..self.position
            };
            if self.free(z) {
                self.position = z;
            }
        }
        self.distance_walked += before.distance(self.position);
    }
    fn move_enemy(&mut self, movement: Position, speed: f32, delta: f32) {
        let steps = (speed * delta / 0.15).ceil().max(1.0) as usize;
        for _ in 0..steps {
            let x = Position {
                x: self.enemy_position.x + movement.x * speed * delta / steps as f32,
                ..self.enemy_position
            };
            if self.terrain_free(x, 0.9) && x.distance(self.position) >= 1.4 {
                self.enemy_position = x;
            }
            let z = Position {
                z: self.enemy_position.z + movement.z * speed * delta / steps as f32,
                ..self.enemy_position
            };
            if self.terrain_free(z, 0.9) && z.distance(self.position) >= 1.4 {
                self.enemy_position = z;
            }
        }
    }
    fn reset_encounter(&mut self) {
        self.position = Position { x: 0.0, z: 6.0 };
        self.facing = std::f32::consts::PI;
        self.enemy_position = ENEMY;
        self.enemy_action = EnemyAction::Approach;
        self.enemy_elapsed = 0.0;
        self.attack = None;
        self.combo_stage = 0;
        self.combo_window = 0.0;
        self.buffered = false;
        self.buffered_perfect = false;
        self.buffered_photon = false;
        self.queued_dodge = false;
        self.queued_jump = false;
        self.locked = false;
        self.pp = 100.0;
        self.dodge_remaining = 0.0;
        self.dodge_cooldown = 0.0;
        self.height = 0.0;
        self.vertical_speed = 0.0;
        self.hurt_remaining = 0.0;
        self.invulnerable = 0.0;
        self.hit_stop = 0.0;
    }
    fn begin_attack(&mut self, photon: bool, perfect: bool, events: &mut Vec<WorldEvent>) {
        self.combo_stage = if photon {
            0
        } else if self.combo_window > 0.0 {
            self.combo_stage % 3 + 1
        } else {
            1
        };
        let duration = if photon {
            0.85
        } else {
            [0.5, 0.52, 0.68][self.combo_stage as usize - 1]
        };
        self.attack = Some(Attack {
            stage: self.combo_stage,
            elapsed: 0.0,
            duration,
            perfect,
            photon,
            hit: false,
        });
        if photon {
            self.pp -= 35.0;
        }
        self.combo_window = 0.0;
        self.buffered = false;
        self.buffered_perfect = false;
        self.buffered_photon = false;
        events.push(WorldEvent::Swing);
        if perfect {
            events.push(WorldEvent::PerfectAttack);
        }
    }
    fn enemy_tick(&mut self, delta: f32, events: &mut Vec<WorldEvent>) {
        if self.session.location() != Location::TrainingField
            || self.session.quest() != QuestState::Active
            || self.defeated()
        {
            return;
        }
        self.enemy_elapsed += delta;
        match self.enemy_action {
            EnemyAction::Approach => {
                let d = direction(self.enemy_position, self.position);
                self.enemy_facing = d.x.atan2(d.z);
                if self.enemy_position.distance(self.position) <= 3.3 {
                    self.enemy_action = EnemyAction::Windup;
                    self.enemy_elapsed = 0.0;
                    self.strike_direction = d;
                    events.push(WorldEvent::EnemyWindup);
                } else {
                    self.move_enemy(d, 2.8, delta);
                }
            }
            EnemyAction::Windup if self.enemy_elapsed >= 0.65 => {
                self.enemy_action = EnemyAction::Strike;
                self.enemy_elapsed = 0.0;
                self.strike_hit = false;
                events.push(WorldEvent::EnemyStrike);
            }
            EnemyAction::Strike => {
                self.move_enemy(self.strike_direction, 8.5, delta);
                if !self.strike_hit && self.enemy_elapsed >= 0.08 {
                    self.strike_hit = true;
                    let to = direction(self.enemy_position, self.position);
                    let in_arc =
                        to.x * self.strike_direction.x + to.z * self.strike_direction.z > 0.5;
                    if self.enemy_position.distance(self.position) <= 2.6
                        && in_arc
                        && self.height < 1.0
                    {
                        if self.dodge_remaining > 0.04 {
                            events.push(WorldEvent::Evaded);
                        } else if self.invulnerable == 0.0 {
                            events.extend(
                                self.session
                                    .damage_hunter(18)
                                    .into_iter()
                                    .map(WorldEvent::Gameplay),
                            );
                            self.hurt_remaining = 0.2;
                            self.invulnerable = 0.6;
                            self.attack = None;
                            self.combo_window = 0.0;
                            self.buffered = false;
                            self.buffered_perfect = false;
                            self.buffered_photon = false;
                            self.move_player(self.strike_direction, 3.0, 0.1);
                        }
                    }
                }
                if self.enemy_elapsed >= 0.2 {
                    self.enemy_action = EnemyAction::Recover;
                    self.enemy_elapsed = 0.0;
                }
            }
            EnemyAction::Recover if self.enemy_elapsed >= 0.85 => {
                self.enemy_action = EnemyAction::Approach;
                self.enemy_elapsed = 0.0;
            }
            EnemyAction::Stagger if self.enemy_elapsed >= 0.4 => {
                self.enemy_action = EnemyAction::Recover;
                self.enemy_elapsed = 0.4;
            }
            _ => {}
        }
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
        let press = input.attack_pressed || input.attack && !self.last_attack;
        self.last_attack = input.attack;
        let mut events = Vec::new();
        if input.interact {
            let command = if self.defeated() {
                Some(Command::RetryQuest)
            } else {
                match self.session.location() {
                    Location::Lobby if self.position.distance(TERMINAL) <= 2.2 => {
                        Some(Command::StartQuest)
                    }
                    Location::TrainingField if self.position.distance(RETURN_PAD) <= 2.2 => {
                        Some(Command::ReturnToLobby)
                    }
                    _ => None,
                }
            };
            if let Some(command) = command {
                match self.session.execute(command) {
                    Ok(result) => {
                        events.extend(result.into_iter().map(WorldEvent::Gameplay));
                        self.reset_encounter();
                        if self.session.location() == Location::Lobby {
                            self.position = Position { x: 0.0, z: -4.0 };
                        }
                        return events;
                    }
                    Err(message) => events.push(WorldEvent::Message(message)),
                }
            }
        }
        if self.defeated() {
            self.locked = false;
            return events;
        }
        if input.lock_on
            && self.session.location() == Location::TrainingField
            && self.session.quest() == QuestState::Active
        {
            self.locked = !self.locked;
        }
        if self.session.quest() != QuestState::Active
            || self.position.distance(self.enemy_position) > 22.0
        {
            self.locked = false;
        }
        self.dodge_cooldown = (self.dodge_cooldown - delta).max(0.0);
        self.invulnerable = (self.invulnerable - delta).max(0.0);
        self.hurt_remaining = (self.hurt_remaining - delta).max(0.0);
        self.queued_dodge |= input.dodge;
        self.queued_jump |= input.jump;
        if input.photon_art && self.pp >= 35.0 {
            self.buffered_photon = true;
        }
        if self.hit_stop > 0.0 {
            if press {
                self.buffered = true;
                self.buffered_perfect |= self.attack.is_some_and(|a| a.timing_window());
            }
            self.hit_stop = (self.hit_stop - delta).max(0.0);
            return events;
        }
        self.combo_window = (self.combo_window - delta).max(0.0);
        if self.attack.is_none() {
            self.pp = (self.pp + 5.0 * delta).min(100.0);
        }
        let movement = normalized(input.movement);
        if self.locked {
            let d = direction(self.position, self.enemy_position);
            self.facing = d.x.atan2(d.z);
        } else if !self.dodging()
            && self.attack.is_none()
            && (movement.x != 0.0 || movement.z != 0.0)
        {
            self.facing = movement.x.atan2(movement.z);
        }
        let jump = std::mem::take(&mut self.queued_jump);
        let dodge = std::mem::take(&mut self.queued_dodge);
        if jump && self.height == 0.0 && !self.dodging() && self.hurt_remaining == 0.0 {
            self.vertical_speed = 7.0;
            events.push(WorldEvent::Jump);
        }
        self.vertical_speed -= 20.0 * delta;
        self.height = (self.height + self.vertical_speed * delta).max(0.0);
        if self.height == 0.0 {
            self.vertical_speed = 0.0;
        }
        if dodge && self.dodge_ready() && self.hurt_remaining == 0.0 {
            self.dodge_direction = if movement.x != 0.0 || movement.z != 0.0 {
                movement
            } else {
                Position {
                    x: -self.facing.sin(),
                    z: -self.facing.cos(),
                }
            };
            self.dodge_remaining = 0.23;
            self.dodge_cooldown = 0.65;
            self.attack = None;
            self.combo_window = 0.0;
            self.buffered = false;
            self.buffered_perfect = false;
            self.buffered_photon = false;
            events.push(WorldEvent::Step);
        }
        if self.dodging() {
            self.move_player(self.dodge_direction, 11.0, delta);
            self.dodge_remaining = (self.dodge_remaining - delta).max(0.0);
        } else {
            let speed = if self.hurt_remaining > 0.0 {
                1.0
            } else if self.attack.is_some() {
                1.3
            } else if input.sprint {
                6.5
            } else {
                4.0
            };
            self.move_player(movement, speed, delta);
        }
        if !self.dodging() && self.hurt_remaining == 0.0 {
            if let Some(attack) = &self.attack {
                if press {
                    self.buffered = true;
                    self.buffered_perfect |= attack.timing_window();
                }
            } else if self.buffered_photon && self.pp >= 35.0 {
                self.begin_attack(true, false, &mut events);
            } else if press || input.attack || self.buffered {
                self.begin_attack(false, self.buffered_perfect, &mut events);
            }
        }
        if let Some(mut attack) = self.attack {
            attack.elapsed += delta;
            let hit_at = if attack.photon {
                0.34
            } else {
                [0.18, 0.2, 0.28][attack.stage as usize - 1]
            };
            if !attack.hit && attack.elapsed >= hit_at {
                attack.hit = true;
                let target = direction(self.position, self.enemy_position);
                let alignment = self.facing.sin() * target.x + self.facing.cos() * target.z;
                if self.session.location() == Location::TrainingField
                    && self.session.quest() == QuestState::Active
                    && self.position.distance(self.enemy_position)
                        <= if attack.photon { 3.5 } else { ATTACK_RANGE }
                    && alignment >= 0.35
                    && self.height < 1.6
                {
                    let base = self.session.hunter().attack_power();
                    let power = if attack.photon {
                        base * 3
                    } else {
                        base * [100, 135, 185][attack.stage as usize - 1] / 100
                    };
                    let power = if attack.perfect {
                        power * 130 / 100
                    } else {
                        power
                    };
                    if let Ok(result) = self.session.damage_enemy(power) {
                        events.extend(result.into_iter().map(WorldEvent::Gameplay));
                        if !attack.photon {
                            self.pp = (self.pp + 8.0).min(100.0);
                        }
                        self.hit_stop = if attack.stage == 3 || attack.photon {
                            0.065
                        } else {
                            0.035
                        };
                        if self.session.quest() == QuestState::Complete {
                            self.enemy_action = EnemyAction::Dead;
                            self.enemy_elapsed = 0.0;
                            self.locked = false;
                        } else if attack.stage == 3 || attack.photon {
                            self.enemy_action = EnemyAction::Stagger;
                            self.enemy_elapsed = 0.0;
                        }
                    }
                }
            }
            if attack.elapsed >= attack.duration {
                self.attack = None;
                self.combo_window = 0.4;
            } else {
                self.attack = Some(attack);
            }
        }
        self.enemy_tick(delta, &mut events);
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ENEMY_MAX_HEALTH;
    fn trial() -> World {
        let mut world = World::default();
        world.session.execute(Command::StartQuest).unwrap();
        world.reset_encounter();
        world
    }
    fn close_trial() -> World {
        let mut w = trial();
        w.position = Position { x: 0.0, z: -1.9 };
        w.enemy_action = EnemyAction::Recover;
        w.enemy_elapsed = -100.0;
        w
    }
    fn ticks(world: &mut World, n: usize, input: Input) {
        for _ in 0..n {
            world.step(input, FIXED_STEP);
        }
    }
    #[test]
    fn movement_normalizes_diagonals_and_rejects_invalid_inputs() {
        let mut a = World::default();
        let mut b = a.clone();
        a.step(
            Input {
                movement: Position { x: 1.0, z: 0.0 },
                ..Input::default()
            },
            FIXED_STEP,
        );
        b.step(
            Input {
                movement: Position { x: 1.0, z: 1.0 },
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert!((a.distance_walked - b.distance_walked).abs() < 0.0001);
        let before = a.clone();
        a.step(
            Input {
                movement: Position {
                    x: f32::NAN,
                    z: 0.0,
                },
                ..Input::default()
            },
            FIXED_STEP,
        );
        a.step(Input::default(), f32::INFINITY);
        assert_eq!(a, before);
    }
    #[test]
    fn walls_and_props_block_walk_and_fast_steps() {
        let mut w = World {
            position: Position { x: 11.5, z: 0.0 },
            ..World::default()
        };
        w.step(
            Input {
                dodge: true,
                movement: Position { x: 1.0, z: 0.0 },
                ..Input::default()
            },
            0.1,
        );
        assert!(w.position.x <= 12.0 - PLAYER_RADIUS);
        w = World {
            position: Position { x: -6.0, z: 0.0 },
            ..World::default()
        };
        ticks(
            &mut w,
            60,
            Input {
                movement: Position { x: 0.0, z: -1.0 },
                ..Input::default()
            },
        );
        assert!(w.position.z >= -0.82);
    }
    #[test]
    fn attack_contacts_once_after_windup_and_never_causes_automatic_counter_damage() {
        let mut w = close_trial();
        w.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert_eq!(w.session.enemy_health(), ENEMY_MAX_HEALTH);
        ticks(&mut w, 15, Input::default());
        assert_eq!(w.session.enemy_health(), ENEMY_MAX_HEALTH - 12);
        assert_eq!(w.session.hunter().health, 100);
        ticks(&mut w, 60, Input::default());
        assert_eq!(w.session.enemy_health(), ENEMY_MAX_HEALTH - 12);
        w.position = Position { x: 0.0, z: 4.0 };
        w.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        ticks(&mut w, 40, Input::default());
        assert_eq!(w.session.enemy_health(), ENEMY_MAX_HEALTH - 12);
        w.position = Position { x: 0.0, z: -1.9 };
        w.facing = 0.0;
        w.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        ticks(&mut w, 45, Input::default());
        assert_eq!(w.session.enemy_health(), ENEMY_MAX_HEALTH - 12);
    }
    #[test]
    fn timed_press_buffers_a_stronger_followup_and_combo_expires() {
        let mut w = close_trial();
        w.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        while !w.attack.unwrap().timing_window() {
            w.step(Input::default(), FIXED_STEP);
        }
        w.step(
            Input {
                attack_pressed: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        while w.attack.is_some_and(|a| a.stage == 1) {
            w.step(Input::default(), FIXED_STEP);
        }
        let events = w.step(Input::default(), FIXED_STEP);
        assert!(events.contains(&WorldEvent::PerfectAttack));
        assert_eq!(w.attack.unwrap().stage, 2);
        assert!(w.attack.unwrap().perfect);
        ticks(&mut w, 25, Input::default());
        assert_eq!(w.session.enemy_health(), ENEMY_MAX_HEALTH - 12 - 20);
        ticks(&mut w, 70, Input::default());
        w.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert_eq!(w.attack.unwrap().stage, 1);
    }
    #[test]
    fn dodge_evades_strike_but_cannot_bypass_collision_or_cooldown() {
        let mut w = close_trial();
        w.enemy_action = EnemyAction::Strike;
        w.enemy_elapsed = 0.07;
        w.strike_direction = Position { x: 0.0, z: 1.0 };
        let events = w.step(
            Input {
                dodge: true,
                movement: Position { x: 1.0, z: 0.0 },
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert!(events.contains(&WorldEvent::Evaded));
        assert_eq!(w.session.hunter().health, 100);
        ticks(&mut w, 16, Input::default());
        assert!(!w.dodging());
        let events = w.step(
            Input {
                dodge: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert!(!events.contains(&WorldEvent::Step));
        ticks(&mut w, 40, Input::default());
        assert!(w.dodge_ready());
    }
    #[test]
    fn hit_stop_retains_step_input_and_lock_on_keeps_strafe_facing() {
        let mut w = close_trial();
        w.step(
            Input {
                lock_on: true,
                movement: Position { x: 1.0, z: 0.0 },
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert!(w.locked_on());
        let to = direction(w.position, w.enemy_position);
        assert!(to.x * w.facing.sin() + to.z * w.facing.cos() > 0.99);
        w.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        while w.hit_stop == 0.0 {
            w.step(Input::default(), FIXED_STEP);
        }
        let events = w.step(
            Input {
                dodge: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert!(!events.contains(&WorldEvent::Step));
        let mut stepped = false;
        for _ in 0..6 {
            stepped |= w
                .step(Input::default(), FIXED_STEP)
                .contains(&WorldEvent::Step);
        }
        assert!(stepped);
        assert!(w.attack.is_none());
    }
    #[test]
    fn photon_art_spends_pp_and_normal_hits_restore_it() {
        let mut w = close_trial();
        w.step(
            Input {
                photon_art: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert!(w.attack.unwrap().photon);
        assert!((w.pp - 65.0).abs() < 0.001);
        ticks(&mut w, 70, Input::default());
        assert_eq!(w.session.enemy_health(), ENEMY_MAX_HEALTH - 36);
        w.pp = 0.0;
        w.step(
            Input {
                photon_art: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert!(w.attack.is_none());
        w.step(
            Input {
                attack: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        ticks(&mut w, 15, Input::default());
        assert!(w.pp >= 8.0);
    }
    #[test]
    fn enemy_chases_telegraphs_defeats_and_retry_restores_encounter() {
        let mut w = trial();
        let start = w.enemy_position;
        let mut windups = 0;
        for _ in 0..1500 {
            for event in w.step(Input::default(), FIXED_STEP) {
                if event == WorldEvent::EnemyWindup {
                    windups += 1;
                }
            }
            if w.defeated() {
                break;
            }
        }
        assert_ne!(w.enemy_position, start);
        assert!(windups >= 6);
        assert!(w.defeated());
        assert_eq!(w.session.hunter().experience, 0);
        w.step(
            Input {
                interact: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert_eq!(w.session.hunter().health, 100);
        assert_eq!(w.session.enemy_health(), ENEMY_MAX_HEALTH);
        assert_eq!(w.enemy_position, ENEMY);
    }
    #[test]
    fn jump_lands_and_quest_rewards_only_once() {
        let mut w = close_trial();
        w.step(
            Input {
                jump: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert!(w.height > 0.0);
        ticks(&mut w, 60, Input::default());
        assert_eq!(w.height, 0.0);
        ticks(
            &mut w,
            800,
            Input {
                attack: true,
                ..Input::default()
            },
        );
        assert_eq!(w.session.quest(), QuestState::Complete);
        assert_eq!(w.session.hunter().level, 2);
        ticks(
            &mut w,
            150,
            Input {
                attack: true,
                ..Input::default()
            },
        );
        assert_eq!(w.session.hunter().level, 2);
        w.position = RETURN_PAD;
        w.step(
            Input {
                interact: true,
                ..Input::default()
            },
            FIXED_STEP,
        );
        assert_eq!(w.session.location(), Location::Lobby);
        assert_eq!(w.session.hunter().health, 120);
    }
}
