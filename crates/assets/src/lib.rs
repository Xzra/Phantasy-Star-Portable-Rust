//! Small OBJ position/face importer. Proprietary game formats belong in adapters.
use std::fmt;

#[derive(Debug, PartialEq)]
pub struct Model {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[usize; 3]>,
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
    /// Imports vertices and polygon faces (fan triangulation). Materials, UVs,
    /// and normals are ignored for this first wireframe viewer.
    pub fn from_obj(source: &str) -> Result<Self, ImportError> {
        let mut model = Self {
            positions: Vec::new(),
            triangles: Vec::new(),
        };
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
                            let raw: i64 = field
                                .split('/')
                                .next()
                                .unwrap_or("")
                                .parse()
                                .map_err(|_| error("invalid face index"))?;
                            let index = if raw < 0 {
                                model.positions.len() as i64 + raw
                            } else {
                                raw - 1
                            };
                            if raw == 0 || index < 0 || index >= model.positions.len() as i64 {
                                return Err(error("face index out of bounds"));
                            }
                            Ok(index as usize)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if indices.len() < 3 {
                        return Err(error("face needs at least three vertices"));
                    }
                    for i in 1..indices.len() - 1 {
                        model
                            .triangles
                            .push([indices[0], indices[i], indices[i + 1]]);
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    const QUAD: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf -4/1 -3/2 -2/3 -1/4 # quad";

    #[test]
    fn triangulates_relative_indices_and_computes_bounds() {
        let model = Model::from_obj(QUAD).unwrap();
        assert_eq!(model.triangles, vec![[0, 1, 2], [0, 2, 3]]);
        assert_eq!(model.bounds(), ([0.0, 0.0, 0.0], [1.0, 1.0, 0.0]));
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
}
