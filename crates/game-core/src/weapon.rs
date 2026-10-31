//! Weapon definitions and equipable Photon Arts. Balance is original.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PhotonArt {
    #[default]
    Sweep,
    RisingCleave,
    PiercingThrust,
    AssaultThrust,
    MeteorFist,
}

#[derive(Debug, Clone, Copy)]
pub struct ArtSpec {
    pub name: &'static str,
    pub short_name: &'static str,
    pub description: &'static str,
    pub pp: f32,
    pub duration: f32,
    pub hit_at: f32,
    pub range: f32,
    pub alignment: f32,
    pub power_percent: u32,
}
impl PhotonArt {
    pub const SWORD: [Self; 3] = [Self::Sweep, Self::RisingCleave, Self::PiercingThrust];
    pub const ALL: [Self; 5] = [
        Self::Sweep,
        Self::RisingCleave,
        Self::PiercingThrust,
        Self::AssaultThrust,
        Self::MeteorFist,
    ];
    pub fn class(self) -> WeaponClass {
        match self {
            Self::Sweep | Self::RisingCleave | Self::PiercingThrust => WeaponClass::Sword,
            Self::AssaultThrust => WeaponClass::Partisan,
            Self::MeteorFist => WeaponClass::Knuckles,
        }
    }
    pub fn spec(self) -> ArtSpec {
        match self {
            Self::AssaultThrust => ArtSpec {
                name: "Assault Thrust",
                short_name: "ASSAULT",
                description: "A committed spear thrust with long, narrow reach.",
                pp: 25.0,
                duration: 0.8,
                hit_at: 0.32,
                range: 4.8,
                alignment: 0.9,
                power_percent: 320,
            },
            Self::MeteorFist => ArtSpec {
                name: "Meteor Fist",
                short_name: "METEOR",
                description: "A heavy straight punch. Stay close and face your target.",
                pp: 25.0,
                duration: 0.65,
                hit_at: 0.25,
                range: 2.2,
                alignment: 0.65,
                power_percent: 330,
            },
            Self::Sweep => ArtSpec {
                name: "Photon Sweep",
                short_name: "SWEEP",
                description: "A spinning cut that strikes around you.",
                pp: 35.0,
                duration: 0.85,
                hit_at: 0.34,
                range: 3.5,
                alignment: -1.0,
                power_percent: 300,
            },
            Self::RisingCleave => ArtSpec {
                name: "Rising Cleave",
                short_name: "CLEAVE",
                description: "A heavy upward cut. Face the target for a strong close hit.",
                pp: 30.0,
                duration: 1.0,
                hit_at: 0.45,
                range: 2.8,
                alignment: 0.55,
                power_percent: 400,
            },
            Self::PiercingThrust => ArtSpec {
                name: "Piercing Thrust",
                short_name: "THRUST",
                description: "A quick, narrow thrust with extended reach.",
                pp: 20.0,
                duration: 0.58,
                hit_at: 0.22,
                range: 4.2,
                alignment: 0.88,
                power_percent: 200,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponClass {
    Sword,
    Partisan,
    Knuckles,
}
#[derive(Debug, Clone, Copy)]
pub struct AttackSpec {
    pub duration: f32,
    pub hit_at: f32,
    pub range: f32,
    pub alignment: f32,
    pub power_percent: u32,
}
impl WeaponClass {
    pub fn name(self) -> &'static str {
        match self {
            Self::Sword => "SWORD",
            Self::Partisan => "PARTISAN",
            Self::Knuckles => "KNUCKLES",
        }
    }
    pub fn attack(self, stage: u8) -> AttackSpec {
        let i = stage.saturating_sub(1).min(2) as usize;
        let (durations, hits, powers, range, alignment) = match self {
            Self::Sword => (
                [0.5, 0.52, 0.68],
                [0.18, 0.2, 0.28],
                [100, 135, 185],
                2.6,
                0.35,
            ),
            Self::Partisan => (
                [0.58, 0.6, 0.78],
                [0.22, 0.24, 0.32],
                [110, 145, 200],
                3.7,
                0.8,
            ),
            Self::Knuckles => (
                [0.32, 0.34, 0.46],
                [0.12, 0.13, 0.18],
                [80, 105, 150],
                1.95,
                0.5,
            ),
        };
        AttackSpec {
            duration: durations[i],
            hit_at: hits[i],
            power_percent: powers[i],
            range,
            alignment,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponKind {
    TrainingSword,
    HunterSaber,
    PhotonEdge,
    RecruitPartisan,
    RecruitKnuckles,
}
impl WeaponKind {
    pub fn class(self) -> WeaponClass {
        match self {
            Self::TrainingSword | Self::HunterSaber | Self::PhotonEdge => WeaponClass::Sword,
            Self::RecruitPartisan => WeaponClass::Partisan,
            Self::RecruitKnuckles => WeaponClass::Knuckles,
        }
    }
    pub fn accepts(self, art: PhotonArt) -> bool {
        self.class() == art.class()
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::TrainingSword => "Training Sword",
            Self::HunterSaber => "Hunter Saber",
            Self::PhotonEdge => "Photon Edge",
            Self::RecruitPartisan => "Recruit Partisan",
            Self::RecruitKnuckles => "Recruit Knuckles",
        }
    }
    pub fn attack_bonus(self) -> u32 {
        match self {
            Self::TrainingSword => 0,
            Self::HunterSaber => 3,
            Self::PhotonEdge => 6,
            Self::RecruitPartisan | Self::RecruitKnuckles => 2,
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::TrainingSword => "A balanced practice sword.",
            Self::HunterSaber => "A reinforced Hunter blade.",
            Self::PhotonEdge => "A high-output photon blade.",
            Self::RecruitPartisan => "Long reach; narrow thrusts.",
            Self::RecruitKnuckles => "Fast punches; stay close.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponLoadout {
    pub kind: WeaponKind,
    pub photon_art: Option<PhotonArt>,
}
impl Default for WeaponLoadout {
    fn default() -> Self {
        Self {
            kind: WeaponKind::TrainingSword,
            photon_art: Some(PhotonArt::Sweep),
        }
    }
}
