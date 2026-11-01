//! One-way UE 5.7.4 migration kit. Original assets only; no proprietary archives.
use crate::{
    hunter::{self, Hunter},
    monster,
    presentation::Environment,
    project::Project,
};
use macroquad::prelude::{Mesh, Vec3};
use psp_assets::{
    rig::{Clip, Rig},
    Model,
};
use psp_core::{
    weapon::{PhotonArt, WeaponClass},
    world::{FIELD_OBSTACLES, LOBBY_OBSTACLES},
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Default)]
struct Glb {
    bytes: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}
impl Glb {
    fn view(&mut self, data: &[u8]) -> usize {
        while self.bytes.len() % 4 != 0 {
            self.bytes.push(0);
        }
        let offset = self.bytes.len();
        self.bytes.extend_from_slice(data);
        let index = self.views.len();
        self.views
            .push(json!({"buffer":0,"byteOffset":offset,"byteLength":data.len()}));
        index
    }
    fn floats(&mut self, values: &[f32], components: usize, kind: &str, bounds: bool) -> usize {
        assert!(
            !values.is_empty()
                && values.len() % components == 0
                && values.iter().all(|v| v.is_finite())
        );
        let data: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let view = self.view(&data);
        let mut a = json!({"bufferView":view,"componentType":5126,"count":values.len()/components,"type":kind});
        if bounds {
            let mut min = vec![f32::INFINITY; components];
            let mut max = vec![f32::NEG_INFINITY; components];
            for row in values.chunks_exact(components) {
                for i in 0..components {
                    min[i] = min[i].min(row[i]);
                    max[i] = max[i].max(row[i]);
                }
            }
            a["min"] = json!(min);
            a["max"] = json!(max);
        }
        let id = self.accessors.len();
        self.accessors.push(a);
        id
    }
    fn joints(&mut self, values: &[u16]) -> usize {
        let data: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let view = self.view(&data);
        let id = self.accessors.len();
        self.accessors.push(
            json!({"bufferView":view,"componentType":5123,"count":values.len()/4,"type":"VEC4"}),
        );
        id
    }
    fn colors(&mut self, values: &[u8]) -> usize {
        let view = self.view(values);
        let id = self.accessors.len();
        self.accessors.push(json!({"bufferView":view,"componentType":5121,"normalized":true,"count":values.len()/4,"type":"VEC4"}));
        id
    }
    fn vertex_targets(&mut self, ids: &[usize]) {
        for id in ids {
            let view = self.accessors[*id]["bufferView"].as_u64().unwrap() as usize;
            self.views[view]["target"] = json!(34962);
        }
    }
    fn finish(mut self, mut doc: Value) -> Result<Vec<u8>, String> {
        doc["asset"] =
            json!({"version":"2.0","generator":"Phantasy Star Rust original-asset exporter"});
        doc["buffers"] = json!([{"byteLength":self.bytes.len()}]);
        doc["bufferViews"] = json!(self.views);
        doc["accessors"] = json!(self.accessors);
        let mut text = serde_json::to_vec(&doc).map_err(|e| e.to_string())?;
        while text.len() % 4 != 0 {
            text.push(b' ');
        }
        while self.bytes.len() % 4 != 0 {
            self.bytes.push(0);
        }
        let length =
            u32::try_from(28 + text.len() + self.bytes.len()).map_err(|_| "GLB exceeds 4 GiB.")?;
        let mut out = Vec::new();
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&(text.len() as u32).to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend(text);
        out.extend_from_slice(&(self.bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(b"BIN\0");
        out.extend(self.bytes);
        Ok(out)
    }
}
fn material(name: &str, color: [f32; 3]) -> Value {
    json!({"name":name,"pbrMetallicRoughness":{"baseColorFactor":[color[0],color[1],color[2],1.0],"metallicFactor":0.0,"roughnessFactor":0.85}})
}
fn rigged(
    model: &Model,
    rig: &Rig,
    materials: Vec<Value>,
    triangle_materials: &[usize],
    clips: &[(&Clip, f32)],
    root_scale: [f32; 3],
    name: &str,
) -> Result<Vec<u8>, String> {
    let mut b = Glb::default();
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, material) in triangle_materials.iter().enumerate() {
        groups.entry(*material).or_default().push(i);
    }
    let materials: Vec<_> = groups.keys().map(|i| materials[*i].clone()).collect();
    let mut primitives = Vec::new();
    for (material, (_, triangles)) in groups.into_iter().enumerate() {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut joints = Vec::new();
        let mut weights = Vec::new();
        for i in triangles {
            let triangle = model.triangles[i];
            let [a, c, d] = triangle.map(|i| Vec3::from_array(model.positions[i]));
            let normal = (c - a).cross(d - a).normalize_or_zero();
            for index in triangle {
                positions.extend(model.positions[index]);
                normals.extend(normal.to_array());
                joints.extend(rig.skin[index].joints);
                weights.extend(rig.skin[index].weights);
            }
        }
        let p = b.floats(&positions, 3, "VEC3", true);
        let n = b.floats(&normals, 3, "VEC3", false);
        let j = b.joints(&joints);
        let w = b.floats(&weights, 4, "VEC4", false);
        b.vertex_targets(&[p, n, j, w]);
        primitives.push(json!({"attributes":{"POSITION":p,"NORMAL":n,"JOINTS_0":j,"WEIGHTS_0":w},"material":material,"mode":4}));
    }
    let skeleton = &rig.skeleton;
    let mut nodes = vec![json!({"name":"CharacterAppearanceRoot","scale":root_scale})];
    for bone in &skeleton.bones {
        nodes.push(json!({"name":bone.name,"translation":bone.local_bind.translation.to_array(),"rotation":bone.local_bind.rotation.normalize().to_array(),"scale":bone.local_bind.scale.to_array()}));
    }
    let mesh_node = nodes.len();
    nodes.push(json!({"name":name,"mesh":0,"skin":0}));
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); mesh_node];

    for (i, bone) in skeleton.bones.iter().enumerate() {
        children[bone.parent.map_or(0, |p| p + 1)].push(i + 1);
    }
    for (i, child) in children.into_iter().enumerate() {
        if !child.is_empty() {
            nodes[i]["children"] = json!(child);
        }
    }
    let inverse: Vec<f32> = skeleton
        .bones
        .iter()
        .flat_map(|bone| bone.inverse_bind.to_cols_array())
        .collect();
    let ibm = b.floats(&inverse, 16, "MAT4", false);
    let mut animations = Vec::new();
    for (clip, speed) in clips {
        let mut samplers = Vec::new();
        let mut channels = Vec::new();
        for track in &clip.tracks {
            for (path, channel, components, kind) in [
                ("translation", &track.translation, 3, "VEC3"),
                ("rotation", &track.rotation, 4, "VEC4"),
                ("scale", &track.scale, 3, "VEC3"),
            ] {
                if let Some(channel) = channel {
                    let times: Vec<f32> = channel.times.iter().map(|t| t / speed).collect();
                    let values: Vec<f32> = channel
                        .values
                        .iter()
                        .flat_map(|v| {
                            let v = if path == "rotation" {
                                macroquad::prelude::Quat::from_array(*v)
                                    .normalize()
                                    .to_array()
                            } else {
                                *v
                            };
                            v[..components].to_vec()
                        })
                        .collect();
                    let input = b.floats(&times, 1, "SCALAR", true);
                    let output = b.floats(&values, components, kind, false);
                    let sampler = samplers.len();
                    samplers.push(json!({"input":input,"output":output,"interpolation":"LINEAR"}));
                    channels.push(
                        json!({"sampler":sampler,"target":{"node":track.bone+1,"path":path}}),
                    );
                }
            }
        }
        animations.push(
            json!({"name":clip.name.replace(' ',"_"),"samplers":samplers,"channels":channels}),
        );
    }
    let mut doc = json!({"scene":0,"scenes":[{"nodes":[0,mesh_node]}],"nodes":nodes,"meshes":[{"name":name,"primitives":primitives}],"materials":materials,
        "skins":[{"name":format!("{name}_Skeleton"),"inverseBindMatrices":ibm,"skeleton":1,"joints":(1..=skeleton.bones.len()).collect::<Vec<_>>()}]});
    if !animations.is_empty() {
        doc["animations"] = json!(animations);
    }
    b.finish(doc)
}
fn hunter_asset(
    project: &Project,
    class: WeaponClass,
    name: &str,
) -> Result<(Vec<u8>, Vec<Value>), String> {
    let character = project.character.character()?;
    let mut h = Hunter::default();
    h.preview(&character, class, 0, 0.0, project.gameplay.animation_energy);
    let rig = h.scene.rig.as_ref().unwrap();
    let model = &h.scene.model;
    let colors = [
        hunter::armor_color(character.armor),
        [0.09, 0.14, 0.2],
        hunter::accent_color(character.accent),
        [0.6, 0.72, 0.8],
        hunter::skin_color(character.skin),
        hunter::hair_color(character.hair),
    ];
    let mut materials: Vec<_> = model
        .material_names
        .iter()
        .zip(colors)
        .map(|(n, c)| material(n, c))
        .collect();
    let weapon_color = match class {
        WeaponClass::Sword => [0.3, 0.95, 1.0],
        WeaponClass::Partisan => [0.25, 1.0, 0.6],
        WeaponClass::Knuckles => [1.0, 0.5, 0.2],
    };
    let weapon = materials.len();
    materials.push(material("WeaponPhoton", weapon_color));
    let assigned: Vec<_> = model
        .triangles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let m = model.triangle_materials[i].unwrap_or(0);
            let bone = rig.skin[t[0]].joints[0];
            if (bone == 16 || class == WeaponClass::Knuckles && bone == 6) && m == 2 {
                weapon
            } else {
                m
            }
        })
        .collect();
    let indices: &[usize] = match class {
        WeaponClass::Sword => &[0, 1, 2, 3, 4, 5, 6, 7, 8],
        WeaponClass::Partisan => &[0, 1, 2, 9, 10, 11, 16],
        WeaponClass::Knuckles => &[0, 1, 2, 12, 13, 14, 15],
    };
    let speed = project.gameplay.settings().weapon(class).speed;
    let clips: Vec<_> = indices
        .iter()
        .map(|i| (&h.clips()[*i], if *i >= 3 { speed } else { 1.0 }))
        .collect();
    let metadata = clips
        .iter()
        .map(|(c, s)| json!({"name":c.name.replace(' ',"_"),"duration_seconds":c.duration/s}))
        .collect();
    let asset = rigged(
        model,
        rig,
        materials,
        &assigned,
        &clips,
        hunter::body_scale(character.build).to_array(),
        name,
    )?;
    Ok((asset, metadata))
}
fn environment_asset(batches: &[Mesh], name: &str) -> Result<Vec<u8>, String> {
    let mut b = Glb::default();
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    for mesh in batches {
        for triangle in mesh.indices.chunks_exact(3) {
            let v = [triangle[0], triangle[1], triangle[2]].map(|i| mesh.vertices[i as usize]);
            let [a, c, d] = v.map(|v| v.position);
            let normal = (c - a).cross(d - a).normalize_or_zero();
            for vertex in v {
                positions.extend(vertex.position.to_array());
                normals.extend(normal.to_array());
                colors.extend(vertex.color);
            }
        }
    }
    let p = b.floats(&positions, 3, "VEC3", true);
    let n = b.floats(&normals, 3, "VEC3", false);
    let c = b.colors(&colors);
    b.vertex_targets(&[p, n, c]);
    b.finish(json!({"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"name":name,"mesh":0}],"meshes":[{"name":name,"primitives":[{"attributes":{"POSITION":p,"NORMAL":n,"COLOR_0":c},"material":0,"mode":4}]}],"materials":[material("Scenery_VertexColor",[1.0;3])]}))
}
/// GLB uses the game's right-handed Y-up metres. The UE glTF translator handles
/// the basis/units; explicitly authored UE primitives use this corresponding basis.
pub fn unreal_position(position: [f32; 3]) -> [f32; 3] {
    [
        position[0] * 100.0,
        -position[2] * 100.0,
        position[1] * 100.0,
    ]
}
fn box_record(label: &str, center: [f32; 3], size: [f32; 3], color: [u8; 3]) -> Value {
    json!({"label":label,"location_cm":unreal_position(center),"size_cm":[size[0]*100.0,size[2]*100.0,size[1]*100.0],"color":color})
}
fn scene_records(project: &Project, lobby: bool) -> Value {
    let mut boxes = Vec::new();
    for (i, o) in if lobby {
        &LOBBY_OBSTACLES[..]
    } else {
        &FIELD_OBSTACLES[..]
    }
    .iter()
    .enumerate()
    {
        let height = if lobby && i < 2 {
            3.0
        } else if lobby {
            0.9
        } else {
            1.8
        };
        boxes.push(box_record(
            &format!("BaseObstacle_{i}"),
            [o.center.x, height * 0.5, o.center.z],
            [o.half.x * 2.0, height, o.half.z * 2.0],
            [55, 78, 86],
        ));
    }
    for (i, p) in project
        .props
        .iter()
        .enumerate()
        .filter(|(_, p)| p.scene == if lobby { "lobby" } else { "field" })
    {
        boxes.push(box_record(
            &format!("ProjectProp_{i}"),
            [p.x, p.height * 0.5, p.z],
            [p.width, p.height, p.depth],
            p.color,
        ));
    }
    let half = if lobby { 12.0 } else { 15.0 };
    for (i, (center, size)) in [
        ([half + 0.2, 0.65, 0.0], [0.4, 1.3, half * 2.0]),
        ([-half - 0.2, 0.65, 0.0], [0.4, 1.3, half * 2.0]),
        ([0.0, 0.65, half + 0.2], [half * 2.0, 1.3, 0.4]),
        ([0.0, 0.65, -half - 0.2], [half * 2.0, 1.3, 0.4]),
    ]
    .into_iter()
    .enumerate()
    {
        boxes.push(box_record(
            &format!("Boundary_{i}"),
            center,
            size,
            [35, 62, 77],
        ));
    }
    let markers = if lobby {
        vec![
            json!({"label":"MissionTerminal_PORT_TO_BLUEPRINT","location_cm":unreal_position([0.0,0.0,-7.5])}),
            json!({"label":"MiraShop_PORT_TO_BLUEPRINT","location_cm":unreal_position([6.0,0.0,5.0])}),
        ]
    } else {
        vec![
            json!({"label":"TrialEnemy_PORT_TO_BLUEPRINT","location_cm":unreal_position([0.0,0.0,-4.0])}),
            json!({"label":"ReturnPad_PORT_TO_BLUEPRINT","location_cm":unreal_position([0.0,0.0,9.0])}),
        ]
    };
    json!({"name":if lobby{"Lobby"}else{"Field"},"environment":if lobby{"LobbyScenery"}else{"FieldScenery"},"boxes":boxes,"markers":markers,"player_start_cm":unreal_position([0.0,1.0,5.5])})
}
fn balance_data(project: &Project) -> Value {
    let settings = project.gameplay.settings();
    let classes=[WeaponClass::Sword,WeaponClass::Partisan,WeaponClass::Knuckles].into_iter().map(|class|{
        let balance=settings.weapon(class);
        let normals:Vec<_>=(1..=3).map(|stage|{let spec=class.attack(stage);json!({"stage":stage,"duration":spec.duration/balance.speed,"hit_at":spec.hit_at/balance.speed,"range_cm":spec.range*balance.reach*100.0,"alignment":spec.alignment,"power_percent":spec.power_percent})}).collect();
        json!({"class":class.name(),"power_multiplier":balance.power,"normal_attacks":normals})
    }).collect::<Vec<_>>();
    let arts=PhotonArt::ALL.into_iter().map(|art|{let s=art.spec();let b=settings.weapon(art.class());json!({"name":s.name,"class":art.class().name(),"pp":s.pp,"duration":s.duration/b.speed,"hit_at":s.hit_at/b.speed,"range_cm":s.range*b.reach*100.0,"alignment":s.alignment,"power_percent":s.power_percent})}).collect::<Vec<_>>();
    json!({"note":"Reference data for Blueprint/C++ port; not executable Unreal gameplay.","walk_speed_cm_s":settings.walk_speed*100.0,"sprint_speed_cm_s":settings.sprint_speed*100.0,"jump_velocity_cm_s":settings.jump_speed*100.0,"gravity_cm_s2":settings.gravity*100.0,"gravity_scale_vs_980":settings.gravity*100.0/980.0,"enemy_speed_cm_s":settings.enemy_speed*100.0,"enemy_damage_multiplier":settings.enemy_damage,"pp_recovery_per_second":settings.pp_recovery,"weapon_classes":classes,"photon_arts":arts})
}
fn payload(project: &Project) -> Result<Vec<(String, Vec<u8>)>, String> {
    project.validate()?;
    let mut files = Vec::new();
    let mut assets = Vec::new();
    for (class, name) in [
        (WeaponClass::Sword, "Hunter_Sword"),
        (WeaponClass::Partisan, "Hunter_Partisan"),
        (WeaponClass::Knuckles, "Hunter_Knuckles"),
    ] {
        let (bytes, clips) = hunter_asset(project, class, name)?;
        let path = format!("Assets/{name}.glb");
        assets.push(json!({"file":path,"name":name,"skeletal":true,"bones":17,"animations":clips}));
        files.push((path, bytes));
    }
    let scene = monster::crawler();
    let materials = scene
        .model
        .material_names
        .iter()
        .zip([
            [0.18, 0.35, 0.38],
            [0.08, 0.12, 0.16],
            [1.0, 0.18, 0.08],
            [0.5, 0.72, 0.68],
        ])
        .map(|(n, c)| material(n, c))
        .collect();
    let assigned = scene
        .model
        .triangle_materials
        .iter()
        .map(|m| m.unwrap_or(0))
        .collect::<Vec<_>>();
    let crawler = rigged(
        &scene.model,
        scene.rig.as_ref().unwrap(),
        materials,
        &assigned,
        &[],
        [1.0; 3],
        "ForestCrawler",
    )?;
    assets.push(json!({"file":"Assets/ForestCrawler.glb","name":"ForestCrawler","skeletal":true,"bones":19,"animations":[]}));
    files.push(("Assets/ForestCrawler.glb".into(), crawler));
    let environment = Environment::default();
    for (lobby, name) in [(true, "LobbyScenery"), (false, "FieldScenery")] {
        let path = format!("Assets/{name}.glb");
        let bytes = environment_asset(environment.export_batches(lobby), name)?;
        assets.push(json!({"file":path,"name":name,"skeletal":false,"bones":0,"animations":[]}));
        files.push((path, bytes));
    }
    let manifest = json!({"version":1,"target_unreal":"5.7.4","project_name":project.name,"source_coordinates":"right-handed Y-up metres; glTF 2.0","unreal_primitive_conversion":"[100*x, -100*z, 100*y] centimetres","assets":assets,"scenes":[scene_records(project,true),scene_records(project,false)],"gameplay_ported":false});
    for (name, value) in [
        ("Data/manifest.json", manifest),
        ("Data/balance.json", balance_data(project)),
    ] {
        files.push((
            name.into(),
            serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
        ));
    }
    files.push(("Data/project.json".into(), project.encode()?));
    for (name, source) in TEMPLATES {
        files.push(((*name).into(), source.as_bytes().to_vec()));
    }
    Ok(files)
}
/// Publish into a new folder only. Never replace a user's existing UE project.
pub fn export(project: &Project, destination: &Path) -> Result<PathBuf, String> {
    if destination.exists() {
        return Err(
            "Export folder already exists. Choose a new folder to preserve your Unreal edits."
                .into(),
        );
    }
    let files = payload(project)?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let name = destination
        .file_name()
        .ok_or("Choose a named output folder.")?
        .to_string_lossy();
    let temp = parent.join(format!(".{name}.export-{}", std::process::id()));
    fs::create_dir(&temp).map_err(|e| e.to_string())?;
    let result = (|| {
        for (name, bytes) in files {
            let path = temp.join(name);
            fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::write(path, bytes).map_err(|e| e.to_string())?;
        }
        fs::rename(&temp, destination).map_err(|e| e.to_string())?;
        Ok(destination.to_path_buf())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&temp);
    }
    result
}
const TEMPLATES: &[(&str, &str)] = &[
    (
        "PhantasyStar.uproject",
        include_str!("../../../unreal/template/PhantasyStar.uproject"),
    ),
    (
        "Config/DefaultEngine.ini",
        include_str!("../../../unreal/template/Config/DefaultEngine.ini"),
    ),
    (
        "Config/DefaultGame.ini",
        include_str!("../../../unreal/template/Config/DefaultGame.ini"),
    ),
    (
        "Content/Python/init_unreal.py",
        include_str!("../../../unreal/template/Content/Python/init_unreal.py"),
    ),
    (
        "Content/Python/import_phantasy.py",
        include_str!("../../../unreal/template/Content/Python/import_phantasy.py"),
    ),
    (
        "Open-Unreal.cmd",
        include_str!("../../../unreal/template/Open-Unreal.cmd"),
    ),
    (
        "MIGRATION.md",
        include_str!("../../../unreal/template/MIGRATION.md"),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::prelude::{Mat4, Quat};
    use std::sync::atomic::{AtomicU64, Ordering};
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    fn decode(bytes: &[u8]) -> (Value, &[u8]) {
        assert_eq!(&bytes[..4], b"glTF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 2);
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize,
            bytes.len()
        );
        let size = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        assert_eq!(&bytes[16..20], b"JSON");
        assert_eq!(&bytes[24 + size..28 + size], b"BIN\0");
        (
            serde_json::from_slice(&bytes[20..20 + size]).unwrap(),
            &bytes[28 + size..],
        )
    }
    fn floats(doc: &Value, bin: &[u8], id: usize) -> Vec<f32> {
        let a = &doc["accessors"][id];
        assert_eq!(a["componentType"], 5126);
        let v = &doc["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
        let offset = v["byteOffset"].as_u64().unwrap() as usize;
        bin[offset..offset + v["byteLength"].as_u64().unwrap() as usize]
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
            .collect()
    }
    #[test]
    fn exported_skin_keeps_bind_geometry_and_scaled_animation_times() {
        let mut project = Project::default();
        project.character.head = 2;
        project.character.build = 0;
        project.gameplay.partisan.speed = 1.6;
        let (bytes, clips) =
            hunter_asset(&project, WeaponClass::Partisan, "Hunter_Partisan").unwrap();
        let (doc, bin) = decode(&bytes);
        assert_eq!(doc["skins"][0]["joints"].as_array().unwrap().len(), 17);
        assert_eq!(clips.len(), 7);
        let nodes = doc["nodes"].as_array().unwrap();
        let mut globals = Vec::new();
        for (i, node) in nodes.iter().enumerate().take(18) {
            let array = |key: &str, n: usize, default: f32| -> Vec<f32> {
                node.get(key).map_or(vec![default; n], |v| {
                    v.as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_f64().unwrap() as f32)
                        .collect()
                })
            };
            let scale = array("scale", 3, 1.0);
            let translation = array("translation", 3, 0.0);
            let rotation = node.get("rotation").map_or(Quat::IDENTITY, |v| {
                Quat::from_array(std::array::from_fn(|i| v[i].as_f64().unwrap() as f32))
            });
            let local = Mat4::from_scale_rotation_translation(
                Vec3::from_slice(&scale),
                rotation,
                Vec3::from_slice(&translation),
            );
            let parent = nodes.iter().take(i).position(|p| {
                p.get("children").is_some_and(|c| {
                    c.as_array()
                        .unwrap()
                        .iter()
                        .any(|v| v.as_u64() == Some(i as u64))
                })
            });
            globals.push(parent.map_or(local, |p| globals[p] * local));
        }
        let inverse = floats(
            &doc,
            bin,
            doc["skins"][0]["inverseBindMatrices"].as_u64().unwrap() as usize,
        );
        for joint in 0..17 {
            let matrix =
                globals[joint + 1] * Mat4::from_cols_slice(&inverse[joint * 16..joint * 16 + 16]);
            let point = Vec3::new(0.3, 1.2, -0.4);
            assert!(
                matrix
                    .transform_point3(point)
                    .distance(hunter::body_scale(0) * point)
                    < 1e-5
            );
        }
        assert!(doc["animations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["name"] == "Assault_thrust"));
        let assault = clips
            .iter()
            .find(|c| c["name"] == "Assault_thrust")
            .unwrap();
        assert!((assault["duration_seconds"].as_f64().unwrap() - 0.5).abs() < 1e-6);
        for a in &doc["animations"].as_array().unwrap()[3..] {
            for s in a["samplers"].as_array().unwrap() {
                let time = floats(&doc, bin, s["input"].as_u64().unwrap() as usize);
                assert!(time.windows(2).all(|v| v[1] > v[0]));
                assert!(time.last().unwrap() <= &0.5);
            }
        }
    }
    #[test]
    fn manifests_convert_props_and_balance_without_claiming_a_gameplay_port() {
        let mut p = Project::default();
        p.gameplay.knuckles.reach = 1.5;
        p.props.push(crate::project::Prop {
            scene: "field".into(),
            x: 10.0,
            z: 2.0,
            width: 1.0,
            height: 2.0,
            depth: 3.0,
            color: [100, 120, 140],
        });
        let files = payload(&p).unwrap();
        assert_eq!(files.iter().filter(|(n, _)| n.ends_with(".glb")).count(), 6);
        let manifest: Value = serde_json::from_slice(
            &files
                .iter()
                .find(|(n, _)| n == "Data/manifest.json")
                .unwrap()
                .1,
        )
        .unwrap();
        assert_eq!(manifest["gameplay_ported"], false);
        assert_eq!(manifest["target_unreal"], "5.7.4");
        let prop = manifest["scenes"][1]["boxes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["label"] == "ProjectProp_0")
            .unwrap();
        assert_eq!(prop["location_cm"], json!([1000.0, -200.0, 100.0]));
        assert_eq!(prop["size_cm"], json!([100.0, 300.0, 200.0]));
        assert_eq!(unreal_position([1.0, 2.0, 3.0]), [100.0, -300.0, 200.0]);
        let balance = balance_data(&p);
        let meteor = balance["photon_arts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["name"] == "Meteor Fist")
            .unwrap();
        assert!((meteor["range_cm"].as_f64().unwrap() - 330.0).abs() < 0.001);
    }
    #[test]
    fn export_refuses_existing_folders_and_invalid_projects_without_mutation() {
        let root = std::env::temp_dir().join(format!(
            "psp-unreal-test-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("kit");
        let mut invalid = Project::default();
        invalid.character.head = 9;
        assert!(export(&invalid, &path).is_err());
        assert!(!path.exists());
        export(&Project::default(), &path).unwrap();
        let previous = fs::read(path.join("Data/project.json")).unwrap();
        assert!(export(&Project::default(), &path).is_err());
        assert_eq!(fs::read(path.join("Data/project.json")).unwrap(), previous);
        assert!(path.join("Content/Python/import_phantasy.py").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
