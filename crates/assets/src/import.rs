//! Common geometry entry point for OBJ, Classic Aqua, and supported ICE v4 archives.
use crate::{ice::IceArchive, ice_decode::decode_v4, Model};

pub struct ImportedModel {
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
                let model = Model::from_aqua(entry.data)?;
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
                })
            }
            Some(b"afp\0" | b"NIFL" | b"aqo\0") => Ok(Self {
                model: Model::from_aqua(data)?,
                diffuse_texture: None,
            }),
            _ => Ok(Self {
                model: Model::from_obj(std::str::from_utf8(data)?)?,
                diffuse_texture: None,
            }),
        }
    }
}
