//! Rendering-independent solo gameplay. A future server can own the same session.

pub mod world;

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    QuestStarted,
    EnemyDamaged { damage: u32, remaining_health: u32 },
    HunterDamaged { damage: u32, remaining_health: u32 },
    QuestCompleted { experience: u32 },
    ReturnedToLobby,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    hunter: Hunter,
    location: Location,
    quest: QuestState,
    enemy_health: u32,
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

    /// Invalid commands return an error without changing the session.
    pub fn execute(&mut self, command: Command) -> Result<Vec<Event>, &'static str> {
        match command {
            Command::StartQuest => {
                if self.location != Location::Lobby || self.quest != QuestState::Available {
                    return Err("the training quest can only start once from the lobby");
                }
                self.location = Location::TrainingField;
                self.quest = QuestState::Active;
                self.enemy_health = 36;
                Ok(vec![Event::QuestStarted])
            }
            Command::Attack => {
                if self.quest != QuestState::Active || self.location != Location::TrainingField {
                    return Err("there is no active enemy to attack");
                }
                let damage = self.hunter.attack_power().min(self.enemy_health);
                self.enemy_health -= damage;
                let mut events = vec![Event::EnemyDamaged {
                    damage,
                    remaining_health: self.enemy_health,
                }];
                if self.enemy_health == 0 {
                    self.quest = QuestState::Complete;
                    self.hunter.award_experience(100);
                    events.push(Event::QuestCompleted { experience: 100 });
                } else {
                    let damage = 5.min(self.hunter.health);
                    self.hunter.health -= damage;
                    events.push(Event::HunterDamaged {
                        damage,
                        remaining_health: self.hunter.health,
                    });
                }
                Ok(events)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_quest_rewards_once_and_returns_to_lobby() {
        let mut session = Session::default();
        session.execute(Command::StartQuest).unwrap();
        session.execute(Command::Attack).unwrap();
        session.execute(Command::Attack).unwrap();
        assert_eq!(session.hunter().health, 90);
        let events = session.execute(Command::Attack).unwrap();
        assert!(events.contains(&Event::QuestCompleted { experience: 100 }));
        assert_eq!(session.hunter().level, 2);
        assert_eq!(session.hunter().health, 120);
        let completed = session.clone();
        assert!(session.execute(Command::Attack).is_err());
        assert_eq!(session, completed);
        session.execute(Command::ReturnToLobby).unwrap();
        assert_eq!(session.location(), Location::Lobby);
        assert!(session.execute(Command::StartQuest).is_err());
    }

    #[test]
    fn invalid_transitions_do_not_mutate_state() {
        let mut session = Session::default();
        let original = session.clone();
        assert!(session.execute(Command::Attack).is_err());
        assert!(session.execute(Command::ReturnToLobby).is_err());
        assert_eq!(session, original);
        session.execute(Command::StartQuest).unwrap();
        let active = session.clone();
        assert!(session.execute(Command::StartQuest).is_err());
        assert!(session.execute(Command::ReturnToLobby).is_err());
        assert_eq!(session, active);
    }
}
