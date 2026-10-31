//! Standalone project workbench. Authoring files are separate from player saves.
use macroquad::prelude::*;
use psp_client::{
    character_ui::{draw_preview, CharacterPanel, Preview},
    hunter::Hunter,
    inventory_ui::draw_menu_cursor,
    presentation::{self, Environment},
    project::{Project, ProjectStore, Prop},
    saves::CharacterRecord,
    scene::capture_frame,
    widgets::*,
};
use psp_core::{
    weapon::WeaponClass,
    world::{ENEMY, FIELD_OBSTACLES, LOBBY_OBSTACLES, RETURN_PAD, SHOPKEEPER, TERMINAL},
};
use std::{
    path::Path,
    process::{Child, Command},
};
fn config() -> Conf {
    Conf {
        window_title: "Phantasy Engine - Project Workbench".into(),
        window_width: 1280,
        window_height: 760,
        ..Default::default()
    }
}

fn save(store: &mut ProjectStore, project: &Project, notice: &mut String) -> bool {
    match store.save(project) {
        Ok(()) => {
            *notice = "Project saved. F6 launches an isolated playtest.".into();
            true
        }
        Err(e) => {
            *notice = e;
            false
        }
    }
}
fn play(
    store: &mut ProjectStore,
    project: &Project,
    notice: &mut String,
    child: &mut Option<Child>,
) {
    if let Some(running) = child.as_mut() {
        match running.try_wait() {
            Ok(None) => {
                *notice = "A playtest is already running. Close it before starting another.".into();
                return;
            }
            Ok(Some(_)) => {}
            Err(e) => {
                *notice = e.to_string();
                return;
            }
        }
    }
    if !save(store, project, notice) {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        *notice = "Cannot find the game beside this editor.".into();
        return;
    };
    let game = exe.with_file_name(if cfg!(windows) {
        "phantasy-star.exe"
    } else {
        "phantasy-star"
    });
    let path = match std::fs::canonicalize(&store.path) {
        Ok(p) => p,
        Err(e) => {
            *notice = e.to_string();
            return;
        }
    };
    match Command::new(game)
        .arg("--project")
        .arg(path)
        .arg("--playtest")
        .spawn()
    {
        Ok(c) => {
            *child = Some(c);
            *notice = "Playtest opened. It never reads or writes your character save.".into();
        }
        Err(e) => {
            *notice = format!("Cannot start game: {e}. Build phantasy-star beside engine-editor.")
        }
    }
}
fn class(index: u8) -> WeaponClass {
    match index {
        1 => WeaponClass::Partisan,
        2 => WeaponClass::Knuckles,
        _ => WeaponClass::Sword,
    }
}
fn class_clip(index: u8) -> usize {
    match index {
        1 => 9,
        2 => 12,
        _ => 3,
    }
}
fn gameplay_panel(project: &mut Project, page: &mut u8, r: Rect) {
    choice(
        "SETTINGS GROUP",
        page,
        &["Movement", "Enemy / PP", "Sword", "Partisan", "Knuckles"],
        r,
    );
    let g = &mut project.gameplay;
    let y = r.y + 58.0;
    let mut rows: Vec<(&str, &mut f32, f32, f32)> = match page {
        0 => vec![
            ("Walk speed", &mut g.walk_speed, 1.0, 8.0),
            ("Sprint speed", &mut g.sprint_speed, 2.0, 12.0),
            ("Jump speed", &mut g.jump_speed, 3.0, 12.0),
            ("Gravity", &mut g.gravity, 8.0, 35.0),
        ],
        1 => vec![
            ("Enemy approach speed", &mut g.enemy_speed, 0.5, 6.0),
            ("Enemy damage multiplier", &mut g.enemy_damage, 0.25, 3.0),
            ("Passive PP per second", &mut g.pp_recovery, 0.0, 20.0),
        ],
        _ => {
            let w = match page {
                2 => &mut g.sword,
                3 => &mut g.partisan,
                _ => &mut g.knuckles,
            };
            vec![
                ("Damage multiplier", &mut w.power, 0.5, 2.0),
                ("Reach multiplier", &mut w.reach, 0.5, 2.0),
                ("Attack speed multiplier", &mut w.speed, 0.5, 2.0),
            ]
        }
    };
    for (i, (name, value, min, max)) in rows.iter_mut().enumerate() {
        slider(
            name,
            value,
            *min,
            *max,
            Rect::new(r.x, y + i as f32 * 52.0, r.w, 45.0),
        );
    }
    label(
        "Changes apply to the next F6 playtest.",
        r.x,
        r.y + r.h - 12.0,
        14.0,
        LIGHTGRAY,
    );
}
fn add_prop(project: &mut Project, scene: u8, notice: &mut String) -> Option<usize> {
    if project.props.len() >= 32 {
        *notice = "A project supports 32 additional props.".into();
        return None;
    }
    for z in [-8.0, -5.0, 0.0, 5.0, 9.0] {
        for x in [-8.0, -4.0, 4.0, 8.0] {
            let p = Prop {
                scene: if scene == 0 { "lobby" } else { "field" }.into(),
                x,
                z,
                width: 1.5,
                height: 1.8,
                depth: 1.5,
                color: [48, 153, 180],
            };
            let obstacles = if scene == 0 {
                &LOBBY_OBSTACLES[..]
            } else {
                &FIELD_OBSTACLES[..]
            };
            let free = obstacles.iter().all(|o| {
                (x - o.center.x).abs() > o.half.x + 1.0 || (z - o.center.z).abs() > o.half.z + 1.0
            }) && project
                .props
                .iter()
                .filter(|o| o.scene == p.scene)
                .all(|o| {
                    (x - o.x).abs() > o.width * 0.5 + 1.0 || (z - o.z).abs() > o.depth * 0.5 + 1.0
                });
            if free && p.core().is_ok() {
                project.props.push(p);
                return Some(project.props.len() - 1);
            }
        }
    }
    *notice = "No free preset position remains; move or remove an existing prop.".into();
    None
}
fn scene_panel(
    project: &mut Project,
    scene: &mut u8,
    selected: &mut usize,
    page: &mut u8,
    r: Rect,
    notice: &mut String,
) {
    choice("SCENE", scene, &["Orbital lobby", "Training field"], r);
    if button("Add prop", Rect::new(r.x, r.y + 53.0, 84.0, 28.0)) {
        if let Some(i) = add_prop(project, *scene, notice) {
            *selected = i;
        }
    }
    if button("Remove", Rect::new(r.x + 90.0, r.y + 53.0, 82.0, 28.0))
        && *selected < project.props.len()
    {
        project.props.remove(*selected);
        *selected = selected.saturating_sub(1);
    }
    if button("Color", Rect::new(r.x + 178.0, r.y + 53.0, 82.0, 28.0)) {
        if let Some(p) = project.props.get_mut(*selected) {
            let colors = [
                [48, 153, 180],
                [187, 79, 104],
                [134, 104, 201],
                [179, 160, 92],
                [78, 145, 95],
            ];
            let i = colors.iter().position(|c| *c == p.color).unwrap_or(0);
            p.color = colors[(i + 1) % colors.len()];
        }
    }
    let indices: Vec<_> = project
        .props
        .iter()
        .enumerate()
        .filter(|(_, p)| p.scene == if *scene == 0 { "lobby" } else { "field" })
        .map(|(i, _)| i)
        .collect();
    if indices.is_empty() {
        label(
            "No props. Add one to begin.",
            r.x,
            r.y + 116.0,
            17.0,
            LIGHTGRAY,
        );
        return;
    }
    if !indices.contains(selected) {
        *selected = indices[0];
    }
    let mut current = indices.iter().position(|i| i == selected).unwrap();
    if button("<", Rect::new(r.x, r.y + 87.0, 28.0, 26.0)) {
        current = current.saturating_sub(1);
    }
    if button(">", Rect::new(r.x + r.w - 28.0, r.y + 87.0, 28.0, 26.0)) {
        current = (current + 1).min(indices.len() - 1);
    }
    *selected = indices[current];
    label(
        format!("Prop {} / {}", current + 1, indices.len()),
        r.x + 42.0,
        r.y + 106.0,
        17.0,
        WHITE,
    );
    choice(
        "PROP INSPECTOR",
        page,
        &["Position", "Size"],
        Rect::new(r.x, r.y + 122.0, r.w, 46.0),
    );
    let p = &mut project.props[*selected];
    let y = r.y + 180.0;
    let rows: Vec<(&str, &mut f32, f32, f32)> = if *page == 0 {
        vec![
            ("Position X", &mut p.x, -10.0, 10.0),
            ("Position Z", &mut p.z, -10.0, 10.0),
        ]
    } else {
        vec![
            ("Width", &mut p.width, 0.3, 4.0),
            ("Height", &mut p.height, 0.3, 4.0),
            ("Depth", &mut p.depth, 0.3, 4.0),
        ]
    };
    for (i, (name, value, min, max)) in rows.into_iter().enumerate() {
        slider(
            name,
            value,
            min,
            max,
            Rect::new(r.x, y + i as f32 * 42.0, r.w, 40.0),
        );
    }
}
fn scene_view(
    project: &Project,
    scene: u8,
    selected: usize,
    yaw: f32,
    r: Rect,
    environment: &Environment,
) {
    let lobby = scene == 0;
    set_camera(&Camera3D {
        position: vec3(yaw.sin() * 21.0, 20.0, yaw.cos() * 21.0),
        target: Vec3::ZERO,
        up: Vec3::Y,
        fovy: 60.0f32.to_radians(),
        aspect: Some(r.w / r.h),
        viewport: Some((
            r.x as i32,
            (screen_height() - r.y - r.h) as i32,
            r.w as i32,
            r.h as i32,
        )),
        ..Default::default()
    });
    environment.draw(lobby);
    draw_grid(
        30,
        1.0,
        Color::from_rgba(67, 103, 117, 255),
        Color::from_rgba(26, 49, 66, 255),
    );
    for o in if lobby {
        &LOBBY_OBSTACLES[..]
    } else {
        &FIELD_OBSTACLES[..]
    } {
        presentation::shaded_box(
            vec3(o.center.x, 0.5, o.center.z),
            vec3(o.half.x * 2.0, 1.0, o.half.z * 2.0),
            DARKGRAY,
        );
    }
    for (i, p) in project
        .props
        .iter()
        .enumerate()
        .filter(|(_, p)| p.scene == if lobby { "lobby" } else { "field" })
    {
        let center = vec3(p.x, p.height * 0.5, p.z);
        let size = vec3(p.width, p.height, p.depth);
        presentation::shaded_box(
            center,
            size,
            Color::from_rgba(p.color[0], p.color[1], p.color[2], 255),
        );
        if i == selected {
            draw_cube_wires(
                center,
                size + Vec3::splat(0.04),
                if p.core().is_ok() { YELLOW } else { RED },
            );
        }
    }
    for p in if lobby {
        vec![TERMINAL, SHOPKEEPER]
    } else {
        vec![ENEMY, RETURN_PAD]
    } {
        draw_cylinder_wires(
            vec3(p.x, 0.05, p.z),
            2.0,
            2.0,
            0.08,
            None,
            presentation::ACCENT,
        );
    }
    set_default_camera();
    label(
        "Yellow: selected prop   Cyan circles: protected interactions",
        r.x + 12.0,
        r.y + 24.0,
        15.0,
        LIGHTGRAY,
    );
}

#[macroquad::main(config)]
async fn main() {
    presentation::initialize_font();
    prevent_quit();
    let options = match psp_client::project::LaunchOptions::parse(std::env::args_os().skip(1)) {
        Ok(o) => o,
        Err(e) => {
            error_screen("Cannot open editor", &e).await;
            return;
        }
    };
    let path = options
        .project
        .unwrap_or_else(psp_client::project::default_path);
    let mut store = match ProjectStore::open(path) {
        Ok(s) => s,
        Err(e) => {
            error_screen("Cannot load project", &e).await;
            return;
        }
    };
    let mut project = store.saved.clone();
    let mut panel = CharacterPanel::default();
    let mut hunter = Hunter::default();
    let environment = Environment::default();
    let mut tab = 0;
    let mut game_page = 0;
    let mut scene = 0;
    let mut prop_page = 0;
    let mut selected_prop = 0;
    let mut weapon = 0;
    let mut clip = 0u8;
    let mut playing = true;
    let mut speed = 1.0;
    let mut time = 0.0;
    let mut yaw = 0.35f32;
    let mut bones = false;
    let mut notice = "Choose a tab to start. Ctrl+S saves; F6 runs a safe playtest.".to_owned();
    let mut undo: Vec<Project> = Vec::new();
    let mut redo: Vec<Project> = Vec::new();
    let mut child = None;
    let mut quit_armed = false;
    let mut reload_armed = false;
    let mut frames = 0;
    let smoke = std::env::var_os("PSP_EDITOR_SMOKE").is_some();
    let mut smoke_saved = None;
    loop {
        let before = project.clone();
        let dt = get_frame_time().clamp(0.0, 0.1);
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if is_quit_requested() || (ctrl && is_key_pressed(KeyCode::Q)) {
            if project == store.saved || quit_armed {
                break;
            }
            quit_armed = true;
            notice = "Unsaved changes. Save, or close again to discard them.".into();
        }
        clear_background(Color::from_rgba(8, 18, 29, 255));
        let width = screen_width();
        let height = screen_height();
        label("PHANTASY ENGINE", 18.0, 29.0, 24.0, WHITE);
        let dirty = project != store.saved;
        label(
            format!("{}{}", project.name, if dirty { " *" } else { "" }),
            260.0,
            28.0,
            17.0,
            LIGHTGRAY,
        );
        let save_clicked = button("Save / Ctrl+S", Rect::new(width - 362.0, 10.0, 126.0, 32.0));
        let reload_clicked = button("Reload", Rect::new(width - 230.0, 10.0, 88.0, 32.0));
        let play_clicked = button("Play / F6", Rect::new(width - 136.0, 10.0, 118.0, 32.0));
        for (i, name) in ["Character", "Animation", "Gameplay", "Scene", "Help"]
            .into_iter()
            .enumerate()
        {
            let r = Rect::new(18.0 + i as f32 * 126.0, 52.0, 118.0, 30.0);
            if button(name, r) {
                tab = i;
            }
            if tab == i {
                draw_rectangle(r.x, r.y + r.h - 2.0, r.w, 2.0, presentation::ACCENT);
            }
        }
        let left = Rect::new(18.0, 94.0, 300.0, height - 146.0);
        let viewport = Rect::new(332.0, 94.0, width - 350.0, height - 146.0);
        if tab != 4 {
            section(
                left,
                [
                    "CHARACTER",
                    "MOTION PREVIEW",
                    "GAMEPLAY SETTINGS",
                    "SCENE PROPS",
                ][tab],
            );
        }
        let r = Rect::new(left.x + 14.0, left.y + 38.0, left.w - 28.0, left.h - 44.0);
        if is_mouse_button_down(MouseButton::Middle) && viewport.contains(mouse_position().into()) {
            yaw -= mouse_delta_position().x * 3.0;
        }
        match tab {
            0 => {
                let mut c = psp_core::character::Character {
                    name: project.character.name.clone(),
                    armor: project.character.armor,
                    accent: project.character.accent,
                    skin: project.character.skin,
                    hair: project.character.hair,
                    head: project.character.head,
                    build: project.character.build,
                };
                panel.draw(&mut c, r);
                project.character = CharacterRecord::from(&c);
            }
            1 => {
                let old = weapon;
                choice(
                    "WEAPON MESH",
                    &mut weapon,
                    &["Sword", "Partisan", "Knuckles"],
                    r,
                );
                if old != weapon {
                    clip = class_clip(weapon) as u8;
                    time = 0.0;
                }
                let names: Vec<_> = hunter.clips().iter().map(|c| c.name.as_str()).collect();
                let old = clip;
                choice(
                    "ANIMATION CLIP",
                    &mut clip,
                    &names,
                    Rect::new(r.x, r.y + 50.0, r.w, 46.0),
                );
                if clip != old {
                    time = 0.0;
                }
                if button(
                    if playing { "Pause" } else { "Play" },
                    Rect::new(r.x, r.y + 105.0, 126.0, 28.0),
                ) || is_key_pressed(KeyCode::Space)
                {
                    playing = !playing;
                }
                if button(
                    if bones { "Hide bones" } else { "Show bones" },
                    Rect::new(r.x + 136.0, r.y + 105.0, 136.0, 28.0),
                ) {
                    bones = !bones;
                }
                let duration = hunter.clips()[clip as usize].duration;
                slider(
                    "Timeline (seconds)",
                    &mut time,
                    0.0,
                    duration,
                    Rect::new(r.x, r.y + 146.0, r.w, 44.0),
                );
                if is_mouse_button_down(MouseButton::Left)
                    && Rect::new(r.x, r.y + 165.0, r.w, 30.0).contains(mouse_position().into())
                {
                    playing = false;
                }
                slider(
                    "Playback speed",
                    &mut speed,
                    0.1,
                    2.0,
                    Rect::new(r.x, r.y + 197.0, r.w, 44.0),
                );
                slider(
                    "Animation energy",
                    &mut project.gameplay.animation_energy,
                    0.6,
                    1.4,
                    Rect::new(r.x, r.y + 248.0, r.w, 44.0),
                );
                if playing {
                    time = (time + dt * speed) % duration;
                }
            }
            2 => gameplay_panel(&mut project, &mut game_page, r),
            3 => scene_panel(
                &mut project,
                &mut scene,
                &mut selected_prop,
                &mut prop_page,
                r,
                &mut notice,
            ),
            _ => {
                section(
                    Rect::new(18.0, 94.0, width - 36.0, height - 146.0),
                    "START HERE / YOUR GAME PROJECT",
                );
                let lines = [
                    "1. Character: edit the default look used by new saves and playtests.",
                    "2. Animation: choose a mesh and clip; pause, scrub, and adjust energy.",
                    "3. Gameplay: tune movement, enemies, PP, and weapon multipliers.",
                    "4. Scene: add colored solid props; edit their position and dimensions.",
                    "5. Save, then F6 opens the game using this exact project.",
                    "Playtests never read or write your real character save.",
                    "Ctrl+Z / Ctrl+Y: undo / redo. Reload discards edits and rereads JSON.",
                    "Projects use readable JSON. Advanced features remain editable in Rust.",
                    "Read START-HERE.md and docs/engine-guide.md for the source map.",
                ];
                for (i, line) in lines.into_iter().enumerate() {
                    label(line, 36.0, 153.0 + i as f32 * 28.0, 17.0, WHITE);
                }
            }
        }
        if tab == 3 {
            scene_view(&project, scene, selected_prop, yaw, viewport, &environment);
        } else if tab != 4 {
            let c = psp_core::character::Character {
                name: project.character.name.clone(),
                armor: project.character.armor,
                accent: project.character.accent,
                skin: project.character.skin,
                hair: project.character.hair,
                head: project.character.head,
                build: project.character.build,
            };
            let p = Preview {
                class: class(weapon),
                clip: if tab == 1 { clip as usize } else { 0 },
                time: if tab == 1 {
                    time
                } else {
                    (get_time() % 2.4) as f32
                },
                energy: project.gameplay.animation_energy,
                yaw,
                bones,
            };
            draw_preview(&mut hunter, &c, &p, viewport);
            label(
                "MMB drag: orbit   Animation tab: preview and scrub clips",
                viewport.x + 12.0,
                viewport.y + viewport.h - 12.0,
                15.0,
                LIGHTGRAY,
            );
        }
        if before != project {
            undo.push(before);
            if undo.len() > 64 {
                undo.remove(0);
            }
            redo.clear();
            quit_armed = false;
            reload_armed = false;
        }
        if ctrl && is_key_pressed(KeyCode::Z) {
            if let Some(previous) = undo.pop() {
                redo.push(project);
                project = previous;
            }
        }
        if ctrl && is_key_pressed(KeyCode::Y) {
            if let Some(next) = redo.pop() {
                undo.push(project);
                project = next;
            }
        }
        if reload_clicked || ctrl && is_key_pressed(KeyCode::R) {
            if !dirty || reload_armed {
                match store.reload() {
                    Ok(p) => {
                        project = p;
                        undo.clear();
                        redo.clear();
                        quit_armed = false;
                        reload_armed = false;
                        notice = "Project reloaded.".into();
                    }
                    Err(e) => notice = e,
                }
            } else {
                reload_armed = true;
                notice = "Unsaved changes. Click Reload again to discard, or save first.".into();
            }
        }
        if save_clicked || ctrl && is_key_pressed(KeyCode::S) {
            save(&mut store, &project, &mut notice);
        }
        if play_clicked || is_key_pressed(KeyCode::F6) {
            play(&mut store, &project, &mut notice, &mut child);
        }
        let error = project.validate().err();
        let status = error.as_deref().unwrap_or(&notice);
        label(
            status,
            20.0,
            height - 28.0,
            15.0,
            if error.is_some() {
                ORANGE
            } else {
                presentation::ACCENT
            },
        );
        label(
            format!("Project: {}", store.path.display()),
            20.0,
            height - 9.0,
            12.0,
            GRAY,
        );
        show_mouse(false);
        set_cursor_grab(false);
        draw_menu_cursor();
        frames += 1;
        if smoke {
            if frames == 1 {
                project.character.name = "Astra".into();
                project.character.head = 2;
                project.character.armor = 2;
                project.gameplay.walk_speed = 4.5;
                project.gameplay.sword.power = 1.2;
                add_prop(&mut project, 0, &mut notice);
            }
            if frames == 3 {
                assert!(save(&mut store, &project, &mut notice), "{notice}");
                smoke_saved = Some(project.clone());
                project.gameplay.walk_speed = 3.0;
                project = store.reload().expect("editor reload failed");
                assert_eq!(Some(&project), smoke_saved.as_ref());
                tab = 1;
                clip = 5;
                time = 0.3;
                playing = false;
            }
            if frames == 4 && std::env::var_os("PSP_EDITOR_PLAYTEST").is_some() {
                play(&mut store, &project, &mut notice, &mut child);
                assert!(child.is_some(), "{notice}");
            }
            let finished = if frames >= 6 && std::env::var_os("PSP_EDITOR_PLAYTEST").is_some() {
                match child
                    .as_mut()
                    .unwrap()
                    .try_wait()
                    .expect("playtest status unavailable")
                {
                    Some(status) => {
                        assert!(status.success(), "playtest failed: {status}");
                        true
                    }
                    None => false,
                }
            } else {
                frames >= 6
            };
            if finished {
                let path =
                    std::env::var_os("PSP_EDITOR_CAPTURE").expect("smoke needs PSP_EDITOR_CAPTURE");
                capture_frame(Path::new(&path)).expect("editor capture failed");
                assert_eq!(Some(&project), smoke_saved.as_ref());
                println!("Editor smoke passed: edit -> save -> reload -> shared motion preview");
                break;
            }
        } else if frames >= 3 {
            if let Some(path) = std::env::var_os("PSP_EDITOR_CAPTURE") {
                capture_frame(Path::new(&path)).expect("editor capture failed");
                break;
            }
        }
        next_frame().await;
    }
    show_mouse(true);
}
