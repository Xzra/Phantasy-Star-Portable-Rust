//! Sword definitions and equipable Photon Arts. Balance is original.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PhotonArt {
    #[default]
    Sweep,
    RisingCleave,
    PiercingThrust,
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
    pub const ALL: [Self; 3] = [Self::Sweep, Self::RisingCleave, Self::PiercingThrust];
    pub fn spec(self) -> ArtSpec {
        match self {
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
pub enum WeaponKind {
    TrainingSword,
    HunterSaber,
    PhotonEdge,
}
impl WeaponKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::TrainingSword => "Training Sword",
            Self::HunterSaber => "Hunter Saber",
            Self::PhotonEdge => "Photon Edge",
        }
    }
    pub fn attack_bonus(self) -> u32 {
        match self {
            Self::TrainingSword => 0,
            Self::HunterSaber => 3,
            Self::PhotonEdge => 6,
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::TrainingSword => "A balanced practice sword.",
            Self::HunterSaber => "A reinforced Hunter blade.",
            Self::PhotonEdge => "A high-output photon blade.",
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
