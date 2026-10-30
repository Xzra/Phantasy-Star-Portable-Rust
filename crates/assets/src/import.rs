//! Common geometry entry point for OBJ, Classic Aqua, and supported ICE v4 archives.
use crate::{ice::IceArchive, ice_decode::decode_v4, Model};

pub struct ImportedModel {
    pub rig: Option<crate::rig::Rig>,
    pub warnings: Vec<String>,
    pub model: Model,
    /// Encoded primary diffuse image, when one is present in the source archive.
    pub diffuse_texture: Option<Vec<u8>>,
}

impl ImportedModel {
    pub fn parse(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if data.len() > 128 * 1024 * 1024 {
            return Err("model input exceeds 128 MiB".into());
        }
        match data.get(..4) {
            Some(b"ICE\0") => {
                let decoded = decode_v4(data)?;
                let archive = IceArchive::parse(&decoded)?;
                let entry = archive
                    .entries
                    .iter()
                    .find(|entry| {
                        let name = entry.name.to_ascii_lowercase();
                        name.ends_with(".aqp") || name.ends_with(".aqo")
                    })
                    .ok_or("ICE archive has no supported Aqua geometry entry")?;
                let (model, skin) = crate::aqua::parse(entry.data)?;
                let mut warnings = Vec::new();
                let rig = if let Some(skin) = skin {
                    // AFP's first mesh LOD can have its own companion skeleton.
                    let model_name = if entry.data.starts_with(b"afp\0") {
                        std::str::from_utf8(&entry.data[16..48])
                            .unwrap_or("")
                            .split('\0')
                            .next()
                            .unwrap_or("")
                    } else {
                        entry.name
                    };
                    let expected = model_name.rsplit_once('.').map(|(s, _)| format!("{s}.aqn"));
                    let fallback = entry.name.rsplit_once('.').map(|(s, _)| format!("{s}.aqn"));
                    let node = expected.iter().chain(fallback.iter()).find_map(|name| {
                        archive
                            .entries
                            .iter()
                            .find(|e| e.name.eq_ignore_ascii_case(name))
                    });
                    match node {
                        Some(node) => match crate::rig::Skeleton::from_aqn(node.data)
                            .and_then(|s| crate::rig::Rig::new(s, skin, model.positions.len()))
                        {
                            Ok(rig) => Some(rig),
                            Err(error) => {
                                warnings.push(format!("Skeleton unavailable: {error}"));
                                None
                            }
                        },
                        None => {
                            warnings.push(
                                "Model has skin weights but no matching AQN skeleton".to_owned(),
                            );
                            None
                        }
                    }
                } else {
                    None
                };
                let stem = entry
                    .name
                    .rsplit_once('.')
                    .map(|(name, _)| name)
                    .unwrap_or(entry.name);
                let expected = format!("{stem}_d.dds");
                let diffuse_texture = archive
                    .entries
                    .iter()
                    .find(|entry| entry.name.eq_ignore_ascii_case(&expected))
                    .map(|entry| entry.data.to_vec());
                Ok(Self {
                    model,
                    diffuse_texture,
                    rig,
                    warnings,
                })
            }
            Some(b"afp\0" | b"NIFL" | b"aqo\0") => Ok(Self {
                model: Model::from_aqua(data)?,
                diffuse_texture: None,
                rig: None,
                warnings: Vec::new(),
            }),
            _ => Ok(Self {
                model: Model::from_obj(std::str::from_utf8(data)?)?,
                diffuse_texture: None,
                rig: None,
                warnings: Vec::new(),
            }),
        }
    }
}
