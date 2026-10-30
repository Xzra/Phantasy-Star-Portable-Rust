use macroquad::prelude::*;
use psp_assets::Model;
use std::path::{Path, PathBuf};

// Stay below Macroquad's default per-draw vertex/index capacities and u16 limits.
const TRIANGLES_PER_MESH: usize = 1500;

struct Scene {
    model: Model,
    positions: Vec<Vec3>,
    solid: Vec<Mesh>,
    textured: Option<Vec<Mesh>>,
}

impl Scene {
    fn load(
        path: Option<&Path>,
        texture: Option<&Texture2D>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let source = match path {
            Some(path) => std::fs::read_to_string(path)?,
            None => include_str!("../../../assets/models/training-crystal.obj").to_owned(),
        };
        Self::from_obj(&source, texture)
    }

    fn from_obj(
        source: &str,
        texture: Option<&Texture2D>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let model = Model::from_obj(source)?;
        let (min, max) = model.bounds();
        // Use f64 intermediates so extreme finite source coordinates cannot overflow.
        let center = std::array::from_fn::<_, 3, _>(|i| (min[i] as f64 + max[i] as f64) * 0.5);
        let extent = (0..3)
            .map(|i| max[i] as f64 - min[i] as f64)
            .fold(0.0_f64, f64::max)
            .max(0.001);
        let positions: Vec<Vec3> = model
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
        let solid = build_meshes(&model, &positions, None);
        let textured = texture.map(|texture| build_meshes(&model, &positions, Some(texture)));
        Ok(Self {
            model,
            positions,
            solid,
            textured,
        })
    }
}

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
    let mut import_error = String::new();
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
                    scene = replacement;
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
                scene.positions.len(),
                scene.model.triangles.len()
            ),
            20.0,
            82.0,
            20.0,
            SKYBLUE,
        );
        draw_text(
            "W: wireframe | T: texture | G: grid | L: reload | Drop an OBJ to open",
            20.0,
            106.0,
            20.0,
            LIGHTGRAY,
        );
        if !import_error.is_empty() {
            let message: String = import_error.chars().take(120).collect();
            draw_text(&message, 20.0, 132.0, 18.0, ORANGE);
        }
        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_extreme_finite_coordinates_without_overflow() {
        let scene = Scene::from_obj("v -3e38 0 0\nv 3e38 0 0\nv 0 3e38 0\nf 1 2 3", None).unwrap();
        assert_eq!(scene.positions[0].x, -1.0);
        assert_eq!(scene.positions[1].x, 1.0);
        assert!(scene.positions.iter().all(|p| p.is_finite()));
    }

    #[test]
    fn original_sample_loads_and_malformed_replacements_are_rejected() {
        let scene = Scene::load(None, None).unwrap();
        assert_eq!(scene.model.triangles.len(), 8);
        assert_eq!(scene.model.texcoords.len(), 3);
        assert!(Scene::from_obj("v NaN 0 0", None).is_err());
        // Constructing a replacement does not mutate the currently loaded scene.
        assert_eq!(scene.model.triangles.len(), 8);
    }

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
