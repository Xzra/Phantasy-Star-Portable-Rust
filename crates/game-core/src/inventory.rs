//! Owned item instances. One PA disc can be slotted into one owned sword at a time.
use crate::shop::{StockItem, MAX_MESETA, STARTING_MESETA};
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
pub struct InventoryState {
    pub meseta: u32,
    pub weapons: Vec<WeaponItem>,
    pub discs: Vec<PhotonArt>,
    pub equipped: WeaponId,
    pub next_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    meseta: u32,
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
            meseta: STARTING_MESETA,
            weapons,
            discs: PhotonArt::ALL.to_vec(),
            equipped: WeaponId(1),
            next_id: 4,
        }
    }
}
impl Inventory {
    pub fn snapshot(&self) -> InventoryState {
        InventoryState {
            meseta: self.meseta,
            weapons: self.weapons.clone(),
            discs: self.discs.clone(),
            equipped: self.equipped,
            next_id: self.next_id,
        }
    }
    pub fn from_state(state: InventoryState) -> Result<Self, &'static str> {
        if state.meseta > MAX_MESETA
            || state.weapons.is_empty()
            || state.weapons.len() + state.discs.len() > INVENTORY_CAPACITY
            || state.next_id == 0
        {
            return Err("Invalid saved inventory capacity, wallet or identifiers.");
        }
        let mut ids = std::collections::HashSet::new();
        let mut assigned = Vec::new();
        for w in &state.weapons {
            if w.id.0 == 0 || w.id.0 >= state.next_id || !ids.insert(w.id.0) {
                return Err("Invalid or duplicate saved weapon identifier.");
            }
            if let Some(art) = w.loadout.photon_art {
                if !state.discs.contains(&art) || assigned.contains(&art) {
                    return Err("Invalid saved PA assignment.");
                }
                assigned.push(art);
            }
        }
        for (i, art) in state.discs.iter().enumerate() {
            if state.discs[..i].contains(art) {
                return Err("Duplicate saved PA disc.");
            }
        }
        if !ids.contains(&state.equipped.0) {
            return Err("Saved equipped weapon is not owned.");
        }
        Ok(Self {
            meseta: state.meseta,
            weapons: state.weapons,
            discs: state.discs,
            equipped: state.equipped,
            next_id: state.next_id,
        })
    }
    pub fn meseta(&self) -> u32 {
        self.meseta
    }
    pub(crate) fn award_meseta(&mut self, amount: u32) -> u32 {
        let before = self.meseta;
        self.meseta = self.meseta.saturating_add(amount).min(MAX_MESETA);
        self.meseta - before
    }
    pub(crate) fn buy(&mut self, item: StockItem) -> Result<(), &'static str> {
        if self.meseta < item.price() {
            return Err("Not enough Meseta. Clear a trial to earn more.");
        }
        match item {
            StockItem::Weapon(w) => {
                self.grant_weapon(w)?;
            }
            StockItem::Disc(a) => self.grant_disc(a)?,
        }
        self.meseta -= item.price();
        Ok(())
    }
    pub(crate) fn sell_weapon(&mut self, id: WeaponId) -> Result<u32, &'static str> {
        if id == self.equipped {
            return Err("Equip another weapon before selling this one.");
        }
        let item = self.weapon(id).ok_or("That weapon is not owned.")?;
        let price = StockItem::Weapon(item.loadout.kind).price() / 2;
        let balance = self
            .meseta
            .checked_add(price)
            .filter(|n| *n <= MAX_MESETA)
            .ok_or("Your wallet is full.")?;
        self.weapons.retain(|w| w.id != id);
        self.meseta = balance;
        Ok(price)
    }
    pub(crate) fn sell_disc(&mut self, art: PhotonArt) -> Result<u32, &'static str> {
        if !self.discs.contains(&art) {
            return Err("That PA disc is not owned.");
        }
        let price = StockItem::Disc(art).price() / 2;
        let balance = self
            .meseta
            .checked_add(price)
            .filter(|n| *n <= MAX_MESETA)
            .ok_or("Your wallet is full.")?;
        self.discs.retain(|a| *a != art);
        for weapon in &mut self.weapons {
            if weapon.loadout.photon_art == Some(art) {
                weapon.loadout.photon_art = None;
            }
        }
        self.meseta = balance;
        Ok(price)
    }
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
    fn shop_failures_do_not_charge_or_remove_items() {
        let mut bag = Inventory::default();
        let before = bag.clone();
        assert!(bag.buy(StockItem::Weapon(WeaponKind::PhotonEdge)).is_err());
        assert!(bag.buy(StockItem::Disc(PhotonArt::Sweep)).is_err());
        assert!(bag.sell_weapon(WeaponId(1)).is_err());
        assert!(bag.sell_weapon(WeaponId(999)).is_err());
        assert_eq!(bag, before);
        while bag.item_count() < INVENTORY_CAPACITY {
            bag.grant_weapon(WeaponKind::TrainingSword).unwrap();
        }
        let before = bag.clone();
        assert!(bag
            .buy(StockItem::Weapon(WeaponKind::TrainingSword))
            .is_err());
        assert_eq!(bag, before);
    }
    #[test]
    fn buying_and_selling_keep_disc_and_weapon_ownership_consistent() {
        let mut bag = Inventory::default();
        bag.buy(StockItem::Weapon(WeaponKind::HunterSaber)).unwrap();
        assert_eq!(bag.meseta(), 120);
        assert_eq!(bag.weapon(WeaponId(4)).unwrap().loadout.photon_art, None);
        assert_eq!(bag.sell_weapon(WeaponId(4)).unwrap(), 90);
        assert_eq!(bag.meseta(), 210);
        assert_eq!(bag.sell_disc(PhotonArt::Sweep).unwrap(), 60);
        assert_eq!(bag.weapon(WeaponId(1)).unwrap().loadout.photon_art, None);
        assert!(!bag.discs().contains(&PhotonArt::Sweep));
        bag.buy(StockItem::Disc(PhotonArt::Sweep)).unwrap();
        assert_eq!(bag.meseta(), 150);
        assert_eq!(bag.disc_owner(PhotonArt::Sweep), None);
        bag.assign(WeaponId(2), Some(PhotonArt::Sweep)).unwrap();
        bag.sell_weapon(WeaponId(2)).unwrap();
        assert!(bag.discs().contains(&PhotonArt::Sweep));
        assert_eq!(bag.disc_owner(PhotonArt::Sweep), None);
    }
    #[test]
    fn wallet_limit_does_not_destroy_sale_items() {
        let mut bag = Inventory::default();
        assert_eq!(bag.award_meseta(u32::MAX), MAX_MESETA - STARTING_MESETA);
        assert_eq!(bag.award_meseta(300), 0);
        let before = bag.clone();
        assert!(bag.sell_disc(PhotonArt::Sweep).is_err());
        assert!(bag.sell_weapon(WeaponId(2)).is_err());
        assert_eq!(bag, before);
    }
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
