//! Editable project documents. Runtime and editor use the same validation and settings.
use crate::saves::{atomic_write, CharacterRecord};
use psp_core::{
    character::Character,
    settings::{GameSettings, SceneProp, WeaponBalance},
    world::{Position, World},
    Location,
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
};
const LIMIT: u64 = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponSettings {
    pub power: f32,
    pub reach: f32,
    pub speed: f32,
}
impl From<WeaponBalance> for WeaponSettings {
    fn from(w: WeaponBalance) -> Self {
        Self {
            power: w.power,
            reach: w.reach,
            speed: w.speed,
        }
    }
}
impl WeaponSettings {
    fn balance(&self) -> WeaponBalance {
        WeaponBalance {
            power: self.power,
            reach: self.reach,
            speed: self.speed,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gameplay {
    pub walk_speed: f32,
    pub sprint_speed: f32,
    pub jump_speed: f32,
    pub gravity: f32,
    pub pp_recovery: f32,
    pub enemy_speed: f32,
    pub enemy_damage: f32,
    pub animation_energy: f32,
    pub sword: WeaponSettings,
    pub partisan: WeaponSettings,
    pub knuckles: WeaponSettings,
}
impl Default for Gameplay {
    fn default() -> Self {
        let s = GameSettings::default();
        Self {
            walk_speed: s.walk_speed,
            sprint_speed: s.sprint_speed,
            jump_speed: s.jump_speed,
            gravity: s.gravity,
            pp_recovery: s.pp_recovery,
            enemy_speed: s.enemy_speed,
            enemy_damage: s.enemy_damage,
            animation_energy: s.animation_energy,
            sword: s.sword.into(),
            partisan: s.partisan.into(),
            knuckles: s.knuckles.into(),
        }
    }
}
impl Gameplay {
    pub fn settings(&self) -> GameSettings {
        GameSettings {
            walk_speed: self.walk_speed,
            sprint_speed: self.sprint_speed,
            jump_speed: self.jump_speed,
            gravity: self.gravity,
            pp_recovery: self.pp_recovery,
            enemy_speed: self.enemy_speed,
            enemy_damage: self.enemy_damage,
            animation_energy: self.animation_energy,
            sword: self.sword.balance(),
            partisan: self.partisan.balance(),
            knuckles: self.knuckles.balance(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prop {
    pub scene: String,
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub color: [u8; 3],
}
impl Prop {
    pub fn core(&self) -> Result<SceneProp, String> {
        let location = match self.scene.as_str() {
            "lobby" => Location::Lobby,
            "field" => Location::TrainingField,
            _ => return Err("Prop scene must be lobby or field.".into()),
        };
        let p = SceneProp {
            location,
            center: Position {
                x: self.x,
                z: self.z,
            },
            size: [self.width, self.height, self.depth],
            color: self.color,
        };
        p.validate().map_err(str::to_owned)?;
        Ok(p)
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub character: CharacterRecord,
    pub gameplay: Gameplay,
    pub props: Vec<Prop>,
}
impl Default for Project {
    fn default() -> Self {
        Self {
            version: 1,
            name: "Hunter Training".into(),
            character: CharacterRecord::from(&Character::default()),
            gameplay: Gameplay::default(),
            props: Vec::new(),
        }
    }
}
impl Project {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported project version.".into());
        }
        if self.name.trim().is_empty()
            || self.name.len() > 60
            || self.name.chars().any(char::is_control)
        {
            return Err("Project name must contain 1-60 printable characters.".into());
        }
        self.character.character()?;
        self.gameplay.settings().validate().map_err(str::to_owned)?;
        if self.props.len() > 32 {
            return Err("Project supports at most 32 props.".into());
        }
        for p in &self.props {
            p.core()?;
        }
        Ok(())
    }
    pub fn apply(&self, world: &mut World) -> Result<(), String> {
        self.validate()?;
        world
            .configure(
                self.gameplay.settings(),
                self.props
                    .iter()
                    .map(Prop::core)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map_err(str::to_owned)
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let mut bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() as u64 > LIMIT {
            return Err("Project exceeds 512 KiB.".into());
        }
        let project: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        project.validate()?;
        Ok(project)
    }
}
fn read(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    if file.metadata().map_err(|e| e.to_string())?.len() > LIMIT {
        return Err("Project exceeds 512 KiB.".into());
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("Project exceeds 512 KiB.".into());
    }
    Ok(Some(bytes))
}
pub struct ProjectStore {
    pub path: PathBuf,
    known_bytes: Option<Vec<u8>>,
    pub saved: Project,
}
impl ProjectStore {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let known_bytes = read(&path)?;
        let saved = known_bytes
            .as_deref()
            .map(Project::decode)
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            path,
            known_bytes,
            saved,
        })
    }
    pub fn save(&mut self, project: &Project) -> Result<(), String> {
        let bytes = project.encode()?;
        if read(&self.path)? != self.known_bytes {
            return Err("Project changed on disk. Reload before saving; your edits are still in the editor.".into());
        }
        if self.known_bytes.as_ref() == Some(&bytes) {
            return Ok(());
        }
        if let Some(previous) = &self.known_bytes {
            atomic_write(&self.path.with_extension("json.bak"), previous)?;
        }
        atomic_write(&self.path, &bytes)?;
        self.known_bytes = Some(bytes);
        self.saved = project.clone();
        Ok(())
    }
    pub fn reload(&mut self) -> Result<Project, String> {
        let bytes = read(&self.path)?.ok_or("Project file does not exist. Save to create it.")?;
        let project = Project::decode(&bytes)?;
        self.known_bytes = Some(bytes);
        self.saved = project.clone();
        Ok(project)
    }
}
pub fn default_path() -> PathBuf {
    let relative = Path::new("projects/starter/project.json");
    if let Some(exe) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        let next_to_exe = exe.join(relative);
        if next_to_exe.exists() {
            return next_to_exe;
        }
    }
    if relative.exists() {
        return relative.to_path_buf();
    }
    std::env::current_dir().unwrap_or_default().join(relative)
}
#[derive(Default)]
pub struct LaunchOptions {
    pub project: Option<PathBuf>,
    pub playtest: bool,
    pub model: Option<PathBuf>,
    pub motion: Option<PathBuf>,
}
impl LaunchOptions {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut result = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            if arg == "--project" {
                if result.project.is_some() {
                    return Err("Use --project once.".into());
                }
                result.project = Some(
                    args.next()
                        .filter(|a| !a.is_empty())
                        .ok_or("--project requires a JSON path.")?
                        .into(),
                );
            } else if arg == "--playtest" {
                result.playtest = true;
            } else if arg.to_string_lossy().starts_with('-') {
                return Err(format!("Unknown option: {}", arg.to_string_lossy()));
            } else if result.model.is_none() {
                result.model = Some(arg.into());
            } else if result.motion.is_none() {
                result.motion = Some(arg.into());
            } else {
                return Err("Expected at most one model and one motion archive.".into());
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "psp-project-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn path(&self) -> PathBuf {
            self.0.join("project.json")
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn starter_document_matches_runtime_defaults_and_project_round_trips() {
        let p = Project::decode(include_bytes!("../../../projects/starter/project.json")).unwrap();
        assert_eq!(p, Project::default());
        let mut world = World::default();
        let before = world.clone();
        p.apply(&mut world).unwrap();
        assert_eq!(world, before);
        let mut p = p;
        p.character.head = 2;
        p.character.name = "Astra".into();
        p.gameplay.sword.reach = 1.4;
        p.props.push(Prop {
            scene: "field".into(),
            x: 9.0,
            z: 0.0,
            width: 1.5,
            height: 2.0,
            depth: 1.0,
            color: [20, 100, 170],
        });
        assert_eq!(Project::decode(&p.encode().unwrap()).unwrap(), p);
        p.apply(&mut world).unwrap();
        assert_eq!(world.settings().sword.reach, 1.4);
        assert_eq!(world.props().len(), 1);
    }
    #[test]
    fn invalid_projects_preserve_runtime_and_are_not_written() {
        let temp = Temp::new();
        let mut store = ProjectStore::open(temp.path()).unwrap();
        let mut world = World::default();
        for case in 0..7 {
            let mut p = Project::default();
            match case {
                0 => p.version = 99,
                1 => p.gameplay.gravity = f32::NAN,
                2 => p.gameplay.sword.speed = 0.0,
                3 => p.character.head = 7,
                4 => p.gameplay.sprint_speed = 2.0,
                5 => p.props.push(Prop {
                    scene: "field".into(),
                    x: 0.0,
                    z: -4.0,
                    width: 1.0,
                    height: 1.0,
                    depth: 1.0,
                    color: [0; 3],
                }),
                _ => p.props.push(Prop {
                    scene: "unknown".into(),
                    x: 9.0,
                    z: 0.0,
                    width: 1.0,
                    height: 1.0,
                    depth: 1.0,
                    color: [0; 3],
                }),
            }
            let before = world.clone();
            assert!(p.apply(&mut world).is_err());
            assert_eq!(world, before);
            assert!(store.save(&p).is_err());
            assert!(!temp.path().exists());
        }
        assert!(Project::decode(&vec![0; LIMIT as usize + 1]).is_err());
        assert!(Project::decode(br#"{"version":1,"unrecognized":true}"#).is_err());
    }
    #[test]
    fn project_writes_keep_backups_and_reject_external_edits() {
        let temp = Temp::new();
        let mut store = ProjectStore::open(temp.path()).unwrap();
        let original = Project::default();
        store.save(&original).unwrap();
        let previous = fs::read(temp.path()).unwrap();
        let mut edited = original.clone();
        edited.gameplay.walk_speed = 4.8;
        store.save(&edited).unwrap();
        assert_eq!(
            fs::read(temp.path().with_extension("json.bak")).unwrap(),
            previous
        );
        assert_eq!(store.reload().unwrap(), edited);
        fs::write(temp.path(), b"external manual edit").unwrap();
        assert!(store.save(&original).is_err());
        assert_eq!(fs::read(temp.path()).unwrap(), b"external manual edit");
        assert!(store.reload().is_err());
        assert_eq!(store.saved, edited);
    }
    #[test]
    fn launch_arguments_keep_project_paths_separate_from_models() {
        let options = LaunchOptions::parse(
            [
                "--project",
                "project folder/project.json",
                "--playtest",
                "dagan.ice",
                "motions.ice",
            ]
            .map(OsString::from),
        )
        .unwrap();
        assert_eq!(
            options.project,
            Some(PathBuf::from("project folder/project.json"))
        );
        assert!(options.playtest);
        assert_eq!(options.model, Some("dagan.ice".into()));
        assert_eq!(options.motion, Some("motions.ice".into()));
        assert!(LaunchOptions::parse(["--project"].map(OsString::from)).is_err());
        assert!(LaunchOptions::parse(["--typo"].map(OsString::from)).is_err());
        assert!(LaunchOptions::parse(["a", "b", "c"].map(OsString::from)).is_err());
    }
}
