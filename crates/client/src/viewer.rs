use macroquad::prelude::*;
use psp_assets::Model;

#[macroquad::main("Phantasy Star Rust — Model Viewer")]
async fn main() {
    let path = std::env::args().nth(1);
    let source = match path.as_deref() {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("Cannot read {path}: {error}");
                return;
            }
        },
        None => include_str!("../../../assets/models/training-crystal.obj").to_owned(),
    };
    let model = match Model::from_obj(&source) {
        Ok(model) => model,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };
    let (min, max) = model.bounds();
    // Normalize in f64 so even extreme but finite OBJ coordinates fit safely.
    let center = std::array::from_fn::<_, 3, _>(|i| (min[i] as f64 + max[i] as f64) * 0.5);
    let extent = (0..3)
        .map(|i| max[i] as f64 - min[i] as f64)
        .fold(0.0_f64, f64::max)
        .max(0.001);
    let vertices: Vec<Vec3> = model
        .positions
        .iter()
        .map(|p| {
            vec3(
                ((p[0] as f64 - center[0]) / extent * 2.0) as f32,
                ((p[1] as f64 - center[1]) / extent * 2.0) as f32,
                ((p[2] as f64 - center[2]) / extent * 2.0) as f32,
            )
        })
        .collect();
    let mut yaw = 0.7_f32;
    let mut pitch = 0.3_f32;
    let mut distance = 4.0_f32;
    let mut rotating = false;
    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        if is_key_pressed(KeyCode::Space) {
            rotating = !rotating;
        }
        if is_key_pressed(KeyCode::R) {
            yaw = 0.7;
            pitch = 0.3;
            distance = 4.0;
        }
        let dt = get_frame_time().min(0.1);
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
        draw_grid(20, 0.25, GRAY, DARKGRAY);
        for triangle in &model.triangles {
            for edge in 0..3 {
                draw_line_3d(
                    vertices[triangle[edge]],
                    vertices[triangle[(edge + 1) % 3]],
                    SKYBLUE,
                );
            }
        }
        set_default_camera();
        draw_text("MODEL VIEWER / wireframe", 20.0, 30.0, 26.0, WHITE);
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
                vertices.len(),
                model.triangles.len()
            ),
            20.0,
            82.0,
            20.0,
            SKYBLUE,
        );
        next_frame().await;
    }
}
