//! Versioned, bounded character saves. Invalid existing files are never overwritten.
use psp_core::{
    inventory::{InventoryState, WeaponId, WeaponItem},
    profile::Profile,
    weapon::{PhotonArt, WeaponKind, WeaponLoadout},
    world::World,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const LIMIT: u64 = 64 * 1024;
static SERIAL: AtomicU64 = AtomicU64::new(0);
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    level: u32,
    experience: u32,
    meseta: u32,
    equipped_weapon: u32,
    next_weapon_id: u32,
    weapons: Vec<WeaponRecord>,
    discs: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WeaponRecord {
    id: u32,
    kind: String,
    art: Option<String>,
}

fn kind_name(kind: WeaponKind) -> &'static str {
    match kind {
        WeaponKind::TrainingSword => "training_sword",
        WeaponKind::HunterSaber => "hunter_saber",
        WeaponKind::PhotonEdge => "photon_edge",
        WeaponKind::RecruitPartisan => "recruit_partisan",
        WeaponKind::RecruitKnuckles => "recruit_knuckles",
    }
}
fn parse_kind(name: &str) -> Result<WeaponKind, String> {
    match name {
        "training_sword" => Ok(WeaponKind::TrainingSword),
        "hunter_saber" => Ok(WeaponKind::HunterSaber),
        "photon_edge" => Ok(WeaponKind::PhotonEdge),
        "recruit_partisan" => Ok(WeaponKind::RecruitPartisan),
        "recruit_knuckles" => Ok(WeaponKind::RecruitKnuckles),
        _ => Err("Unknown saved weapon kind.".into()),
    }
}
fn art_name(art: PhotonArt) -> &'static str {
    match art {
        PhotonArt::Sweep => "sweep",
        PhotonArt::RisingCleave => "rising_cleave",
        PhotonArt::PiercingThrust => "piercing_thrust",
        PhotonArt::AssaultThrust => "assault_thrust",
        PhotonArt::MeteorFist => "meteor_fist",
    }
}
fn parse_art(name: &str) -> Result<PhotonArt, String> {
    match name {
        "sweep" => Ok(PhotonArt::Sweep),
        "rising_cleave" => Ok(PhotonArt::RisingCleave),
        "piercing_thrust" => Ok(PhotonArt::PiercingThrust),
        "assault_thrust" => Ok(PhotonArt::AssaultThrust),
        "meteor_fist" => Ok(PhotonArt::MeteorFist),
        _ => Err("Unknown saved Photon Art.".into()),
    }
}

pub fn encode(profile: &Profile) -> Result<Vec<u8>, String> {
    World::from_profile(profile.clone()).map_err(str::to_owned)?;
    let record = Record {
        version: 1,
        level: profile.level,
        experience: profile.experience,
        meseta: profile.inventory.meseta,
        equipped_weapon: profile.inventory.equipped.0,
        next_weapon_id: profile.inventory.next_id,
        weapons: profile
            .inventory
            .weapons
            .iter()
            .map(|w| WeaponRecord {
                id: w.id.0,
                kind: kind_name(w.loadout.kind).into(),
                art: w.loadout.photon_art.map(|a| art_name(a).into()),
            })
            .collect(),
        discs: profile
            .inventory
            .discs
            .iter()
            .map(|a| art_name(*a).into())
            .collect(),
    };
    let mut bytes = serde_json::to_vec_pretty(&record).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}
pub fn decode(bytes: &[u8]) -> Result<Profile, String> {
    if bytes.len() as u64 > LIMIT {
        return Err("Save exceeds the 64 KiB limit.".into());
    }
    let record: Record = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if record.version != 1 {
        return Err("Unsupported save version.".into());
    }
    let mut weapons = Vec::new();
    for w in record.weapons {
        weapons.push(WeaponItem {
            id: WeaponId(w.id),
            loadout: WeaponLoadout {
                kind: parse_kind(&w.kind)?,
                photon_art: w.art.as_deref().map(parse_art).transpose()?,
            },
        });
    }
    let discs = record
        .discs
        .iter()
        .map(|a| parse_art(a))
        .collect::<Result<Vec<_>, _>>()?;
    let profile = Profile {
        level: record.level,
        experience: record.experience,
        inventory: InventoryState {
            meseta: record.meseta,
            weapons,
            discs,
            equipped: WeaponId(record.equipped_weapon),
            next_id: record.next_weapon_id,
        },
    };
    World::from_profile(profile.clone()).map_err(str::to_owned)?;
    Ok(profile)
}
fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    if file.metadata().map_err(|e| e.to_string())?.len() > LIMIT {
        return Err("Save exceeds the 64 KiB limit.".into());
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("Save exceeds the 64 KiB limit.".into());
    }
    Ok(Some(bytes))
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(
        ".psp-save-{}-{}.tmp",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map_err(|e| e.to_string())
}

pub struct SaveStore {
    path: Option<PathBuf>,
    known_bytes: Option<Vec<u8>>,
    last_saved: Option<Profile>,
    blocked: bool,
    dirty_seconds: f32,
    pub status: String,
}
impl SaveStore {
    pub fn open(path: PathBuf) -> (Self, World) {
        let mut store = Self {
            path: Some(path),
            known_bytes: None,
            last_saved: None,
            blocked: false,
            dirty_seconds: 0.0,
            status: "Autosave ready".into(),
        };
        match read_optional(store.path.as_ref().unwrap()) {
            Ok(Some(bytes)) => match decode(&bytes) {
                Ok(profile) => {
                    store.known_bytes = Some(bytes);
                    store.last_saved = Some(profile.clone());
                    store.status = "Character loaded / autosave ready".into();
                    return (store, World::from_profile(profile).unwrap());
                }
                Err(e) => store.block(&e),
            },
            Ok(None) => {}
            Err(e) => store.block(&e),
        }
        (store, World::default())
    }
    pub fn disabled() -> Self {
        Self {
            path: None,
            known_bytes: None,
            last_saved: None,
            blocked: false,
            dirty_seconds: 0.0,
            status: "Preview session / saving disabled".into(),
        }
    }
    pub fn unavailable(reason: &str) -> Self {
        let mut store = Self::disabled();
        store.block(reason);
        store
    }
    fn block(&mut self, error: &str) {
        self.blocked = true;
        self.status = "Save unavailable / existing data preserved".into();
        eprintln!("Save unavailable: {error}");
    }
    fn write(&mut self, profile: &Profile) -> Result<(), String> {
        if self.blocked {
            return Err(self.status.clone());
        }
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        if self.last_saved.as_ref() == Some(profile) {
            return Ok(());
        }
        let bytes = encode(profile)?;
        if read_optional(path)? != self.known_bytes {
            self.blocked = true;
            return Err("Save changed outside this game / reopen to load it".into());
        }
        if let Some(previous) = &self.known_bytes {
            atomic_write(&path.with_extension("json.bak"), previous)?;
        }
        atomic_write(path, &bytes)?;
        self.known_bytes = Some(bytes);
        self.last_saved = Some(profile.clone());
        self.status = "Progress saved".into();
        Ok(())
    }
    pub fn flush(&mut self, profile: &Profile) {
        if let Err(e) = self.write(profile) {
            self.status = if self.blocked {
                "Save unavailable / existing data preserved"
            } else {
                "Save failed / retry with F5"
            }
            .into();
            eprintln!("Save failed: {e}");
        } else if self.enabled() {
            self.status = "Progress saved".into();
        }
        self.dirty_seconds = 0.0;
    }
    pub fn tick(&mut self, profile: &Profile, delta: f32) {
        if self.path.is_none() || self.blocked || self.last_saved.as_ref() == Some(profile) {
            self.dirty_seconds = 0.0;
            return;
        }
        self.dirty_seconds += delta;
        if self.dirty_seconds >= 1.0 {
            self.flush(profile);
        }
    }
    pub fn enabled(&self) -> bool {
        self.path.is_some() && !self.blocked
    }
}

pub fn default_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PSP_SAVE_PATH") {
        return Some(PathBuf::from(path));
    }
    if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("PhantasyStarPortableRust/profile.json"))
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")))
            .map(|p| p.join("phantasy-star-portable-rust/profile.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "psp-save-test-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn path(&self) -> PathBuf {
            self.0.join("profile.json")
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn round_trip_preserves_owned_gear_disc_slots_and_progression() {
        let mut world = World::default();
        world.equip_weapon(WeaponId(3)).unwrap();
        world.equip_photon_art(Some(PhotonArt::Sweep)).unwrap();
        let mut p = world.profile();
        p.level = 9;
        p.experience = 270;
        p.inventory.meseta = 77;
        assert_eq!(decode(&encode(&p).unwrap()).unwrap(), p);
        assert_eq!(
            World::from_profile(p).unwrap().session().location(),
            psp_core::Location::Lobby
        );
    }
    #[test]
    fn version_one_sword_saves_and_new_class_loadouts_round_trip() {
        let legacy = World::default().profile();
        assert_eq!(decode(&encode(&legacy).unwrap()).unwrap(), legacy);
        let mut profile = legacy;
        let mut bag = psp_core::inventory::Inventory::from_state(profile.inventory).unwrap();
        for (kind, art) in [
            (WeaponKind::RecruitPartisan, PhotonArt::AssaultThrust),
            (WeaponKind::RecruitKnuckles, PhotonArt::MeteorFist),
        ] {
            let id = bag.grant_weapon(kind).unwrap();
            bag.grant_disc(art).unwrap();
            bag.assign(id, Some(art)).unwrap();
            bag.equip(id).unwrap();
        }
        profile.inventory = bag.snapshot();
        assert_eq!(decode(&encode(&profile).unwrap()).unwrap(), profile);
        // Even well-formed JSON must reject cross-class slots.
        let invalid = String::from_utf8(encode(&profile).unwrap())
            .unwrap()
            .replace(
                "\"kind\": \"recruit_knuckles\"",
                "\"kind\": \"recruit_partisan\"",
            );
        assert!(decode(invalid.as_bytes()).is_err());
    }
    #[test]
    fn malformed_future_oversized_and_unknown_items_are_rejected() {
        let bytes = encode(&World::default().profile()).unwrap();
        for input in [
            b"{".to_vec(),
            vec![0; LIMIT as usize + 1],
            String::from_utf8(bytes.clone())
                .unwrap()
                .replace("\"version\": 1", "\"version\": 2")
                .into_bytes(),
            String::from_utf8(bytes)
                .unwrap()
                .replace("training_sword", "unknown_weapon")
                .into_bytes(),
        ] {
            assert!(decode(&input).is_err());
        }
    }
    #[test]
    fn autosave_reloads_and_retains_the_previous_valid_profile_as_backup() {
        let temp = Temp::new();
        let (mut store, world) = SaveStore::open(temp.path());
        let p = world.profile();
        store.flush(&p);
        let first = fs::read(temp.path()).unwrap();
        let mut changed = p.clone();
        changed.inventory.meseta = 99;
        changed.level = 2;
        store.tick(&changed, 0.5);
        assert_eq!(fs::read(temp.path()).unwrap(), first);
        store.tick(&changed, 0.5);
        assert_eq!(
            fs::read(temp.path().with_extension("json.bak")).unwrap(),
            first
        );
        let (store, loaded) = SaveStore::open(temp.path());
        assert!(store.enabled());
        assert_eq!(loaded.profile(), changed);
    }
    #[test]
    fn invalid_or_external_saves_are_not_overwritten() {
        let temp = Temp::new();
        fs::write(temp.path(), b"broken profile").unwrap();
        let (mut store, world) = SaveStore::open(temp.path());
        assert!(!store.enabled());
        store.flush(&world.profile());
        assert_eq!(fs::read(temp.path()).unwrap(), b"broken profile");
        fs::remove_file(temp.path()).unwrap();
        let (mut store, world) = SaveStore::open(temp.path());
        store.flush(&world.profile());
        let external = encode(&{
            let mut p = world.profile();
            p.inventory.meseta = 42;
            p
        })
        .unwrap();
        fs::write(temp.path(), &external).unwrap();
        let mut p = world.profile();
        p.inventory.meseta = 100;
        store.flush(&p);
        assert_eq!(fs::read(temp.path()).unwrap(), external);
        assert!(!store.enabled());
    }
    #[test]
    fn failed_write_preserves_primary_and_disabled_runs_write_nothing() {
        let temp = Temp::new();
        let (mut store, world) = SaveStore::open(temp.path());
        store.flush(&world.profile());
        let before = fs::read(temp.path()).unwrap();
        fs::create_dir(temp.path().with_extension("json.bak")).unwrap();
        let mut p = world.profile();
        p.inventory.meseta = 10;
        store.flush(&p);
        assert_eq!(fs::read(temp.path()).unwrap(), before);
        assert!(store.status.starts_with("Save failed"));
        let mut disabled = SaveStore::disabled();
        disabled.flush(&p);
        assert!(!disabled.enabled());
    }
}
