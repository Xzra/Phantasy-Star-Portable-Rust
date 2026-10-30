use macroquad::prelude::*;
use psp_assets::Model;

// Stay below Macroquad's default per-draw vertex/index capacities and u16 limits.
const TRIANGLES_PER_MESH: usize = 1500;

fn build_meshes(model: &Model, positions: &[Vec3], texture: Option<&Texture2D>) -> Vec<Mesh> {
    let light = vec3(0.4, 0.8, 0.6).normalize();
    model
        .triangles
        .chunks(TRIANGLES_PER_MESH)
        .enumerate()
        .map(|(chunk, triangles)| {
            let mut mesh = Mesh {
                vertices: Vec::with_capacity(triangles.len() * 3),
                indices: Vec::with_capacity(triangles.len() * 3),
                texture: texture.cloned(),
            };
            for (local, triangle) in triangles.iter().enumerate() {
                let [a, b, c] = triangle.map(|index| positions[index]);
                let normal = (b - a).cross(c - a).normalize_or_zero();
                let intensity = 0.25 + 0.75 * normal.dot(light).max(0.0);
                let color = if texture.is_some() {
                    Color::new(intensity, intensity, intensity, 1.0)
                } else {
                    Color::new(0.35 * intensity, 0.75 * intensity, intensity, 1.0)
                };
                let uvs = model.triangle_uvs[chunk * TRIANGLES_PER_MESH + local];
                for corner in 0..3 {
                    let uv = uvs[corner]
                        .map(|index| model.texcoords[index])
                        .unwrap_or([0.0; 2]);
                    mesh.indices.push(mesh.vertices.len() as u16);
                    mesh.vertices.push(Vertex::new2(
                        positions[triangle[corner]],
                        vec2(uv[0], 1.0 - uv[1]),
                        color,
                    ));
                }
            }
            mesh
        })
        .collect()
}

#[macroquad::main("Phantasy Star Rust — Model Viewer")]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (path, texture_path) = match args.as_slice() {
        [] => (None, None),
        [path] if path != "--help" => (Some(path), None),
        [path, flag, texture] if flag == "--texture" => (Some(path), Some(texture)),
        _ => {
            eprintln!("Usage: model-viewer [model.obj [--texture image.png]]");
            return;
        }
    };
    let source = match path {
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
    let solid_meshes = build_meshes(&model, &vertices, None);
    let textured_meshes = texture
        .as_ref()
        .map(|texture| build_meshes(&model, &vertices, Some(texture)));
    let mut yaw = 0.7_f32;
    let mut pitch = 0.3_f32;
    let mut distance = 4.0_f32;
    let mut rotating = false;
    let mut wireframe = false;
    let mut textured = texture.is_some();
    let mut grid = true;
    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
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
        if grid {
            draw_grid(20, 0.25, GRAY, DARKGRAY);
        }
        if wireframe {
            for triangle in &model.triangles {
                for edge in 0..3 {
                    draw_line_3d(
                        vertices[triangle[edge]],
                        vertices[triangle[(edge + 1) % 3]],
                        SKYBLUE,
                    );
                }
            }
        } else {
            let meshes = if textured {
                textured_meshes.as_ref().unwrap_or(&solid_meshes)
            } else {
                &solid_meshes
            };
            for mesh in meshes {
                draw_mesh(mesh);
            }
        }
        set_default_camera();
        draw_text(
            if wireframe {
                "MODEL VIEWER / wireframe"
            } else {
                "MODEL VIEWER / flat shaded"
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
                vertices.len(),
                model.triangles.len()
            ),
            20.0,
            82.0,
            20.0,
            SKYBLUE,
        );
        draw_text(
            "W: wireframe | T: texture | G: grid",
            20.0,
            106.0,
            20.0,
            LIGHTGRAY,
        );
        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_batches_preserve_triangles_and_stay_within_draw_limits() {
        let model =
            Model::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0.25 0.75\nf 1/1 2/1 3/1").unwrap();
        let positions = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
        let mut large = model;
        large.triangles = vec![[0, 1, 2]; TRIANGLES_PER_MESH + 1];
        large.triangle_uvs = vec![[Some(0); 3]; TRIANGLES_PER_MESH + 1];
        let meshes = build_meshes(&large, &positions, None);
        assert_eq!(meshes.len(), 2);
        assert_eq!(meshes[0].indices.len(), TRIANGLES_PER_MESH * 3);
        assert_eq!(meshes[1].indices, vec![0, 1, 2]);
        assert_eq!(meshes[1].vertices[0].uv, vec2(0.25, 0.25));
        for mesh in meshes {
            assert!(mesh
                .indices
                .iter()
                .all(|index| (*index as usize) < mesh.vertices.len()));
        }
    }

    #[test]
    fn degenerate_faces_produce_finite_vertices() {
        let model = Model::from_obj("v 0 0 0\nf 1 1 1").unwrap();
        let meshes = build_meshes(&model, &[Vec3::ZERO], None);
        assert_eq!(meshes[0].vertices.len(), 3);
        assert!(meshes[0].vertices.iter().all(|v| v.position.is_finite()));
    }
}
