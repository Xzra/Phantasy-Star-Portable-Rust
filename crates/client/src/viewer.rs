use macroquad::prelude::*;
use psp_assets::materials::MaterialLibrary;
use psp_assets::Model;
use std::path::{Path, PathBuf};

// Stay below Macroquad's default per-draw vertex/index capacities and u16 limits.
const TRIANGLES_PER_MESH: usize = 1500;

fn capture_frame(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let frame = get_screen_data();
    let mut image =
        image::RgbaImage::from_raw(frame.width.into(), frame.height.into(), frame.bytes)
            .ok_or("invalid framebuffer dimensions")?;
    image::imageops::flip_vertical_in_place(&mut image);
    image.save_with_format(path, image::ImageFormat::Png)?;
    Ok(())
}

struct Scene {
    model: Model,
    positions: Vec<Vec3>,
    solid: Vec<Mesh>,
    textured: Option<Vec<Mesh>>,
    warnings: Vec<String>,
}

impl Scene {
    fn load(
        path: Option<&Path>,
        texture: Option<&Texture2D>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let imported = match path {
            Some(path) => {
                use std::io::Read;
                let file = std::fs::File::open(path)?;
                if file.metadata()?.len() > 128 * 1024 * 1024 {
                    return Err("model input exceeds 128 MiB".into());
                }
                let mut bytes = Vec::new();
                file.take(128 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
                psp_assets::import::ImportedModel::parse(&bytes)?
            }
            None => psp_assets::import::ImportedModel::parse(include_bytes!(
                "../../../assets/models/training-crystal.obj"
            ))?,
        };
        let model = imported.model;
        let mut materials = MaterialLibrary::default();
        let mut warnings = Vec::new();
        let automatic_texture = if texture.is_none() {
            imported.diffuse_texture.as_ref().and_then(|bytes| {
                match Image::from_file_with_format(bytes, None) {
                    Ok(image) => Some(Texture2D::from_image(&image)),
                    Err(error) => {
                        warnings.push(format!("Cannot decode archive diffuse texture: {error}"));
                        None
                    }
                }
            })
        } else {
            None
        };
        for filename in &model.material_libraries {
            let material_path = path
                .and_then(Path::parent)
                .unwrap_or(Path::new("."))
                .join(filename);
            let source = if path.is_none() && filename == "training-crystal.mtl" {
                Ok(include_str!("../../../assets/models/training-crystal.mtl").to_owned())
            } else {
                std::fs::read_to_string(&material_path)
            };
            match source {
                Ok(source) => match MaterialLibrary::from_mtl(&source) {
                    Ok(library) => materials.materials.extend(library.materials),
                    Err(error) => warnings.push(format!("{}: {error}", material_path.display())),
                },
                Err(error) => warnings.push(format!("{}: {error}", material_path.display())),
            }
        }
        for name in &model.material_names {
            if !materials.materials.contains_key(name) {
                warnings.push(format!(
                    "Material '{name}' is undefined; using fallback color"
                ));
            }
        }
        Ok(Self::from_model(
            model,
            texture.or(automatic_texture.as_ref()),
            &materials,
            warnings,
        ))
    }

    #[cfg(test)]
    fn from_obj(
        source: &str,
        texture: Option<&Texture2D>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::from_model(
            Model::from_obj(source)?,
            texture,
            &MaterialLibrary::default(),
            Vec::new(),
        ))
    }

    fn from_model(
        model: Model,
        texture: Option<&Texture2D>,
        materials: &MaterialLibrary,
        warnings: Vec<String>,
    ) -> Self {
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
        let solid = build_meshes(&model, &positions, None, materials);
        let textured =
            texture.map(|texture| build_meshes(&model, &positions, Some(texture), materials));
        Self {
            model,
            positions,
            solid,
            textured,
            warnings,
        }
    }
}

fn build_meshes(
    model: &Model,
    positions: &[Vec3],
    texture: Option<&Texture2D>,
    materials: &MaterialLibrary,
) -> Vec<Mesh> {
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
                let uvs = model.triangle_uvs[chunk * TRIANGLES_PER_MESH + local];
                let normals = model.triangle_normals[chunk * TRIANGLES_PER_MESH + local];
                let material = model.triangle_materials[chunk * TRIANGLES_PER_MESH + local]
                    .and_then(|index| materials.materials.get(&model.material_names[index]));
                let diffuse =
                    material
                        .map(|material| material.diffuse)
                        .unwrap_or(if texture.is_some() {
                            [1.0; 3]
                        } else {
                            [0.35, 0.75, 1.0]
                        });
                for corner in 0..3 {
                    let shading_normal = normals[corner]
                        .map(|index| {
                            let value = Vec3::from_array(model.normals[index]);
                            // Scale first: normalizing a large finite normal directly can overflow.
                            let scale = value.abs().max_element();
                            if scale > 0.0 {
                                (value / scale).normalize_or_zero()
                            } else {
                                normal
                            }
                        })
                        .unwrap_or(normal);
                    let intensity = 0.25 + 0.75 * shading_normal.dot(light).max(0.0);
                    let color = Color::new(
                        diffuse[0] * intensity,
                        diffuse[1] * intensity,
                        diffuse[2] * intensity,
                        1.0,
                    );
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
        assert!(scene.warnings.is_empty());
        assert!(scene.solid[0].vertices[0].color[1] > scene.solid[0].vertices[0].color[0]);
        assert!(scene.solid[0].vertices[12].color[0] > scene.solid[0].vertices[12].color[1]);
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
        large.triangle_normals = vec![[None; 3]; TRIANGLES_PER_MESH + 1];
        large.triangle_materials = vec![None; TRIANGLES_PER_MESH + 1];
        let meshes = build_meshes(&large, &positions, None, &MaterialLibrary::default());
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
        let meshes = build_meshes(&model, &[Vec3::ZERO], None, &MaterialLibrary::default());
        assert_eq!(meshes[0].vertices.len(), 3);
        assert!(meshes[0].vertices.iter().all(|v| v.position.is_finite()));
    }

    #[test]
    fn authored_normals_shade_corners_and_zero_normals_fall_back() {
        let model =
            Model::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nvn 0 0 -3e38\nvn 0 0 0\nf 1//1 2//2 3")
                .unwrap();
        let meshes = build_meshes(
            &model,
            &[Vec3::ZERO, Vec3::X, Vec3::Y],
            None,
            &MaterialLibrary::default(),
        );
        let vertices = &meshes[0].vertices;
        assert!(vertices[0].color[2] < vertices[1].color[2]);
        assert_eq!(vertices[1].color, vertices[2].color);
    }

    #[test]
    fn loads_relative_material_files_and_keeps_geometry_when_library_is_missing() {
        let directory = std::env::temp_dir().join(format!("psp-materials-{}", std::process::id()));
        std::fs::create_dir_all(directory.join("materials")).unwrap();
        let path = directory.join("sample.obj");
        std::fs::write(
            &path,
            "mtllib materials/color.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl red\nf 1 2 3",
        )
        .unwrap();
        let material_path = directory.join("materials/color.mtl");
        std::fs::write(&material_path, "newmtl red\nKd 1 0 0").unwrap();
        let scene = Scene::load(Some(&path), None).unwrap();
        assert!(scene.warnings.is_empty());
        assert_eq!(scene.solid[0].vertices[0].color[1], 0);
        assert!(scene.solid[0].vertices[0].color[0] > 0);
        std::fs::remove_file(&material_path).unwrap();
        let scene = Scene::load(Some(&path), None).unwrap();
        assert_eq!(scene.model.triangles.len(), 1);
        assert_eq!(scene.warnings.len(), 2);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
