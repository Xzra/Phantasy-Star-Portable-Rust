use macroquad::prelude::*;
use psp_client::{
    animation::ProjectionMotion,
    asset_paths,
    combat_ui::{hud, Feedback},
    hunter::Hunter,
    inventory_ui::{draw_menu_cursor, InventoryMenu},
    monster,
    presentation::{self, Environment},
    scene::{capture_frame, Scene},
};
use psp_core::{
    weapon::PhotonArt,
    world::{
        EnemyAction, Input, Position, World, WorldEvent, ENEMY, FIXED_STEP, RETURN_PAD, TERMINAL,
    },
    Event, Location, QuestState,
};
use std::path::{Path, PathBuf};

const INK: Color = Color::new(0.035, 0.065, 0.11, 1.0);
const ACCENT: Color = Color::new(0.15, 0.85, 0.95, 1.0);

fn window_config() -> Conf {
    Conf {
        window_title: "Phantasy Star Rust - Solo Prototype".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        ..Default::default()
    }
}

fn point(p: Position, y: f32) -> Vec3 {
    vec3(p.x, y, p.z)
}

fn obstacle_height(world: &World, index: usize) -> f32 {
    if world.session().location() == Location::Lobby && index < 2 {
        3.0
    } else if world.session().location() == Location::Lobby {
        0.9
    } else {
        1.8
    }
}

fn chase_camera(world: &World, target: Vec3, desired: Vec3) -> Vec3 {
    let edge = world.half_extent() - 0.25;
    let desired = vec3(
        desired.x.clamp(-edge, edge),
        desired.y,
        desired.z.clamp(-edge, edge),
    );
    for step in 1..=40 {
        let p = target.lerp(desired, step as f32 / 40.0);
        let blocked = world.obstacles().iter().enumerate().any(|(i, box_)| {
            (p.x - box_.center.x).abs() < box_.half.x + 0.18
                && (p.z - box_.center.z).abs() < box_.half.z + 0.18
                && p.y < obstacle_height(world, i) + 0.18
        }) || (world.session().location() == Location::Lobby
            && (p.x - TERMINAL.x).abs() < 0.78
            && (p.z - TERMINAL.z).abs() < 0.53
            && p.y < 2.0);
        if blocked {
            return target.lerp(desired, (step - 1) as f32 / 40.0);
        }
    }
    desired
}

fn smoke_input(world: &World) -> Input {
    if world.defeated() {
        return Input {
            interact: true,
            ..Input::default()
        };
    }
    let active = world.session().location() == Location::TrainingField
        && world.session().quest() == QuestState::Active;
    let goal = if world.session().location() == Location::Lobby {
        TERMINAL
    } else if active {
        let enemy = world.enemy_position();
        let p = world.position();
        let d = p.distance(enemy).max(0.01);
        Position {
            x: enemy.x + (p.x - enemy.x) / d * 2.0,
            z: enemy.z + (p.z - enemy.z) / d * 2.0,
        }
    } else {
        RETURN_PAD
    };
    let distance = world.position().distance(goal);
    let dodge = active
        && world.enemy_action() == EnemyAction::Windup
        && world.enemy_elapsed() > world.enemy_windup_duration() - 0.17
        && world.dodge_ready();
    Input {
        movement: if dodge {
            Position {
                x: world.enemy_facing().cos(),
                z: -world.enemy_facing().sin(),
            }
        } else if distance > 0.15 {
            Position {
                x: goal.x - world.position().x,
                z: goal.z - world.position().z,
            }
        } else {
            Position::default()
        },
        sprint: true,
        interact: !active && distance < 2.0,
        attack: active && world.position().distance(world.enemy_position()) < 2.6,
        photon_art: active
            && world.attack().is_none()
            && world
                .weapon()
                .photon_art
                .is_some_and(|a| world.pp() >= a.spec().pp)
            && world.position().distance(world.enemy_position()) < 3.2,
        lock_on: active && !world.locked_on(),
        dodge,
        heal: active && world.session().hunter().health <= 60 && world.session().mates() > 0,
        ..Input::default()
    }
}

fn draw_pad(position: Position, color: Color, active: bool, time: f32) {
    draw_cylinder(
        point(position, 0.06),
        1.25,
        1.25,
        0.12,
        None,
        Color::new(0.1, 0.16, 0.22, 1.0),
    );
    draw_cylinder_wires(point(position, 0.13), 1.15, 1.15, 0.04, None, color);
    if active {
        for i in 0..6 {
            let angle = i as f32 * std::f32::consts::TAU / 6.0 + time * 0.4;
            draw_sphere(
                point(position, 0.22) + vec3(angle.cos(), 0.0, angle.sin()),
                0.055,
                None,
                color,
            );
        }
    }
}

fn draw_environment(world: &World, environment: &Environment) {
    let lobby = world.session().location() == Location::Lobby;
    let size = world.half_extent() * 2.0;
    environment.draw(lobby);
    for side in [-1.0, 1.0] {
        presentation::cube(
            vec3(
                side * (size / 2.0 + 0.2),
                if lobby { 0.65 } else { 0.2 },
                0.0,
            ),
            vec3(0.4, if lobby { 1.3 } else { 0.4 }, size + 0.4),
            None,
            INK,
        );
        presentation::cube(
            vec3(
                0.0,
                if lobby { 0.65 } else { 0.2 },
                side * (size / 2.0 + 0.2),
            ),
            vec3(size + 0.4, if lobby { 1.3 } else { 0.4 }, 0.4),
            None,
            INK,
        );
        presentation::cube(
            vec3(side * (size / 2.0), if lobby { 1.32 } else { 0.43 }, 0.0),
            vec3(0.08, 0.06, size),
            None,
            ACCENT,
        );
        presentation::cube(
            vec3(0.0, if lobby { 1.32 } else { 0.43 }, side * (size / 2.0)),
            vec3(size, 0.06, 0.08),
            None,
            ACCENT,
        );
    }
    for (i, obstacle) in world.obstacles().iter().enumerate() {
        let height = obstacle_height(world, i);
        if !lobby {
            draw_cylinder(
                point(obstacle.center, height * 0.4),
                obstacle.half.x * 0.7,
                obstacle.half.x,
                height * 0.8,
                None,
                Color::from_rgba(76, 86, 75, 255),
            );
            draw_cylinder(
                point(obstacle.center, height * 0.85),
                obstacle.half.x * 0.3,
                obstacle.half.x * 0.7,
                height * 0.3,
                None,
                Color::from_rgba(107, 119, 89, 255),
            );
            continue;
        }
        presentation::cube(
            point(obstacle.center, height / 2.0),
            vec3(obstacle.half.x * 2.0, height, obstacle.half.z * 2.0),
            None,
            INK,
        );
        presentation::cube(
            point(obstacle.center, height + 0.03),
            vec3(obstacle.half.x * 2.0, 0.06, obstacle.half.z * 2.0),
            None,
            if lobby { ACCENT } else { ORANGE },
        );
    }
    if lobby {
        draw_pad(TERMINAL, ACCENT, true, world.time());
        presentation::cube(point(TERMINAL, 0.9), vec3(1.2, 1.8, 0.7), None, INK);
        presentation::cube(
            point(TERMINAL, 1.55) + vec3(0.0, 0.0, 0.38),
            vec3(0.95, 0.65, 0.05),
            None,
            ACCENT,
        );
        draw_sphere(point(TERMINAL, 2.45), 0.24, None, SKYBLUE);
        for i in -5..5 {
            presentation::cube(
                vec3(0.0, 0.025, i as f32),
                vec3(0.1, 0.02, 0.5),
                None,
                ACCENT,
            );
        }
        // Original scenery: an observation window, distant planet and landing lights.
        draw_sphere(
            vec3(-14.0, 16.0, -46.0),
            8.0,
            None,
            Color::new(0.10, 0.30, 0.44, 1.0),
        );
        for i in 0..28 {
            let x = (i as f32 * 13.37).sin() * 45.0;
            let y = 6.0 + (i as f32 * 4.17).cos().abs() * 28.0;
            draw_sphere(vec3(x, y, -55.0), 0.055, None, LIGHTGRAY);
        }
    } else {
        draw_pad(
            RETURN_PAD,
            if world.session().quest() == QuestState::Complete {
                GREEN
            } else {
                ACCENT
            },
            true,
            world.time(),
        );
        draw_pad(
            ENEMY,
            if world.session().quest() == QuestState::Active {
                ORANGE
            } else {
                GREEN
            },
            true,
            world.time(),
        );
    }
}

fn draw_player(world: &World, hunter: &Hunter, bones: bool) {
    let p = world.position();
    draw_cylinder(
        point(p, 0.025),
        0.45,
        0.45,
        0.02,
        None,
        Color::new(0.0, 0.02, 0.04, 0.55),
    );
    hunter.draw(bones);
    if let Some(attack) = world
        .attack()
        .filter(|a| a.art == Some(PhotonArt::Sweep) && (0.3..0.75).contains(&a.progress()))
    {
        let radius = 1.0 + (attack.progress() - 0.3) * 5.0;
        draw_cylinder_wires(
            point(p, world.height() + 0.45),
            radius,
            radius,
            0.12,
            None,
            VIOLET,
        );
    }
}

fn draw_enemy(world: &World, model: Option<&Scene>, hit_flash: f32, death_visible: bool) {
    if world.session().location() != Location::TrainingField
        || (world.session().enemy_health() == 0 && !death_visible)
    {
        return;
    }
    if world.enemy_action() == EnemyAction::Windup || world.enemy_action() == EnemyAction::Strike {
        let p = point(world.enemy_position(), 0.04);
        let f = vec3(world.enemy_facing().sin(), 0.0, world.enemy_facing().cos());
        let r = vec3(f.z, 0.0, -f.x);
        let a = p + f * 3.0 + r * 1.5;
        let b = p + f * 3.0 - r * 1.5;
        let mesh = Mesh {
            vertices: vec![
                Vertex::new2(p, Vec2::ZERO, Color::new(1.0, 0.12, 0.05, 0.3)),
                Vertex::new2(a, Vec2::ZERO, Color::new(1.0, 0.12, 0.05, 0.3)),
                Vertex::new2(b, Vec2::ZERO, Color::new(1.0, 0.12, 0.05, 0.3)),
            ],
            indices: vec![0, 1, 2],
            texture: None,
        };
        draw_mesh(&mesh);
        draw_line_3d(p, a, ORANGE);
        draw_line_3d(p, b, ORANGE);
        draw_line_3d(a, b, ORANGE);
        draw_cylinder_wires(p, 1.3, 1.3, 0.07, None, RED);
    }
    if let Some(model) = model {
        model.draw(true);
    }

    if hit_flash > 0.0 {
        for i in 0..10 {
            let angle = i as f32 * std::f32::consts::TAU / 10.0;
            draw_line_3d(
                point(world.enemy_position(), 0.9),
                point(world.enemy_position(), 0.9)
                    + vec3(angle.cos(), (angle * 2.0).sin(), angle.sin()) * (1.0 - hit_flash) * 1.7,
                YELLOW,
            );
        }
    }
}

fn button(label: &str, y: f32) -> bool {
    let rect = Rect::new(screen_width() / 2.0 - 140.0, y, 280.0, 44.0);
    let mouse = mouse_position();
    let hover = rect.contains(vec2(mouse.0, mouse.1));
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hover {
            Color::new(0.12, 0.35, 0.44, 1.0)
        } else {
            INK
        },
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, ACCENT);
    let w = measure_text(label, None, 22, 1.0).width;
    draw_text(label, (screen_width() - w) / 2.0, y + 29.0, 22.0, WHITE);
    hover && is_mouse_button_pressed(MouseButton::Left)
}

#[macroquad::main(window_config)]
async fn main() {
    presentation::initialize_font();
    let environment = Environment::default();
    let mut world = World::default();
    if let Ok(id) = std::env::var("PSP_SMOKE_WEAPON") {
        world
            .equip_weapon(psp_core::inventory::WeaponId(
                id.parse()
                    .expect("PSP_SMOKE_WEAPON must be an owned weapon ID"),
            ))
            .expect("PSP_SMOKE_WEAPON is not owned");
    }
    if let Ok(art) = std::env::var("PSP_SMOKE_ART") {
        let art = match art.as_str() {
            "sweep" => PhotonArt::Sweep,
            "cleave" => PhotonArt::RisingCleave,
            "thrust" => PhotonArt::PiercingThrust,
            _ => panic!("PSP_SMOKE_ART must be sweep, cleave or thrust"),
        };
        world.equip_photon_art(Some(art)).unwrap();
    }
    let mut hunter = Hunter::default();
    let mut bones = false;
    let mut feedback = Feedback::default();
    let executable_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();
    let working_dir = std::env::current_dir().unwrap_or_default();
    let candidate = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| asset_paths::model(&executable_dir, &working_dir));
    let imported = candidate
        .as_ref()
        .and_then(|path| match Scene::load(Some(path), None) {
            Ok(scene) => Some(scene),
            Err(error) => {
                eprintln!("Optional monster unavailable: {error}");
                None
            }
        });
    let external_model = imported.is_some();
    let mut projection = Some(imported.unwrap_or_else(monster::crawler));
    if let Some(scene) = &mut projection {
        scene.place_on_ground(point(world.enemy_position(), 0.0), 1.3);
    }
    let motion_path = std::env::args_os().nth(2).map(PathBuf::from).or_else(|| {
        candidate
            .as_ref()
            .and_then(|path| asset_paths::motions(path, &executable_dir, &working_dir))
    });
    let mut motion = ProjectionMotion::new(
        projection
            .as_ref()
            .and_then(|scene| {
                motion_path
                    .as_ref()
                    .filter(|_| external_model)
                    .and_then(|path| match scene.load_motions(path) {
                        Ok(library) => {
                            for warning in library.warnings {
                                eprintln!("{warning}");
                            }
                            Some(library.clips)
                        }
                        Err(error) => {
                            eprintln!("Optional motions unavailable: {error}");
                            None
                        }
                    })
            })
            .unwrap_or_default(),
    );
    let mut paused = false;
    let mut weapon_open = false;
    let mut inventory_menu = InventoryMenu::default();
    let mut accumulator = 0.0;
    let mut pending_interaction = false;
    let mut pending_attack = false;
    let mut pending_dodge = false;
    let mut pending_jump = false;
    let mut pending_lock = false;
    let mut pending_art = false;
    let mut pending_heal = false;
    let mut yaw = 0.0_f32;
    let mut pitch = 0.4_f32;
    let mut distance = 6.4_f32;
    if let Ok(preview) = std::env::var("PSP_GAME_PREVIEW") {
        match preview.as_str() {
            "weapon" => {
                paused = true;
                weapon_open = true;
            }
            "grip" => {
                yaw = 2.7;
                distance = 3.0;
                pitch = 0.25;
            }
            _ => panic!("PSP_GAME_PREVIEW must be weapon or grip"),
        }
    }
    let mut camera_target = point(world.position(), 1.0);
    let mut previous_mouse = mouse_position();
    let mut message = "Welcome aboard. Follow the blue lights to the mission terminal.".to_owned();
    let mut message_time = 6.0_f32;
    let mut hit_flash = 0.0_f32;
    let capture = std::env::var_os("PSP_GAME_CAPTURE");
    let smoke = std::env::var_os("PSP_GAME_SMOKE").is_some();
    let mut captured_lobby = false;
    let mut captured_field = false;
    let mut captured_hit = false;
    let mut captured_death = false;
    let mut captured_results = false;
    let mut captured_wave2 = false;
    let mut captured_wave3 = false;
    let mut block_attack = false;
    let mut block_art = false;
    let mut frames = 0;
    loop {
        let delta = get_frame_time().clamp(0.0, 0.1);
        if is_key_pressed(KeyCode::Escape) {
            weapon_open = false;
            paused = !paused;
            accumulator = 0.0;
            pending_interaction = false;
            pending_attack = false;
            pending_dodge = false;
            pending_jump = false;
            pending_lock = false;
            pending_art = false;
            pending_heal = false;
        }
        if is_key_pressed(KeyCode::Tab) || is_key_pressed(KeyCode::I) {
            weapon_open = !weapon_open;
            paused = weapon_open;
            accumulator = 0.0;
            pending_interaction = false;
            pending_attack = false;
            pending_dodge = false;
            pending_jump = false;
            pending_lock = false;
            pending_art = false;
            pending_heal = false;
            block_attack = true;
            block_art = true;
        }
        if paused && is_key_pressed(KeyCode::Enter) {
            paused = false;
            weapon_open = false;
        }
        if paused {
            block_attack = true;
            block_art = true;
        }
        if !is_key_down(KeyCode::J) && !is_mouse_button_down(MouseButton::Left) {
            block_attack = false;
        }
        if !is_key_down(KeyCode::F) && !is_mouse_button_down(MouseButton::Right) {
            block_art = false;
        }
        let mouse = mouse_position();
        if !paused {
            if is_mouse_button_down(MouseButton::Middle) {
                yaw -= (mouse.0 - previous_mouse.0) * 0.008;
                pitch = (pitch + (mouse.1 - previous_mouse.1) * 0.006).clamp(0.18, 1.15);
            }
            if is_key_down(KeyCode::Left) {
                yaw += delta * 1.5;
            }
            if is_key_down(KeyCode::Right) {
                yaw -= delta * 1.5;
            }
            distance = (distance - mouse_wheel().1 * 0.45).clamp(3.0, 11.0);
            let x = f32::from(is_key_down(KeyCode::D)) - f32::from(is_key_down(KeyCode::A));
            let forward = f32::from(is_key_down(KeyCode::W)) - f32::from(is_key_down(KeyCode::S));
            let movement = Position {
                x: yaw.cos() * x - yaw.sin() * forward,
                z: -yaw.sin() * x - yaw.cos() * forward,
            };
            pending_interaction |= is_key_pressed(KeyCode::E);
            pending_attack |= !block_attack
                && (is_key_pressed(KeyCode::J) || is_mouse_button_pressed(MouseButton::Left));
            pending_dodge |= is_key_pressed(KeyCode::X);
            pending_jump |= is_key_pressed(KeyCode::Space);
            pending_lock |= is_key_pressed(KeyCode::Q);
            pending_art |= !block_art
                && (is_key_pressed(KeyCode::F) || is_mouse_button_pressed(MouseButton::Right));
            pending_heal |= is_key_pressed(KeyCode::H);
            accumulator = (accumulator + delta).min(FIXED_STEP * 6.0);
            while accumulator >= FIXED_STEP {
                let location = world.session().location();
                let input = if smoke {
                    smoke_input(&world)
                } else {
                    Input {
                        movement,
                        sprint: is_key_down(KeyCode::LeftShift),
                        interact: pending_interaction,
                        attack: !block_attack
                            && (is_key_down(KeyCode::J) || is_mouse_button_down(MouseButton::Left)),
                        attack_pressed: pending_attack,
                        dodge: pending_dodge,
                        jump: pending_jump,
                        lock_on: pending_lock,
                        photon_art: pending_art,
                        heal: pending_heal,
                    }
                };
                let events = world.step(input, FIXED_STEP);
                pending_interaction = false;
                pending_attack = false;
                pending_dodge = false;
                pending_jump = false;
                pending_lock = false;
                pending_art = false;
                pending_heal = false;
                for event in events {
                    match event {
                        WorldEvent::Gameplay(Event::QuestStarted) => {
                            motion.reset();
                            feedback = Feedback::default();
                            message =
                                "Hunter trial / 3 waves / H to heal / Q to lock on.".to_owned();
                            message_time = 4.0;
                        }
                        WorldEvent::Gameplay(Event::EnemyDefeated { wave }) => {
                            motion.die(world.time());
                            message = if wave < psp_core::TRIAL_WAVES {
                                format!("WAVE {} CLEAR / Next wave incoming", wave)
                            } else {
                                "TRIAL CLEAR".to_owned()
                            };
                            message_time = 2.0;
                        }
                        WorldEvent::Gameplay(Event::WaveStarted { wave }) => {
                            motion.reset();
                            feedback = Feedback::default();
                            message = format!(
                                "WAVE {} / {}{}",
                                wave,
                                psp_core::TRIAL_WAVES,
                                if wave == 3 { " / ENHANCED ENEMY" } else { "" }
                            );
                            message_time = 2.5;
                        }
                        WorldEvent::Gameplay(Event::HunterHealed { amount, .. }) => {
                            feedback.heal(&world, amount);
                            message = format!("RECOVERY / +{} HP", amount);
                            message_time = 1.2;
                        }
                        WorldEvent::Gameplay(Event::EnemyDamaged { damage, .. }) => {
                            feedback.damage(&world, damage);
                            motion.hit(world.time());
                            hit_flash = 0.35;
                        }
                        WorldEvent::Gameplay(Event::QuestCompleted { .. }) => {
                            motion.die(world.time());
                            message = format!(
                                "TRIAL CLEAR  /  Level {}  /  +100 XP",
                                world.session().hunter().level
                            );
                            message_time = 5.0;
                        }
                        WorldEvent::Gameplay(Event::ReturnedToLobby) => {
                            message = "Welcome back. Health restored.".to_owned();
                            message_time = 4.0;
                        }
                        WorldEvent::PerfectAttack => {
                            message = "JUST ATTACK / +30% damage".to_owned();
                            message_time = 0.9;
                        }
                        WorldEvent::Evaded => {
                            message = "STEP EVADE".to_owned();
                            message_time = 0.9;
                        }
                        WorldEvent::Gameplay(Event::HunterDefeated) => {
                            message = "Incapacitated / E to retry".to_owned();
                            message_time = 5.0;
                        }
                        WorldEvent::Message(text) => {
                            message = text.to_owned();
                            message_time = 3.0;
                        }
                        _ => {}
                    }
                }
                if world.session().location() != location {
                    camera_target = point(world.position(), 1.0);
                }
                accumulator -= FIXED_STEP;
            }
            hit_flash = (hit_flash - delta).max(0.0);
            message_time = (message_time - delta).max(0.0);
            camera_target =
                camera_target.lerp(point(world.position(), 1.0), 1.0 - (-12.0 * delta).exp());
        }
        if is_key_pressed(KeyCode::F2) {
            bones = !bones;
        }
        hunter.update(&world);
        if !paused {
            feedback.tick(delta, &hunter, &world);
        }
        if world.locked_on() && !paused {
            let p = world.position();
            let e = world.enemy_position();
            let desired = (p.x - e.x).atan2(p.z - e.z);
            let angle = (desired - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            yaw += angle * (1.0 - (-9.0 * delta).exp());
        }
        previous_mouse = mouse;
        let offset = vec3(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        ) * distance;
        let aim = if world.locked_on() {
            camera_target.lerp(point(world.enemy_position(), 1.1), 0.28)
                + vec3(yaw.cos(), 0.0, -yaw.sin()) * 1.2
        } else {
            camera_target + vec3(yaw.cos(), 0.0, -yaw.sin()) * 0.55
        };
        let camera_position = chase_camera(&world, aim, aim + offset);
        clear_background(if world.session().location() == Location::Lobby {
            Color::from_rgba(10, 19, 32, 255)
        } else {
            Color::from_rgba(117, 155, 162, 255)
        });
        let camera = Camera3D {
            position: camera_position,
            target: aim
                + vec3(
                    (world.time() * 130.0).sin(),
                    (world.time() * 99.0).cos(),
                    0.0,
                ) * hit_flash
                    * 0.055,
            up: Vec3::Y,
            fovy: 55.0_f32.to_radians(),
            ..Default::default()
        };
        set_camera(&camera);
        draw_environment(&world, &environment);
        if world.session().location() == Location::TrainingField {
            if let Some(scene) = &mut projection {
                scene.set_grounded_transform(
                    point(world.enemy_position(), 0.0),
                    1.3,
                    world.enemy_facing() - std::f32::consts::FRAC_PI_2,
                );
                motion.update(scene, &world, bones);
            }
        }
        draw_enemy(
            &world,
            projection.as_ref(),
            hit_flash,
            motion.death_visible(world.time()),
        );
        if camera_position.distance(camera_target) > 1.0 {
            draw_player(&world, &hunter, bones);
        }
        feedback.draw_world(&world);
        set_default_camera();
        feedback.draw_screen(&world, &camera);
        hud(
            &world,
            &message,
            message_time,
            external_model,
            motion.status(
                external_model,
                projection.as_ref().is_some_and(|s| s.rig.is_some()),
            ),
        );
        if paused {
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.0, 0.02, 0.04, 0.85),
            );
            if weapon_open {
                if inventory_menu.draw(&mut world) {
                    weapon_open = false;
                    paused = false;
                }
            } else {
                let y = screen_height() / 2.0 - 170.0;
                let label = "SOLO SESSION PAUSED";
                let w = measure_text(label, None, 30, 1.0).width;
                draw_text(label, (screen_width() - w) / 2.0, y, 30.0, ACCENT);
                if button("Resume  /  Enter", y + 30.0) {
                    paused = false;
                }
                if button("Inventory  /  Tab", y + 90.0) {
                    weapon_open = true;
                }
                if button("New session", y + 150.0) {
                    world = World::default();
                    inventory_menu = InventoryMenu::default();
                    hunter = Hunter::default();
                    feedback = Feedback::default();
                    motion.reset();
                    paused = false;
                    accumulator = 0.0;
                    pending_interaction = false;
                    pending_attack = false;
                    pending_dodge = false;
                    pending_jump = false;
                    pending_lock = false;
                    pending_art = false;
                    pending_heal = false;
                    camera_target = point(world.position(), 1.0);
                    message = "New solo session started.".to_owned();
                    message_time = 4.0;
                }
                if button("Quit", y + 210.0) || is_key_pressed(KeyCode::Q) {
                    break;
                }
                let note =
                    "Progress lasts for this session. Saving and multiplayer are not implemented.";
                let w = measure_text(note, None, 17, 1.0).width;
                draw_text(note, (screen_width() - w) / 2.0, y + 286.0, 17.0, LIGHTGRAY);
            }
        }
        // Use a visible in-window pointer for both inventory and pause menus.
        // Hide the system pointer here to avoid drawing two cursors.
        show_mouse(!paused);
        if paused {
            set_cursor_grab(false);
            draw_menu_cursor();
        }
        frames += 1;
        if smoke {
            let complete = world.session().location() == Location::Lobby
                && world.session().quest() == QuestState::Complete;
            if let Some(path) = capture.as_ref() {
                let path = Path::new(path);
                let suffix = if complete {
                    Some("complete")
                } else if !captured_results
                    && world.session().location() == Location::TrainingField
                    && world.session().quest() == QuestState::Complete
                {
                    captured_results = true;
                    Some("results")
                } else if !captured_wave2 && world.session().wave() == 2 {
                    captured_wave2 = true;
                    Some("wave2")
                } else if !captured_wave3 && world.session().wave() == 3 {
                    captured_wave3 = true;
                    Some("wave3")
                } else if !captured_death
                    && world.session().location() == Location::TrainingField
                    && world.session().quest() == QuestState::Complete
                    && motion.death_visible(world.time())
                    && motion.death_age(world.time()).is_some_and(|age| age >= 0.6)
                {
                    captured_death = true;
                    Some("death")
                } else if !captured_hit
                    && world.session().location() == Location::TrainingField
                    && hit_flash > 0.0
                    && world.attack_cooldown() < 0.4
                {
                    captured_hit = true;
                    Some("hit")
                } else if !captured_lobby && frames >= 3 {
                    captured_lobby = true;
                    Some("lobby")
                } else if !captured_field
                    && world.session().location() == Location::TrainingField
                    && world.position().distance(world.enemy_position()) < 3.0
                {
                    captured_field = true;
                    Some("field")
                } else {
                    None
                };
                if let Some(suffix) = suffix {
                    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                    if let Err(error) =
                        capture_frame(&path.with_file_name(format!("{stem}-{suffix}.png")))
                    {
                        eprintln!("Capture failed: {error}");
                        std::process::exit(1);
                    }
                }
            }
            if complete {
                println!(
                    "Solo smoke passed: lobby -> training -> level {} -> lobby",
                    world.session().hunter().level
                );
                break;
            }
            if world.time() > 90.0 {
                eprintln!("Solo smoke timed out");
                std::process::exit(1);
            }
        } else if let Some(path) = capture.as_ref().filter(|_| frames >= 3) {
            if let Err(error) = capture_frame(Path::new(path)) {
                eprintln!("Capture failed: {error}");
                std::process::exit(1);
            }
            break;
        }
        next_frame().await;
    }
}
