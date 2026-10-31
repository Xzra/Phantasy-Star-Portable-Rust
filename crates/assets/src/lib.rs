//! Small OBJ position/face importer. Proprietary game formats belong in adapters.
use std::fmt;

pub mod aqua;
pub mod catalog;
pub mod ice;
pub mod ice_decode;
pub mod import;
pub mod land;
pub mod materials;
pub mod rig;

#[derive(Debug, Default, PartialEq)]
pub struct Model {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[usize; 3]>,
    pub texcoords: Vec<[f32; 2]>,
    /// UV indices per triangle corner; OBJ can share positions across UV seams.
    pub triangle_uvs: Vec<[Option<usize>; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// Authored normals are indexed per corner so hard edges remain distinct.
    pub triangle_normals: Vec<[Option<usize>; 3]>,
    pub material_libraries: Vec<String>,
    pub material_names: Vec<String>,
    pub triangle_materials: Vec<Option<usize>>,
}

#[derive(Debug, PartialEq)]
pub struct ImportError {
    pub line: usize,
    pub message: &'static str,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OBJ line {}: {}", self.line, self.message)
    }
}
impl std::error::Error for ImportError {}

impl Model {
    /// Imports geometry and material assignments. Companion MTL files are resolved
    /// separately; this parser does no file I/O.
    pub fn from_obj(source: &str) -> Result<Self, ImportError> {
        let mut model = Self {
            positions: Vec::new(),
            triangles: Vec::new(),
            texcoords: Vec::new(),
            triangle_uvs: Vec::new(),
            normals: Vec::new(),
            triangle_normals: Vec::new(),
            material_libraries: Vec::new(),
            material_names: Vec::new(),
            triangle_materials: Vec::new(),
        };
        let mut current_material = None;
        for (offset, line) in source.lines().enumerate() {
            let error = |message| ImportError {
                line: offset + 1,
                message,
            };
            let mut fields = line.split('#').next().unwrap_or("").split_whitespace();
            match fields.next() {
                Some("v") => {
                    let mut position = [0.0_f32; 3];
                    for value in &mut position {
                        *value = fields
                            .next()
                            .ok_or_else(|| error("vertex needs three coordinates"))?
                            .parse()
                            .map_err(|_| error("invalid coordinate"))?;
                        if !value.is_finite() {
                            return Err(error("coordinate must be finite"));
                        }
                    }
                    model.positions.push(position);
                }
                Some("f") => {
                    let indices = fields
                        .map(|field| {
                            let mut parts = field.split('/');
                            let position =
                                resolve_index(parts.next().unwrap_or(""), model.positions.len())
                                    .map_err(error)?;
                            let uv = match parts.next() {
                                Some("") | None => None,
                                Some(raw) => {
                                    Some(resolve_index(raw, model.texcoords.len()).map_err(error)?)
                                }
                            };
                            let normal = match parts.next() {
                                Some("") | None => None,
                                Some(raw) => {
                                    Some(resolve_index(raw, model.normals.len()).map_err(error)?)
                                }
                            };
                            if parts.next().is_some() {
                                return Err(error("face corner has too many indices"));
                            }
                            Ok((position, uv, normal))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if indices.len() < 3 {
                        return Err(error("face needs at least three vertices"));
                    }
                    for i in 1..indices.len() - 1 {
                        model
                            .triangles
                            .push([indices[0].0, indices[i].0, indices[i + 1].0]);
                        model
                            .triangle_uvs
                            .push([indices[0].1, indices[i].1, indices[i + 1].1]);
                        model
                            .triangle_normals
                            .push([indices[0].2, indices[i].2, indices[i + 1].2]);
                        model.triangle_materials.push(current_material);
                    }
                }
                Some("vt") => {
                    let mut uv = [0.0_f32; 2];
                    for coordinate in &mut uv {
                        *coordinate = fields
                            .next()
                            .ok_or_else(|| error("UV needs two coordinates"))?
                            .parse()
                            .map_err(|_| error("invalid UV coordinate"))?;
                        if !coordinate.is_finite() {
                            return Err(error("UV coordinate must be finite"));
                        }
                    }
                    model.texcoords.push(uv);
                }
                Some("vn") => {
                    let mut normal = [0.0_f32; 3];
                    for coordinate in &mut normal {
                        *coordinate = fields
                            .next()
                            .ok_or_else(|| error("normal needs three coordinates"))?
                            .parse()
                            .map_err(|_| error("invalid normal coordinate"))?;
                        if !coordinate.is_finite() {
                            return Err(error("normal coordinate must be finite"));
                        }
                    }
                    model.normals.push(normal);
                }
                Some("mtllib") => {
                    let libraries: Vec<_> = fields.map(str::to_owned).collect();
                    if libraries.is_empty() {
                        return Err(error("material library name is missing"));
                    }
                    model.material_libraries.extend(libraries);
                }
                Some("usemtl") => {
                    let name = fields.collect::<Vec<_>>().join(" ");
                    if name.is_empty() {
                        return Err(error("material name is missing"));
                    }
                    let index = model
                        .material_names
                        .iter()
                        .position(|existing| *existing == name)
                        .unwrap_or_else(|| {
                            model.material_names.push(name);
                            model.material_names.len() - 1
                        });
                    current_material = Some(index);
                }
                _ => {}
            }
        }
        if model.positions.is_empty() || model.triangles.is_empty() {
            return Err(ImportError {
                line: 0,
                message: "model needs vertices and faces",
            });
        }
        Ok(model)
    }

    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for position in &self.positions {
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        (min, max)
    }
}

fn resolve_index(raw: &str, count: usize) -> Result<usize, &'static str> {
    let raw: i64 = raw.parse().map_err(|_| "invalid face index")?;
    let index = if raw < 0 { count as i64 + raw } else { raw - 1 };
    if raw == 0 || index < 0 || index >= count as i64 {
        return Err("face index out of bounds");
    }
    Ok(index as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    const QUAD: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nf -4/1 -3/2 -2/3 -1/4 # quad";

    #[test]
    fn triangulates_relative_indices_and_computes_bounds() {
        let model = Model::from_obj(QUAD).unwrap();
        assert_eq!(model.triangles, vec![[0, 1, 2], [0, 2, 3]]);
        assert_eq!(
            model.triangle_uvs,
            vec![[Some(0), Some(1), Some(2)], [Some(0), Some(2), Some(3)]]
        );
        assert_eq!(model.bounds(), ([0.0, 0.0, 0.0], [1.0, 1.0, 0.0]));
    }

    #[test]
    fn keeps_uv_seams_and_supports_missing_or_relative_uvs() {
        let source =
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 1\nvn 0 0 1\nf 1/1 2/1 3/1\nf 1/-1 2//1 3";
        let model = Model::from_obj(source).unwrap();
        assert_eq!(model.triangles, vec![[0, 1, 2], [0, 1, 2]]);
        assert_eq!(
            model.triangle_uvs,
            vec![[Some(0); 3], [Some(1), None, None]]
        );
    }

    #[test]
    fn rejects_invalid_uv_references_and_reports_source_line() {
        for face in ["f 1/0 2 3", "f 1/2 2 3", "f 1/-2 2 3"] {
            let source = format!("v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\n{face}");
            assert_eq!(Model::from_obj(&source).unwrap_err().line, 5);
        }
        assert!(Model::from_obj("vt inf 0").is_err());
    }

    #[test]
    fn rejects_bad_models_without_panicking() {
        for source in [
            "",
            "v NaN 0 0",
            "v 0 0",
            "v 0 0 0\nf 0 1 1",
            "v 0 0 0\nf -2 1 1",
            "v 0 0 0\nf 1 2 3",
        ] {
            assert!(Model::from_obj(source).is_err(), "{source}");
        }
    }

    #[test]
    fn preserves_normal_seams_and_relative_indices_when_triangulating() {
        let source =
            "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvn 0 0 1\nvn 0 1 0\nf 1//1 2//-1 3//1 4//-1";
        let model = Model::from_obj(source).unwrap();
        assert_eq!(
            model.triangle_normals,
            vec![[Some(0), Some(1), Some(0)], [Some(0), Some(0), Some(1)]]
        );
        assert_eq!(model.triangle_uvs, vec![[None; 3]; 2]);
    }

    #[test]
    fn rejects_invalid_normal_indices_and_nonfinite_normals() {
        for face in ["f 1//0 2 3", "f 1//2 2 3", "f 1//-2 2 3", "f 1///1 2 3"] {
            let source = format!("v 0 0 0\nv 1 0 0\nv 0 1 0\nvn 0 0 1\n{face}");
            assert_eq!(Model::from_obj(&source).unwrap_err().line, 5);
        }
        assert!(Model::from_obj("vn 0 inf 0").is_err());
    }

    #[test]
    fn material_assignments_follow_triangulation_and_switches() {
        let source = "mtllib first.mtl second.mtl\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nusemtl blue crystal\nf 1 2 3 4\nusemtl pink\nf 1 2 3\nusemtl blue crystal\nf 1 2 3";
        let model = Model::from_obj(source).unwrap();
        assert_eq!(model.material_libraries, ["first.mtl", "second.mtl"]);
        assert_eq!(model.material_names, ["blue crystal", "pink"]);
        assert_eq!(
            model.triangle_materials,
            vec![None, Some(0), Some(0), Some(1), Some(0)]
        );
        for directive in ["usemtl", "mtllib"] {
            assert!(Model::from_obj(directive).is_err());
        }
    }
}
