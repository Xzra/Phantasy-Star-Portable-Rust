//! Bounded, read-only archive inventory. JSON lines contain metadata, never payloads.
use serde::Serialize;
use std::{
    error::Error,
    fs,
    io::{self, Read, Write},
    path::Path,
};

const INPUT_LIMIT: u64 = 128 * 1024 * 1024;

#[derive(Serialize)]
struct EntryInfo<'a> {
    group: u8,
    name: &'a str,
    extension: &'a str,
    bytes: usize,
    prefix_hex: String,
}

pub fn run(directory: &str, limit: usize) -> Result<(), Box<dyn Error>> {
    if !(1..=100_000).contains(&limit) {
        return Err("inventory limit must be between 1 and 100000 files".into());
    }
    let root = Path::new(directory);
    let mut files = Vec::new();
    for item in fs::read_dir(root)? {
        let item = item?;
        if item.file_type()?.is_file() && item.path().extension().is_none_or(|e| e != "download") {
            if files.len() == 100_000 {
                return Err("directory exceeds the 100000-file inventory limit".into());
            }
            files.push(item.path());
        }
    }
    files.sort();
    let available = files.len();
    let mut decoded = 0;
    let mut failed = 0;
    let mut entry_count = 0;
    let stdout = io::stdout();
    let mut output = stdout.lock();
    for path in files.iter().take(limit) {
        match inspect(path) {
            Ok(record) => {
                entry_count += record["entries"].as_array().map_or(0, Vec::len);
                decoded += 1;
                serde_json::to_writer(&mut output, &record)?;
            }
            Err(error) => {
                failed += 1;
                serde_json::to_writer(
                    &mut output,
                    &serde_json::json!({
                        "type": "error", "archive": path.file_name().unwrap().to_string_lossy(),
                        "error": error.to_string()
                    }),
                )?;
            }
        }
        writeln!(output)?;
    }
    serde_json::to_writer(
        &mut output,
        &serde_json::json!({
            "type": "summary", "scanned": decoded + failed, "decoded": decoded,
            "failed": failed, "entries": entry_count, "available_files": available,
            "limit_reached": available > limit
        }),
    )?;
    writeln!(output)?;
    Ok(())
}

fn inspect(path: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > INPUT_LIMIT {
        return Err("archive exceeds the 128 MiB input limit".into());
    }
    let mut bytes = Vec::new();
    file.take(INPUT_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > INPUT_LIMIT {
        return Err("archive exceeds the 128 MiB input limit".into());
    }
    inspect_bytes(
        &path.file_name().unwrap_or_default().to_string_lossy(),
        &bytes,
    )
}

fn inspect_bytes(name: &str, bytes: &[u8]) -> Result<serde_json::Value, Box<dyn Error>> {
    let decoded = psp_assets::ice_decode::decode_v4(bytes)?;
    let archive = psp_assets::ice::IceArchive::parse(&decoded)?;
    let entries: Vec<_> = archive
        .entries
        .iter()
        .map(|e| EntryInfo {
            group: e.group,
            name: e.name,
            extension: e.extension,
            bytes: e.data.len(),
            prefix_hex: e.data.iter().take(8).map(|b| format!("{b:02x}")).collect(),
        })
        .collect();
    Ok(serde_json::json!({"type": "archive", "archive": name,
                          "stored_bytes": bytes.len(), "entries": entries}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventories_decoded_entry_metadata_without_exposing_payload() {
        let mut bytes = vec![0; 0x150 + 96];
        bytes[..4].copy_from_slice(b"ICE\0");
        for (offset, value) in [
            (8, 4_u32),
            (12, 0x80),
            (28, 432),
            (0x130, 96),
            (0x138, 1),
            (0x154, 96),
            (0x158, 16),
            (0x15c, 80),
            (0x160, 9),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[0x150..0x154].copy_from_slice(b"set\0");
        bytes[0x190..0x198].copy_from_slice(b"test.set");
        bytes[0x1a0..].copy_from_slice(b"private payload!");
        let record = inspect_bytes("original", &bytes).unwrap();
        assert_eq!(record["entries"][0]["name"], "test.set");
        assert_eq!(record["entries"][0]["bytes"], 16);
        assert_eq!(record["entries"][0]["prefix_hex"], "7072697661746520");
        assert!(!record.to_string().contains("private payload!"));
    }

    #[test]
    fn rejects_unsupported_inputs_and_does_not_require_graphics() {
        assert!(inspect_bytes("bad", b"not an ICE file").is_err());
        assert!(run("unused", 0).is_err());
        assert!(run("unused", 100_001).is_err());
    }
}
