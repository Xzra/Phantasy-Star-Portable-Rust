use macroquad::prelude::*;
use psp_client::scene::{capture_frame, Scene};
use psp_core::{
    world::{Input, Position, World, WorldEvent, ENEMY, FIXED_STEP, RETURN_PAD, TERMINAL},
    Event, Location, QuestState,
};
use std::path::Path;

const INK: Color = Color::new(0.035, 0.065, 0.11, 1.0);
const ACCENT: Color = Color::new(0.15, 0.85, 0.95, 1.0);
const PANEL: Color = Color::new(0.035, 0.07, 0.12, 0.94);

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
    let (goal, interact, attack) = match (world.session().location(), world.session().quest()) {
        (Location::Lobby, _) => (TERMINAL, true, false),
        (Location::TrainingField, QuestState::Active) => (
            Position {
                x: ENEMY.x,
                z: ENEMY.z + 2.0,
            },
            false,
            true,
        ),
        _ => (RETURN_PAD, true, false),
    };
    let distance = world.position().distance(goal);
    Input {
        movement: if distance > 0.15 {
            Position {
                x: goal.x - world.position().x,
                z: goal.z - world.position().z,
            }
        } else {
            Position::default()
        },
        sprint: true,
        interact: interact && distance < 2.0,
        attack: attack && distance < 0.2,
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

fn draw_environment(world: &World) {
    let lobby = world.session().location() == Location::Lobby;
    let size = world.half_extent() * 2.0;
    let floor = if lobby {
        Color::from_rgba(92, 113, 132, 255)
    } else {
        Color::from_rgba(55, 72, 83, 255)
    };
    draw_cube(vec3(0.0, -0.18, 0.0), vec3(size, 0.36, size), None, floor);
    for i in -(world.half_extent() as i32)..=(world.half_extent() as i32) {
        let shade = Color::new(0.23, 0.33, 0.4, 1.0);
        draw_line_3d(
            vec3(i as f32, 0.012, -size / 2.0),
            vec3(i as f32, 0.012, size / 2.0),
            shade,
        );
        draw_line_3d(
            vec3(-size / 2.0, 0.012, i as f32),
            vec3(size / 2.0, 0.012, i as f32),
            shade,
        );
    }
    for side in [-1.0, 1.0] {
        draw_cube(
            vec3(side * (size / 2.0 + 0.2), 0.65, 0.0),
            vec3(0.4, 1.3, size + 0.4),
            None,
            INK,
        );
        draw_cube(
            vec3(0.0, 0.65, side * (size / 2.0 + 0.2)),
            vec3(size + 0.4, 1.3, 0.4),
            None,
            INK,
        );
        draw_cube(
            vec3(side * (size / 2.0), 1.32, 0.0),
            vec3(0.08, 0.06, size),
            None,
            ACCENT,
        );
        draw_cube(
            vec3(0.0, 1.32, side * (size / 2.0)),
            vec3(size, 0.06, 0.08),
            None,
            ACCENT,
        );
    }
    for (i, obstacle) in world.obstacles().iter().enumerate() {
        let height = obstacle_height(world, i);
        draw_cube(
            point(obstacle.center, height / 2.0),
            vec3(obstacle.half.x * 2.0, height, obstacle.half.z * 2.0),
            None,
            INK,
        );
        draw_cube(
            point(obstacle.center, height + 0.03),
            vec3(obstacle.half.x * 2.0, 0.06, obstacle.half.z * 2.0),
            None,
            if lobby { ACCENT } else { ORANGE },
        );
    }
    if lobby {
        draw_pad(TERMINAL, ACCENT, true, world.time());
        draw_cube(point(TERMINAL, 0.9), vec3(1.2, 1.8, 0.7), None, INK);
        draw_cube(
            point(TERMINAL, 1.55) + vec3(0.0, 0.0, 0.38),
            vec3(0.95, 0.65, 0.05),
            None,
            ACCENT,
        );
        draw_sphere(point(TERMINAL, 2.45), 0.24, None, SKYBLUE);
        for i in -5..5 {
            draw_cube(
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
            Color::new(0.08, 0.25, 0.38, 1.0),
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

fn draw_player(world: &World) {
    let p = world.position();
    let facing = world.facing();
    let forward = vec3(facing.sin(), 0.0, facing.cos());
    let right = vec3(facing.cos(), 0.0, -facing.sin());
    let stride = (world.distance_walked() * 5.5).sin() * 0.18;
    for side in [-1.0, 1.0] {
        draw_cube(
            point(p, 0.31) + right * (side * 0.18) + forward * (stride * side),
            vec3(0.24, 0.6, 0.28),
            None,
            INK,
        );
        draw_cube(
            point(p, 1.06) + right * (side * 0.43) - forward * (stride * side),
            vec3(0.22, 0.65, 0.26),
            None,
            Color::new(0.12, 0.38, 0.5, 1.0),
        );
    }
    draw_cube(
        point(p, 1.02),
        vec3(0.64, 0.78, 0.4),
        None,
        Color::new(0.12, 0.52, 0.65, 1.0),
    );
    draw_cube(
        point(p, 1.18) + forward * 0.23,
        vec3(0.22, 0.2, 0.1),
        None,
        ACCENT,
    );
    draw_sphere(
        point(p, 1.65),
        0.27,
        None,
        Color::new(0.13, 0.18, 0.25, 1.0),
    );
    draw_sphere(point(p, 1.68) + forward * 0.23, 0.13, None, YELLOW);
    let cooldown = world.attack_cooldown();
    if cooldown > 0.24 {
        let progress = (0.55 - cooldown) / 0.31;
        for i in 0..9 {
            let angle = facing - 1.2 + progress * 2.4 - i as f32 * 0.045;
            let direction = vec3(angle.sin(), 0.0, angle.cos());
            draw_line_3d(
                point(p, 1.0) + direction * 0.6,
                point(p, 1.0) + direction * 2.1,
                if i < 3 { WHITE } else { ACCENT },
            );
        }
    } else {
        draw_cube(
            point(p, 0.93) + right * 0.53,
            vec3(0.07, 1.05, 0.07),
            None,
            ACCENT,
        );
    }
}

fn draw_enemy(world: &World, model: Option<&Scene>, hit_flash: f32) {
    if world.session().location() != Location::TrainingField
        || world.session().quest() != QuestState::Active
    {
        return;
    }
    if let Some(model) = model {
        model.draw(true);
    } else {
        let bob = (world.time() * 2.0).sin() * 0.1;
        draw_sphere(
            point(ENEMY, 1.1 + bob),
            0.65,
            None,
            Color::new(0.18, 0.24, 0.32, 1.0),
        );
        draw_cube(point(ENEMY, 1.1 + bob), vec3(1.7, 0.22, 0.35), None, ORANGE);
        draw_sphere(
            point(ENEMY, 1.15 + bob) + vec3(0.0, 0.0, 0.55),
            0.18,
            None,
            RED,
        );
    }
    if hit_flash > 0.0 {
        for i in 0..10 {
            let angle = i as f32 * std::f32::consts::TAU / 10.0;
            draw_line_3d(
                point(ENEMY, 0.9),
                point(ENEMY, 0.9)
                    + vec3(angle.cos(), (angle * 2.0).sin(), angle.sin()) * (1.0 - hit_flash) * 1.7,
                YELLOW,
            );
        }
    }
}

fn hud(world: &World, message: &str, message_time: f32, projection: bool) {
    let width = screen_width();
    let height = screen_height();
    let hunter = world.session().hunter();
    draw_rectangle(18.0, 16.0, 300.0, 111.0, PANEL);
    draw_text("PHANTASY STAR / SOLO", 32.0, 42.0, 23.0, ACCENT);
    draw_text(
        format!(
            "HUNTER    LV {}    XP {} / {}",
            hunter.level,
            hunter.experience,
            hunter.level * 100
        ),
        32.0,
        68.0,
        20.0,
        WHITE,
    );
    draw_rectangle(32.0, 81.0, 270.0, 12.0, Color::new(0.15, 0.2, 0.25, 1.0));
    draw_rectangle(
        32.0,
        81.0,
        270.0 * hunter.health as f32 / hunter.max_health() as f32,
        12.0,
        GREEN,
    );
    draw_text(
        format!("HP {} / {}", hunter.health, hunter.max_health()),
        32.0,
        114.0,
        18.0,
        LIGHTGRAY,
    );
    let location = if world.session().location() == Location::Lobby {
        "ORBITAL HUB"
    } else {
        "HUNTER TRAINING FIELD"
    };
    let measured = measure_text(location, None, 25, 1.0);
    draw_text(location, width - measured.width - 24.0, 42.0, 25.0, WHITE);
    let objective = match (world.session().location(), world.session().quest()) {
        (Location::Lobby, QuestState::Available) => {
            ("Reach the blue terminal.", "Press E to begin training.")
        }
        (Location::Lobby, _) => ("Training complete.", "Replay: Esc > New session."),
        (Location::TrainingField, QuestState::Active) => {
            ("Approach and face the target.", "Space / LMB to attack.")
        }
        _ => ("Training cleared.", "Return pad: approach, press E."),
    };
    draw_rectangle(18.0, 140.0, 300.0, 80.0, PANEL);
    draw_text("OBJECTIVE", 32.0, 162.0, 17.0, ACCENT);
    draw_text(objective.0, 32.0, 186.0, 18.0, WHITE);
    draw_text(objective.1, 32.0, 207.0, 18.0, WHITE);
    if world.session().location() == Location::TrainingField
        && world.session().quest() == QuestState::Active
    {
        let label = if projection {
            "DAGAN / TRAINING PROJECTION"
        } else {
            "TRAINING DRONE"
        };
        let w = measure_text(label, None, 20, 1.0).width;
        let center = width - 188.0;
        draw_rectangle(center - 170.0, 66.0, 340.0, 50.0, PANEL);
        draw_text(label, center - w / 2.0, 87.0, 20.0, ORANGE);
        draw_rectangle(center - 150.0, 96.0, 300.0, 8.0, DARKGRAY);
        draw_rectangle(
            center - 150.0,
            96.0,
            300.0 * world.session().enemy_health() as f32 / 36.0,
            8.0,
            ORANGE,
        );
    }
    draw_rectangle(18.0, height - 68.0, width - 36.0, 50.0, PANEL);
    draw_text(
        "WASD: move    Shift: sprint    E: interact    Space / LMB: attack",
        32.0,
        height - 46.0,
        18.0,
        LIGHTGRAY,
    );
    draw_text(
        "RMB drag / arrows: camera    Wheel: zoom    Esc: pause",
        32.0,
        height - 25.0,
        18.0,
        LIGHTGRAY,
    );
    if let Some(prompt) = world.interaction_prompt() {
        let w = measure_text(prompt, None, 22, 1.0).width;
        draw_rectangle(
            width / 2.0 - w / 2.0 - 18.0,
            height - 126.0,
            w + 36.0,
            42.0,
            PANEL,
        );
        draw_text(prompt, width / 2.0 - w / 2.0, height - 98.0, 22.0, ACCENT);
    }
    if message_time > 0.0 {
        let w = measure_text(message, None, 24, 1.0).width;
        draw_text(message, (width - w) / 2.0, height - 153.0, 24.0, YELLOW);
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
    let mut world = World::default();
    let candidate = std::env::args().nth(1).or_else(|| {
        ["local/dagan.ice", "local/models/dagan/dagan.ice"]
            .into_iter()
            .find(|path| Path::new(path).is_file())
            .map(str::to_owned)
    });
    let mut projection =
        candidate.and_then(|path| match Scene::load(Some(Path::new(&path)), None) {
            Ok(scene) => Some(scene),
            Err(error) => {
                eprintln!("Optional projection unavailable: {error}");
                None
            }
        });
    if let Some(scene) = &mut projection {
        scene.place_on_ground(point(ENEMY, 0.0), 1.3);
    }
    let mut paused = false;
    let mut accumulator = 0.0;
    let mut pending_interaction = false;
    let mut yaw = 0.0_f32;
    let mut pitch = 0.6_f32;
    let mut distance = 7.0_f32;
    let mut camera_target = point(world.position(), 1.0);
    let mut previous_mouse = mouse_position();
    let mut message = "Welcome aboard. Follow the blue lights to the mission terminal.".to_owned();
    let mut message_time = 6.0_f32;
    let mut hit_flash = 0.0_f32;
    let capture = std::env::var_os("PSP_GAME_CAPTURE");
    let smoke = std::env::var_os("PSP_GAME_SMOKE").is_some();
    let mut captured_lobby = false;
    let mut captured_field = false;
    let mut block_attack = false;
    let mut frames = 0;
    loop {
        let delta = get_frame_time().clamp(0.0, 0.1);
        if is_key_pressed(KeyCode::Escape) {
            paused = !paused;
            accumulator = 0.0;
            pending_interaction = false;
        }
        if paused && is_key_pressed(KeyCode::Enter) {
            paused = false;
        }
        if paused {
            block_attack = true;
        }
        if !is_key_down(KeyCode::Space) && !is_mouse_button_down(MouseButton::Left) {
            block_attack = false;
        }
        let mouse = mouse_position();
        if !paused {
            if is_mouse_button_down(MouseButton::Right) {
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
                            && (is_key_down(KeyCode::Space)
                                || is_mouse_button_down(MouseButton::Left)),
                    }
                };
                let events = world.step(input, FIXED_STEP);
                pending_interaction = false;
                for event in events {
                    match event {
                        WorldEvent::Gameplay(Event::QuestStarted) => {
                            message =
                                "Hunter training started. Approach the projection.".to_owned();
                            message_time = 4.0;
                        }
                        WorldEvent::Gameplay(Event::EnemyDamaged { .. }) => {
                            hit_flash = 0.35;
                        }
                        WorldEvent::Gameplay(Event::QuestCompleted { .. }) => {
                            message = format!(
                                "TRAINING CLEAR  /  Level {}  /  +100 XP",
                                world.session().hunter().level
                            );
                            message_time = 5.0;
                        }
                        WorldEvent::Gameplay(Event::ReturnedToLobby) => {
                            message = "Welcome back. Health restored.".to_owned();
                            message_time = 4.0;
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
        previous_mouse = mouse;
        let offset = vec3(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        ) * distance;
        let aim = camera_target + vec3(yaw.cos(), 0.0, -yaw.sin()) * 0.9;
        let camera_position = chase_camera(&world, aim, aim + offset);
        clear_background(Color::from_rgba(10, 19, 32, 255));
        set_camera(&Camera3D {
            position: camera_position,
            target: aim,
            up: Vec3::Y,
            ..Default::default()
        });
        draw_environment(&world);
        draw_enemy(&world, projection.as_ref(), hit_flash);
        if camera_position.distance(camera_target) > 1.0 {
            draw_player(&world);
        }
        set_default_camera();
        hud(&world, &message, message_time, projection.is_some());
        if paused {
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.0, 0.02, 0.04, 0.85),
            );
            let y = screen_height() / 2.0 - 140.0;
            let label = "SOLO SESSION PAUSED";
            let w = measure_text(label, None, 30, 1.0).width;
            draw_text(label, (screen_width() - w) / 2.0, y, 30.0, ACCENT);
            if button("Resume  /  Enter", y + 30.0) {
                paused = false;
            }
            if button("New session", y + 90.0) {
                world = World::default();
                paused = false;
                accumulator = 0.0;
                pending_interaction = false;
                camera_target = point(world.position(), 1.0);
                message = "New solo session started.".to_owned();
                message_time = 4.0;
            }
            if button("Quit", y + 150.0) || is_key_pressed(KeyCode::Q) {
                break;
            }
            let note =
                "Progress lasts for this session. Saving and multiplayer are not implemented.";
            let w = measure_text(note, None, 17, 1.0).width;
            draw_text(note, (screen_width() - w) / 2.0, y + 226.0, 17.0, LIGHTGRAY);
        }
        frames += 1;
        if smoke {
            let complete = world.session().location() == Location::Lobby
                && world.session().quest() == QuestState::Complete;
            if let Some(path) = capture.as_ref() {
                let path = Path::new(path);
                let suffix = if complete {
                    Some("complete")
                } else if !captured_lobby && frames >= 3 {
                    captured_lobby = true;
                    Some("lobby")
                } else if !captured_field
                    && world.session().location() == Location::TrainingField
                    && world.position().distance(ENEMY) < 3.0
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
            if world.time() > 30.0 {
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
