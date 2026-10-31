//! Diffuse colors from the OBJ companion MTL format. Texture maps are not loaded.
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone, PartialEq)]
pub struct ObjMaterial {
    pub diffuse: [f32; 3],
}

impl Default for ObjMaterial {
    fn default() -> Self {
        Self { diffuse: [0.8; 3] }
    }
}

#[derive(Debug, Default)]
pub struct MaterialLibrary {
    pub materials: HashMap<String, ObjMaterial>,
}

#[derive(Debug, PartialEq)]
pub struct MaterialError {
    pub line: usize,
    pub message: &'static str,
}

impl fmt::Display for MaterialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MTL line {}: {}", self.line, self.message)
    }
}
impl std::error::Error for MaterialError {}

impl MaterialLibrary {
    /// Reads newmtl/Kd only. Other properties, including map_Kd, are ignored.
    pub fn from_mtl(source: &str) -> Result<Self, MaterialError> {
        let mut library = Self::default();
        let mut current = None;
        for (offset, line) in source.lines().enumerate() {
            let error = |message| MaterialError {
                line: offset + 1,
                message,
            };
            let mut fields = line.split('#').next().unwrap_or("").split_whitespace();
            match fields.next() {
                Some("newmtl") => {
                    let name = fields.collect::<Vec<_>>().join(" ");
                    if name.is_empty() {
                        return Err(error("material name is missing"));
                    }
                    library
                        .materials
                        .insert(name.clone(), ObjMaterial::default());
                    current = Some(name);
                }
                Some("Kd") => {
                    let name = current
                        .as_ref()
                        .ok_or_else(|| error("Kd appears before newmtl"))?;
                    let mut diffuse = [0.0_f32; 3];
                    for channel in &mut diffuse {
                        *channel = fields
                            .next()
                            .ok_or_else(|| error("Kd needs three RGB channels"))?
                            .parse()
                            .map_err(|_| error("Kd channel is not a number"))?;
                        if !channel.is_finite() || !(0.0..=1.0).contains(channel) {
                            return Err(error(
                                "Kd channels must be finite values between zero and one",
                            ));
                        }
                    }
                    if fields.next().is_some() {
                        return Err(error("Kd needs exactly three RGB channels"));
                    }
                    library.materials.get_mut(name).unwrap().diffuse = diffuse;
                }
                _ => {}
            }
        }
        Ok(library)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_named_materials_and_preserves_defaults() {
        let library = MaterialLibrary::from_mtl(
            "newmtl blue crystal\nKd 0.2 0.5 1 # color\nnewmtl uncolored\nmap_Kd ignored.png",
        )
        .unwrap();
        assert_eq!(library.materials["blue crystal"].diffuse, [0.2, 0.5, 1.0]);
        assert_eq!(library.materials["uncolored"], ObjMaterial::default());
    }

    #[test]
    fn rejects_bad_colors_with_source_lines() {
        for color in [
            "NaN 0 0",
            "inf 0 0",
            "-0.1 0 0",
            "1.1 0 0",
            "0 0",
            "0 0 0 0",
            "spectral file.rfl",
        ] {
            let source = format!("newmtl test\nKd {color}");
            assert_eq!(MaterialLibrary::from_mtl(&source).unwrap_err().line, 2);
        }
        assert!(MaterialLibrary::from_mtl("Kd 0 0 0").is_err());
        assert!(MaterialLibrary::from_mtl("newmtl").is_err());
    }
}
