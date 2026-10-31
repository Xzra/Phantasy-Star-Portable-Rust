//! Owned item instances. One PA disc can be slotted into one owned sword at a time.
use crate::weapon::{PhotonArt, WeaponKind, WeaponLoadout};

pub const INVENTORY_CAPACITY: usize = 32;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponItem {
    pub id: WeaponId,
    pub loadout: WeaponLoadout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    weapons: Vec<WeaponItem>,
    discs: Vec<PhotonArt>,
    equipped: WeaponId,
    next_id: u32,
}
impl Default for Inventory {
    fn default() -> Self {
        let weapons = [
            WeaponKind::TrainingSword,
            WeaponKind::HunterSaber,
            WeaponKind::PhotonEdge,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, kind)| WeaponItem {
            id: WeaponId(i as u32 + 1),
            loadout: WeaponLoadout {
                kind,
                photon_art: Some(PhotonArt::ALL[i]),
            },
        })
        .collect();
        Self {
            weapons,
            discs: PhotonArt::ALL.to_vec(),
            equipped: WeaponId(1),
            next_id: 4,
        }
    }
}
impl Inventory {
    pub fn weapons(&self) -> &[WeaponItem] {
        &self.weapons
    }
    pub fn discs(&self) -> &[PhotonArt] {
        &self.discs
    }
    pub fn item_count(&self) -> usize {
        self.weapons.len() + self.discs.len()
    }
    pub fn equipped_id(&self) -> WeaponId {
        self.equipped
    }
    pub fn weapon(&self, id: WeaponId) -> Option<&WeaponItem> {
        self.weapons.iter().find(|w| w.id == id)
    }
    pub fn equipped(&self) -> WeaponLoadout {
        self.weapon(self.equipped)
            .expect("equipped weapon is owned")
            .loadout
    }
    pub fn disc_owner(&self, art: PhotonArt) -> Option<WeaponId> {
        self.weapons
            .iter()
            .find(|w| w.loadout.photon_art == Some(art))
            .map(|w| w.id)
    }
    pub fn equip(&mut self, id: WeaponId) -> Result<(), &'static str> {
        if self.weapon(id).is_none() {
            return Err("That weapon is not in your inventory.");
        }
        self.equipped = id;
        Ok(())
    }
    /// Validate first, then atomically move the owned disc to this weapon's slot.
    pub fn assign(&mut self, id: WeaponId, art: Option<PhotonArt>) -> Result<(), &'static str> {
        if self.weapon(id).is_none() {
            return Err("That weapon is not in your inventory.");
        }
        if art.is_some_and(|a| !self.discs.contains(&a)) {
            return Err("You do not own that PA disc.");
        }
        if let Some(art) = art {
            for weapon in &mut self.weapons {
                if weapon.loadout.photon_art == Some(art) {
                    weapon.loadout.photon_art = None;
                }
            }
        }
        self.weapons
            .iter_mut()
            .find(|w| w.id == id)
            .unwrap()
            .loadout
            .photon_art = art;
        Ok(())
    }
    pub fn grant_weapon(&mut self, kind: WeaponKind) -> Result<WeaponId, &'static str> {
        if self.item_count() >= INVENTORY_CAPACITY {
            return Err("Inventory is full.");
        }
        let next = self
            .next_id
            .checked_add(1)
            .ok_or("Weapon identifiers exhausted.")?;
        let id = WeaponId(self.next_id);
        self.weapons.push(WeaponItem {
            id,
            loadout: WeaponLoadout {
                kind,
                photon_art: None,
            },
        });
        self.next_id = next;
        Ok(id)
    }
    pub fn grant_disc(&mut self, art: PhotonArt) -> Result<(), &'static str> {
        if self.discs.contains(&art) {
            return Err("You already own that PA disc.");
        }
        if self.item_count() >= INVENTORY_CAPACITY {
            return Err("Inventory is full.");
        }
        self.discs.push(art);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weapon_instances_keep_their_slots_when_switching() {
        let mut bag = Inventory::default();
        let first = bag.equipped();
        bag.equip(WeaponId(2)).unwrap();
        assert_eq!(bag.equipped().kind, WeaponKind::HunterSaber);
        assert_eq!(bag.equipped().photon_art, Some(PhotonArt::RisingCleave));
        bag.equip(WeaponId(1)).unwrap();
        assert_eq!(bag.equipped(), first);
        let duplicate = bag.grant_weapon(WeaponKind::TrainingSword).unwrap();
        assert_ne!(duplicate, WeaponId(1));
        assert_eq!(bag.weapon(duplicate).unwrap().loadout.photon_art, None);
    }
    #[test]
    fn transferring_a_disc_never_duplicates_or_consumes_it() {
        let mut bag = Inventory::default();
        let count = bag.item_count();
        bag.assign(WeaponId(2), Some(PhotonArt::Sweep)).unwrap();
        assert_eq!(bag.weapon(WeaponId(1)).unwrap().loadout.photon_art, None);
        assert_eq!(bag.disc_owner(PhotonArt::Sweep), Some(WeaponId(2)));
        assert_eq!(bag.disc_owner(PhotonArt::RisingCleave), None);
        bag.assign(WeaponId(2), None).unwrap();
        assert!(bag.discs().contains(&PhotonArt::Sweep));
        assert_eq!(bag.item_count(), count);
    }
    #[test]
    fn invalid_ownership_and_capacity_fail_without_mutation() {
        let mut bag = Inventory::default();
        let before = bag.clone();
        assert!(bag.assign(WeaponId(999), Some(PhotonArt::Sweep)).is_err());
        assert!(bag.equip(WeaponId(999)).is_err());
        assert!(bag.grant_disc(PhotonArt::Sweep).is_err());
        assert_eq!(bag, before);
        bag.assign(WeaponId(1), None).unwrap();
        bag.discs.retain(|a| *a != PhotonArt::Sweep);
        let before = bag.clone();
        assert!(bag.assign(WeaponId(1), Some(PhotonArt::Sweep)).is_err());
        assert_eq!(bag, before);
        bag.grant_disc(PhotonArt::Sweep).unwrap();
        while bag.item_count() < INVENTORY_CAPACITY {
            bag.grant_weapon(WeaponKind::TrainingSword).unwrap();
        }
        let before = bag.clone();
        assert!(bag.grant_weapon(WeaponKind::PhotonEdge).is_err());
        assert_eq!(bag, before);
    }
}
