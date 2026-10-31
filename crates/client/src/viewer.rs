use macroquad::prelude::*;
use psp_client::scene::{capture_frame, Scene};
use std::path::PathBuf;

#[macroquad::main("Phantasy Star Rust — Model Viewer")]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut path = None;
    let mut texture_path = None;
    let mut motion_path = None;
    let mut selected_clip = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--texture" => texture_path = args.next(),
            "--motions" => motion_path = args.next(),
            "--clip" => selected_clip = args.next(),
            value if !value.starts_with('-') && path.is_none() => path = Some(arg),
            _ => {
                eprintln!("Usage: model-viewer [model] [--texture image] [--motions archive.ice] [--clip name]");
                return;
            }
        }
    }
    let texture = if let Some(path) = texture_path {
        match load_texture(path).await {
            Ok(texture) => Some(texture),
            Err(error) => {
                eprintln!("Cannot load texture {path}: {error}");
                return;
            }
        }
    } else {
        None
    };
    let mut current_path = path.map(PathBuf::from);
    let mut scene = match Scene::load(current_path.as_deref(), texture.as_ref()) {
        Ok(scene) => scene,
        Err(error) => {
            eprintln!("Cannot load model: {error}");
            return;
        }
    };
    let mut clips = motion_path
        .and_then(
            |path| match scene.load_motions(std::path::Path::new(path)) {
                Ok(library) => {
                    for warning in library.warnings {
                        eprintln!("{warning}");
                    }
                    Some(library.clips)
                }
                Err(error) => {
                    eprintln!("Cannot load motions: {error}");
                    None
                }
            },
        )
        .unwrap_or_default();
    let mut clip_index = selected_clip
        .and_then(|name| clips.iter().position(|c| c.name.contains(name)))
        .unwrap_or(0);
    let mut animation_playing = !clips.is_empty();
    let mut animation_time = 0.0;
    let mut skeleton_visible = false;
    let capture_time = std::env::var("PSP_ANIMATION_TIME")
        .ok()
        .and_then(|v| v.parse::<f32>().ok());
    let mut import_error = String::new();
    let mut yaw = 0.7_f32;
    let mut pitch = 0.3_f32;
    let mut distance = 4.0_f32;
    let mut rotating = false;
    let mut wireframe = false;
    let mut textured = scene.textured.is_some();
    let mut grid = true;
    // Capture only our own framebuffer; useful for automated graphics smoke checks.
    let capture_path = std::env::var_os("PSP_VIEWER_CAPTURE").map(PathBuf::from);
    let mut rendered_frames = 0;
    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        let requested_path = get_dropped_files()
            .into_iter()
            .find_map(|file| file.path)
            .or_else(|| {
                if is_key_pressed(KeyCode::L) {
                    current_path.clone()
                } else {
                    None
                }
            });
        if let Some(path) = requested_path {
            match Scene::load(Some(&path), texture.as_ref()) {
                Ok(replacement) => {
                    textured = replacement.textured.is_some();
                    scene = replacement;
                    clips = motion_path
                        .and_then(|p| scene.load_motions(std::path::Path::new(p)).ok())
                        .map(|l| l.clips)
                        .unwrap_or_default();
                    clip_index = 0;
                    animation_time = 0.0;
                    animation_playing = !clips.is_empty();
                    current_path = Some(path);
                    import_error.clear();
                    yaw = 0.7;
                    pitch = 0.3;
                    distance = 4.0;
                }
                Err(error) => {
                    import_error = format!("Cannot load {}: {error}", path.display());
                }
            }
        }
        if is_key_pressed(KeyCode::Space) {
            rotating = !rotating;
        }
        if is_key_pressed(KeyCode::W) {
            wireframe = !wireframe;
        }
        if is_key_pressed(KeyCode::T) {
            textured = !textured;
        }
        if is_key_pressed(KeyCode::G) {
            grid = !grid;
        }
        if is_key_pressed(KeyCode::R) {
            yaw = 0.7;
            pitch = 0.3;
            distance = 4.0;
        }
        if is_key_pressed(KeyCode::A) {
            animation_playing = !animation_playing;
        }
        if is_key_pressed(KeyCode::B) {
            skeleton_visible = !skeleton_visible;
        }
        if is_key_pressed(KeyCode::N) && !clips.is_empty() {
            clip_index = (clip_index + 1) % clips.len();
            animation_time = 0.0;
        }
        let dt = get_frame_time().min(0.1);
        if animation_playing {
            animation_time += dt;
        }
        let pose = clips.get(clip_index).and_then(|clip| {
            scene
                .rig
                .as_ref()
                .map(|rig| clip.sample(&rig.skeleton, capture_time.unwrap_or(animation_time), true))
        });
        if let Some(pose) = &pose {
            scene.animate(pose);
        }
        if rotating || is_key_down(KeyCode::Right) {
            yaw += dt;
        }
        if is_key_down(KeyCode::Left) {
            yaw -= dt;
        }
        if is_key_down(KeyCode::Up) {
            pitch += dt;
        }
        if is_key_down(KeyCode::Down) {
            pitch -= dt;
        }
        pitch = pitch.clamp(-1.4, 1.4);
        distance = (distance - mouse_wheel().1 * 0.25).clamp(1.8, 15.0);
        clear_background(Color::from_rgba(14, 19, 30, 255));
        set_camera(&Camera3D {
            position: vec3(
                yaw.sin() * pitch.cos(),
                pitch.sin(),
                yaw.cos() * pitch.cos(),
            ) * distance,
            up: Vec3::Y,
            target: Vec3::ZERO,
            ..Default::default()
        });
        if grid {
            // Normalized geometry fits in [-1, 1]; keep the ground below it.
            draw_grid_ex(
                20,
                0.25,
                GRAY,
                DARKGRAY,
                vec3(0.0, -1.05, 0.0),
                Quat::IDENTITY,
            );
        }
        if wireframe {
            for triangle in &scene.model.triangles {
                for edge in 0..3 {
                    draw_line_3d(
                        scene.positions[triangle[edge]],
                        scene.positions[triangle[(edge + 1) % 3]],
                        SKYBLUE,
                    );
                }
            }
        } else {
            let meshes = if textured {
                scene.textured.as_ref().unwrap_or(&scene.solid)
            } else {
                &scene.solid
            };
            for mesh in meshes {
                draw_mesh(mesh);
            }
        }
        if skeleton_visible {
            if let Some(rig) = &scene.rig {
                scene.draw_skeleton(pose.as_deref().unwrap_or(&rig.skeleton.bind_pose()));
            }
        }
        set_default_camera();
        draw_text(
            if wireframe {
                "MODEL VIEWER / wireframe"
            } else {
                "MODEL VIEWER / shaded"
            },
            20.0,
            30.0,
            26.0,
            WHITE,
        );
        draw_text(
            "Arrows: orbit | Wheel: zoom | Space: rotate | R: reset | Esc: quit",
            20.0,
            58.0,
            20.0,
            LIGHTGRAY,
        );
        draw_text(
            format!(
                "{} vertices / {} triangles",
                scene.positions.len(),
                scene.model.triangles.len()
            ),
            20.0,
            82.0,
            20.0,
            SKYBLUE,
        );
        draw_text(
            "W: wireframe | T: texture | G: grid | L: reload | Drop OBJ, AQP, or ICE",
            20.0,
            106.0,
            20.0,
            LIGHTGRAY,
        );
        if let Some(clip) = clips.get(clip_index) {
            draw_text(
                format!(
                    "A: play/pause | N: next | B: bones | {} ({:.2}s)",
                    clip.name, clip.duration
                ),
                20.0,
                screen_height() - 24.0,
                18.0,
                LIME,
            );
        } else if let Some(rig) = &scene.rig {
            draw_text(
                format!(
                    "{} bones | B: skeleton | Supply --motions archive.ice for animation",
                    rig.skeleton.bones.len()
                ),
                20.0,
                screen_height() - 24.0,
                18.0,
                LIME,
            );
        }
        if !import_error.is_empty() {
            let message: String = import_error.chars().take(120).collect();
            draw_text(&message, 20.0, 132.0, 18.0, ORANGE);
        }
        if let Some(warning) = scene.warnings.first() {
            let message: String =
                format!("{} material warning(s): {warning}", scene.warnings.len())
                    .chars()
                    .take(120)
                    .collect();
            draw_text(&message, 20.0, 156.0, 18.0, YELLOW);
        }
        rendered_frames += 1;
        if let Some(path) = capture_path.as_ref().filter(|_| rendered_frames >= 3) {
            match capture_frame(path) {
                Ok(()) => println!("Captured viewer frame: {}", path.display()),
                Err(error) => {
                    eprintln!("Cannot capture frame: {error}");
                    std::process::exit(1);
                }
            }
            break;
        }
        next_frame().await;
    }
}
