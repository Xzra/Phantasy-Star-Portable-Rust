//! Fixed lobby stock. Prices and currency are original prototype balance.
use crate::weapon::{PhotonArt, WeaponKind};
pub const STARTING_MESETA: u32 = 300;
pub const TRIAL_MESETA: u32 = 300;
pub const MAX_MESETA: u32 = 999_999;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StockItem {
    Weapon(WeaponKind),
    Disc(PhotonArt),
}
pub const STOCK: [StockItem; 6] = [
    StockItem::Weapon(WeaponKind::TrainingSword),
    StockItem::Weapon(WeaponKind::HunterSaber),
    StockItem::Weapon(WeaponKind::PhotonEdge),
    StockItem::Disc(PhotonArt::Sweep),
    StockItem::Disc(PhotonArt::RisingCleave),
    StockItem::Disc(PhotonArt::PiercingThrust),
];
impl StockItem {
    pub fn name(self) -> &'static str {
        match self {
            Self::Weapon(w) => w.name(),
            Self::Disc(a) => a.spec().name,
        }
    }
    pub fn price(self) -> u32 {
        match self {
            Self::Weapon(WeaponKind::TrainingSword) => 60,
            Self::Weapon(WeaponKind::HunterSaber) => 180,
            Self::Weapon(WeaponKind::PhotonEdge) => 320,
            Self::Disc(PhotonArt::Sweep) => 120,
            Self::Disc(PhotonArt::RisingCleave) => 160,
            Self::Disc(PhotonArt::PiercingThrust) => 100,
        }
    }
}
