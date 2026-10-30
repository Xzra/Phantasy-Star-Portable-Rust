use macroquad::prelude::*;
use psp_assets::rig::{skin_point, Rig, Transform};
use psp_assets::{materials::MaterialLibrary, Model};
use std::path::Path;
const TRIANGLES_PER_MESH: usize = 1500;

pub struct Scene {
    pub model: Model,
    pub positions: Vec<Vec3>,
    pub solid: Vec<Mesh>,
    pub textured: Option<Vec<Mesh>>,
    pub warnings: Vec<String>,
    pub rig: Option<Rig>,
    presentation: Mat4,
    solid_colors: Vec<Vec<[u8; 4]>>,
    textured_colors: Option<Vec<Vec<[u8; 4]>>>,
}

impl Scene {
    /// Place normalized static geometry in the game world without changing source data.
    pub fn place_on_ground(&mut self, center: Vec3, scale: f32) {
        let minimum = self
            .positions
            .iter()
            .map(|p| p.y)
            .fold(f32::INFINITY, f32::min);
        let transform = |p: Vec3| {
            vec3(
                p.x * scale + center.x,
                (p.y - minimum) * scale + center.y,
                p.z * scale + center.z,
            )
        };
        self.presentation = Mat4::from_scale_rotation_translation(
            Vec3::splat(scale),
            Quat::IDENTITY,
            center - vec3(0.0, minimum * scale, 0.0),
        ) * self.presentation;
        for position in &mut self.positions {
            *position = transform(*position);
        }
        for mesh in self
            .solid
            .iter_mut()
            .chain(self.textured.iter_mut().flatten())
        {
            for vertex in &mut mesh.vertices {
                vertex.position = transform(vertex.position);
            }
        }
    }

    pub fn draw(&self, textured: bool) {
        let meshes = if textured {
            self.textured.as_ref().unwrap_or(&self.solid)
        } else {
            &self.solid
        };
        for mesh in meshes {
            draw_mesh(mesh);
        }
    }
    pub fn load(
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
        let mut warnings = imported.warnings;
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
        let mut scene = Self::from_model(
            model,
            texture.or(automatic_texture.as_ref()),
            &materials,
            warnings,
        );
        scene.rig = imported.rig;
        Ok(scene)
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

    pub fn from_model(
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
        let solid_colors = solid
            .iter()
            .map(|m| m.vertices.iter().map(|v| v.color).collect())
            .collect();
        let textured_colors = textured.as_ref().map(|meshes: &Vec<Mesh>| {
            meshes
                .iter()
                .map(|m| m.vertices.iter().map(|v| v.color).collect())
                .collect()
        });
        Self {
            rig: None,
            presentation: Mat4::from_scale_rotation_translation(
                Vec3::splat((2.0 / extent) as f32),
                Quat::IDENTITY,
                Vec3::from_array(center.map(|v| (-v * 2.0 / extent) as f32)),
            ),
            solid_colors,
            textured_colors,
            model,
            positions,
            solid,
            textured,
            warnings,
        }
    }
}

impl Scene {
    pub fn set_model_transform(&mut self, transform: Mat4) {
        self.presentation = transform;
    }
    pub fn load_motions(
        &self,
        path: &Path,
    ) -> Result<psp_assets::rig::MotionLibrary, Box<dyn std::error::Error>> {
        use std::io::Read;
        let rig = self
            .rig
            .as_ref()
            .ok_or("model has no supported skeleton/skin")?;
        let file = std::fs::File::open(path)?;
        if file.metadata()?.len() > 128 * 1024 * 1024 {
            return Err("motion archive exceeds 128 MiB".into());
        }
        let mut bytes = Vec::new();
        file.take(128 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        rig.load_clips(&bytes)
    }
    /// Deform source positions every frame, preserving UV seams and material colors.
    pub fn animate(&mut self, pose: &[Transform]) {
        let Some(rig) = &self.rig else {
            return;
        };
        let palette = rig.skeleton.palette(pose);
        let normal_presentation = self.presentation.inverse().transpose();
        let normal_palette: Vec<Mat4> = palette.iter().map(|m| m.inverse().transpose()).collect();
        for ((output, source), skin) in self
            .positions
            .iter_mut()
            .zip(&self.model.positions)
            .zip(&rig.skin)
        {
            *output = self.presentation.transform_point3(skin_point(
                Vec3::from_array(*source),
                skin,
                &palette,
            ));
        }
        let light = vec3(0.4, 0.8, 0.6).normalize();
        for (meshes, colors) in std::iter::once((&mut self.solid, &self.solid_colors))
            .chain(self.textured.iter_mut().zip(self.textured_colors.iter()))
        {
            for (batch, (mesh, colors)) in meshes.iter_mut().zip(colors).enumerate() {
                for (local, vertex) in mesh.vertices.iter_mut().enumerate() {
                    let triangle = batch * TRIANGLES_PER_MESH + local / 3;
                    let corner = local % 3;
                    let indices = self.model.triangles[triangle];
                    let index = indices[corner];
                    let [a, b, c] = indices.map(|i| Vec3::from_array(self.model.positions[i]));
                    let normal = self.model.triangle_normals[triangle][corner]
                        .map(|i| Vec3::from_array(self.model.normals[i]))
                        .unwrap_or_else(|| (b - a).cross(c - a));
                    let magnitude = normal.abs().max_element();
                    let normal = if magnitude > 0.0 {
                        (normal / magnitude).normalize_or_zero()
                    } else {
                        (b - a).cross(c - a).normalize_or_zero()
                    };
                    let influence = &rig.skin[index];
                    let mut deformed = Vec3::ZERO;
                    for (joint, weight) in influence.joints.iter().zip(influence.weights) {
                        if weight > 0.0 {
                            deformed +=
                                normal_palette[*joint as usize].transform_vector3(normal) * weight;
                        }
                    }
                    if influence.weights.iter().all(|w| *w == 0.0) {
                        deformed = normal;
                    }
                    let old = 0.25 + 0.75 * normal.dot(light).max(0.0);
                    let new = 0.25
                        + 0.75
                            * normal_presentation
                                .transform_vector3(deformed)
                                .normalize_or_zero()
                                .dot(light)
                                .max(0.0);
                    vertex.position = self.positions[index];
                    vertex.color = std::array::from_fn(|i| {
                        if i == 3 {
                            colors[local][3]
                        } else {
                            (colors[local][i] as f32 * new / old).clamp(0.0, 255.0) as u8
                        }
                    });
                }
            }
        }
    }
    pub fn draw_skeleton(&self, pose: &[Transform]) {
        let Some(rig) = &self.rig else {
            return;
        };
        let globals = rig.skeleton.globals(pose);
        for (i, bone) in rig.skeleton.bones.iter().enumerate() {
            let p = self
                .presentation
                .transform_point3(globals[i].transform_point3(Vec3::ZERO));
            draw_sphere(p, 0.025, None, YELLOW);
            if let Some(parent) = bone.parent {
                let parent = self
                    .presentation
                    .transform_point3(globals[parent].transform_point3(Vec3::ZERO));
                draw_line_3d(parent, p, LIME);
            }
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

pub fn capture_frame(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let frame = get_screen_data();
    let mut image =
        image::RgbaImage::from_raw(frame.width.into(), frame.height.into(), frame.bytes)
            .ok_or("invalid framebuffer dimensions")?;
    image::imageops::flip_vertical_in_place(&mut image);
    image.save_with_format(path, image::ImageFormat::Png)?;
    Ok(())
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
