//! Durable character state. Encounters, HP, position and temporary rewards are not saved.
use crate::inventory::InventoryState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub character: crate::character::Character,
    pub level: u32,
    pub experience: u32,
    pub inventory: InventoryState,
}

#[cfg(test)]
mod tests {
    use crate::{inventory::WeaponId, world::World, Location, QuestState};
    #[test]
    fn restoring_a_profile_returns_to_a_fresh_lobby_without_replaying_rewards() {
        let mut profile = World::default().profile();
        profile.level = 7;
        profile.experience = 50;
        profile.inventory.meseta = 42;
        profile.inventory.equipped = WeaponId(2);
        let world = World::from_profile(profile.clone()).unwrap();
        assert_eq!(world.profile(), profile);
        assert_eq!(world.session().location(), Location::Lobby);
        assert_eq!(world.session().quest(), QuestState::Available);
        assert_eq!(world.session().hunter().health, 220);
        assert!(world.attack().is_none());
        assert_eq!(world.pp(), 100.0);
        assert_eq!(world.stats().defeats, 0);
    }
    #[test]
    fn rejects_invalid_progression_and_item_relationships() {
        let starter = World::default().profile();
        for (level, xp) in [(0, 0), (101, 0), (1, 100), (100, 1), (u32::MAX, 0)] {
            let mut p = starter.clone();
            p.level = level;
            p.experience = xp;
            assert!(World::from_profile(p).is_err());
        }
        for case in 0..8 {
            let mut p = starter.clone();
            match case {
                0 => p.inventory.weapons.clear(),
                1 => p.inventory.weapons[1].id = p.inventory.weapons[0].id,
                2 => p.inventory.equipped = WeaponId(99),
                3 => p.inventory.discs.clear(),
                4 => {
                    p.inventory.weapons[1].loadout.photon_art =
                        p.inventory.weapons[0].loadout.photon_art
                }
                5 => p.inventory.discs.push(p.inventory.discs[0]),
                6 => p.inventory.meseta = u32::MAX,
                _ => p.inventory.next_id = 1,
            }
            assert!(World::from_profile(p).is_err(), "case {case}");
        }
    }
    #[test]
    fn level_cap_keeps_progression_valid_after_rewards() {
        let mut hunter = crate::Hunter {
            level: 99,
            experience: 9890,
            health: 10,
        };
        hunter.award_experience(100);
        assert_eq!(hunter.level, 100);
        assert_eq!(hunter.experience, 0);
        hunter.award_experience(100);
        assert_eq!(hunter.experience, 0);
    }
}
