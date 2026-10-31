//! Rendering-independent solo gameplay. A future server can own the same session.

pub mod inventory;
pub mod weapon;
pub mod world;

pub const ENEMY_MAX_HEALTH: u32 = 144;
pub const TRIAL_WAVES: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    Lobby,
    TrainingField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunter {
    pub level: u32,
    pub experience: u32,
    pub health: u32,
}

impl Hunter {
    pub fn max_health(&self) -> u32 {
        100 + (self.level - 1) * 20
    }

    pub fn attack_power(&self) -> u32 {
        12 + (self.level - 1) * 3
    }

    fn award_experience(&mut self, amount: u32) {
        self.experience += amount;
        while self.experience >= self.level * 100 {
            self.experience -= self.level * 100;
            self.level += 1;
            self.health = self.max_health();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestState {
    Available,
    Active,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    StartQuest,
    Attack,
    ReturnToLobby,
    RetryQuest,
    NextWave,
    UseMate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    QuestStarted,
    EnemyDamaged { damage: u32, remaining_health: u32 },
    HunterDamaged { damage: u32, remaining_health: u32 },
    QuestCompleted { experience: u32 },
    ReturnedToLobby,
    HunterDefeated,
    EnemyDefeated { wave: u8 },
    WaveStarted { wave: u8 },
    HunterHealed { amount: u32, remaining_health: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    hunter: Hunter,
    location: Location,
    quest: QuestState,
    enemy_health: u32,
    wave: u8,
    mates: u8,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            hunter: Hunter {
                level: 1,
                experience: 0,
                health: 100,
            },
            location: Location::Lobby,
            quest: QuestState::Available,
            enemy_health: 0,
            wave: 0,
            mates: 3,
        }
    }
}

impl Session {
    pub fn hunter(&self) -> &Hunter {
        &self.hunter
    }
    pub fn location(&self) -> Location {
        self.location
    }
    pub fn quest(&self) -> QuestState {
        self.quest
    }
    pub fn enemy_health(&self) -> u32 {
        self.enemy_health
    }

    pub fn wave(&self) -> u8 {
        self.wave
    }
    pub fn mates(&self) -> u8 {
        self.mates
    }
    pub fn enemy_max_health(&self) -> u32 {
        ENEMY_MAX_HEALTH + 24 * self.wave.saturating_sub(1) as u32
    }
    /// Invalid commands return an error without changing the session.
    pub fn execute(&mut self, command: Command) -> Result<Vec<Event>, &'static str> {
        match command {
            Command::StartQuest => {
                if self.location != Location::Lobby || self.quest == QuestState::Active {
                    return Err("deploy from the lobby when no quest is active");
                }
                self.location = Location::TrainingField;
                self.quest = QuestState::Active;
                self.wave = 1;
                self.mates = 3;
                self.enemy_health = self.enemy_max_health();
                Ok(vec![Event::QuestStarted])
            }
            Command::Attack => self.damage_enemy(self.hunter.attack_power()),
            Command::RetryQuest => {
                if self.location != Location::TrainingField
                    || self.quest != QuestState::Active
                    || self.hunter.health != 0
                {
                    return Err("retry is only available after defeat in an active quest");
                }
                self.hunter.health = self.hunter.max_health();
                self.wave = 1;
                self.mates = 3;
                self.enemy_health = self.enemy_max_health();
                Ok(vec![Event::QuestStarted])
            }
            Command::NextWave => {
                if self.location != Location::TrainingField
                    || self.quest != QuestState::Active
                    || self.enemy_health != 0
                    || self.wave >= TRIAL_WAVES
                    || self.hunter.health == 0
                {
                    return Err("clear the current wave before advancing");
                }
                self.wave += 1;
                self.enemy_health = self.enemy_max_health();
                Ok(vec![Event::WaveStarted { wave: self.wave }])
            }
            Command::UseMate => {
                if self.location != Location::TrainingField
                    || self.quest != QuestState::Active
                    || self.hunter.health == 0
                    || self.hunter.health == self.hunter.max_health()
                    || self.mates == 0
                {
                    return Err(
                        "recovery requires missing health and an available mate during the trial",
                    );
                }
                let amount = 40.min(self.hunter.max_health() - self.hunter.health);
                self.hunter.health += amount;
                self.mates -= 1;
                Ok(vec![Event::HunterHealed {
                    amount,
                    remaining_health: self.hunter.health,
                }])
            }
            Command::ReturnToLobby => {
                if self.location != Location::TrainingField || self.quest != QuestState::Complete {
                    return Err("finish the training quest before returning to the lobby");
                }
                self.location = Location::Lobby;
                self.hunter.health = self.hunter.max_health();
                Ok(vec![Event::ReturnedToLobby])
            }
        }
    }
    fn damage_enemy(&mut self, power: u32) -> Result<Vec<Event>, &'static str> {
        if self.quest != QuestState::Active
            || self.location != Location::TrainingField
            || self.hunter.health == 0
            || self.enemy_health == 0
        {
            return Err("there is no active combat encounter");
        }
        let damage = power.min(self.enemy_health);
        self.enemy_health -= damage;
        let mut events = vec![Event::EnemyDamaged {
            damage,
            remaining_health: self.enemy_health,
        }];
        if self.enemy_health == 0 {
            events.push(Event::EnemyDefeated { wave: self.wave });
            if self.wave < TRIAL_WAVES {
                return Ok(events);
            }
            self.quest = QuestState::Complete;
            self.hunter.award_experience(100);
            events.push(Event::QuestCompleted { experience: 100 });
        }
        Ok(events)
    }
    fn damage_hunter(&mut self, power: u32) -> Vec<Event> {
        if self.quest != QuestState::Active
            || self.location != Location::TrainingField
            || self.hunter.health == 0
        {
            return Vec::new();
        }
        let damage = power.min(self.hunter.health);
        self.hunter.health -= damage;
        let mut events = vec![Event::HunterDamaged {
            damage,
            remaining_health: self.hunter.health,
        }];
        if self.hunter.health == 0 {
            events.push(Event::HunterDefeated);
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_quest_rewards_once_and_returns_to_lobby() {
        let mut session = Session::default();
        session.execute(Command::StartQuest).unwrap();
        while session.wave() < TRIAL_WAVES || session.enemy_health() > 12 {
            session
                .execute(if session.enemy_health() == 0 {
                    Command::NextWave
                } else {
                    Command::Attack
                })
                .unwrap();
        }
        assert_eq!(session.hunter().health, 100);
        let events = session.execute(Command::Attack).unwrap();
        assert!(events.contains(&Event::QuestCompleted { experience: 100 }));
        assert_eq!(session.hunter().level, 2);
        assert_eq!(session.hunter().health, 120);
        let completed = session.clone();
        assert!(session.execute(Command::Attack).is_err());
        assert_eq!(session, completed);
        session.execute(Command::ReturnToLobby).unwrap();
        assert_eq!(session.location(), Location::Lobby);
        session.execute(Command::StartQuest).unwrap();
        assert_eq!(session.hunter().level, 2);
        assert_eq!(session.wave(), 1);
    }

    #[test]
    fn invalid_transitions_do_not_mutate_state() {
        let mut session = Session::default();
        let original = session.clone();
        assert!(session.execute(Command::Attack).is_err());
        assert!(session.execute(Command::NextWave).is_err());
        assert!(session.execute(Command::UseMate).is_err());
        assert!(session.execute(Command::ReturnToLobby).is_err());
        assert_eq!(session, original);
        session.execute(Command::StartQuest).unwrap();
        let active = session.clone();
        assert!(session.execute(Command::StartQuest).is_err());
        assert!(session.execute(Command::ReturnToLobby).is_err());
        assert_eq!(session, active);
    }
}
