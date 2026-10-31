//! Read an external PSO2NGS File Downloader item index without fetching game data.
use psp_assets::catalog::AssetKind;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const MAX_INDEX_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct ReferenceItem {
    #[serde(rename = "csvFileName", default)]
    pub sheet: String,
    #[serde(rename = "itemType", default)]
    pub version: String,
    #[serde(default)]
    pub category: String,
    pub infos: BTreeMap<String, String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ArchiveReference {
    pub path: PathBuf,
    pub labels: Vec<String>,
}

impl ReferenceItem {
    pub fn name(&self) -> &str {
        ["English Name", "Japanese Name", "Path"]
            .into_iter()
            .filter_map(|key| self.infos.get(key))
            .find(|value| !value.trim().is_empty())
            .map(String::as_str)
            .unwrap_or("Unnamed item")
    }

    pub fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        [&self.sheet, &self.version, &self.category]
            .into_iter()
            .chain(self.infos.values())
            .any(|value| value.to_lowercase().contains(&query))
    }

    pub fn archives(&self) -> Vec<ArchiveReference> {
        let mut paths: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
        for (label, value) in &self.infos {
            for token in value.split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '|')) {
                if let Some(path) = archive_path(token) {
                    let labels = paths.entry(path).or_default();
                    if !labels.contains(label) {
                        labels.push(label.clone());
                    }
                }
            }
        }
        paths
            .into_iter()
            .map(|(path, labels)| ArchiveReference { path, labels })
            .collect()
    }
}

/// Only exact hex identifiers become paths; arbitrary index paths are never joined.
fn archive_path(token: &str) -> Option<PathBuf> {
    let token = token.to_ascii_lowercase();
    let parts: Vec<_> = token
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .collect();
    let is_hex = |part: &str| part.bytes().all(|byte| byte.is_ascii_hexdigit());
    match parts.as_slice() {
        [hash] if hash.len() == 32 && is_hex(hash) => Some(Path::new("win32").join(hash)),
        [prefix, hash]
            if prefix.len() == 2 && hash.len() == 30 && is_hex(prefix) && is_hex(hash) =>
        {
            Some(Path::new("win32reboot").join(prefix).join(hash))
        }
        _ => None,
    }
}

pub fn load_index(path: &Path) -> Result<Vec<ReferenceItem>, Box<dyn Error>> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > MAX_INDEX_BYTES {
        return Err("reference index exceeds 128 MiB".into());
    }
    let mut data = Vec::new();
    file.take(MAX_INDEX_BYTES + 1).read_to_end(&mut data)?;
    if data.len() as u64 > MAX_INDEX_BYTES {
        return Err("reference index exceeds 128 MiB".into());
    }
    Ok(serde_json::from_slice(&data)?)
}

fn local_status(root: &Path, relative: &Path) -> String {
    let path = root.join(relative);
    let result = (|| -> std::io::Result<_> {
        let mut file = fs::File::open(path)?;
        let size = file.metadata()?.len();
        let mut prefix = Vec::new();
        (&mut file).take(32).read_to_end(&mut prefix)?;
        Ok((size, AssetKind::from_prefix(&prefix)))
    })();
    match result {
        Ok((size, kind)) => format!("present: {size} bytes, {kind:?}"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "missing".to_owned(),
        Err(error) => format!("unreadable: {error}"),
    }
}

pub fn search(index: &str, query: &str, args: &[String]) -> Result<(), Box<dyn Error>> {
    if query.trim().is_empty() {
        return Err("search query cannot be empty".into());
    }
    let mut root = None;
    let mut limit = 20_usize;
    let mut version = None;
    let mut args = args.iter();
    while let Some(option) = args.next() {
        match option.as_str() {
            "--root" => {
                root = Some(PathBuf::from(
                    args.next().ok_or("--root needs a data-directory path")?,
                ))
            }
            "--limit" => {
                limit = args
                    .next()
                    .ok_or("--limit needs a positive number")?
                    .parse()?
            }
            "--classic" if version.is_none() => version = Some("PSO2"),
            "--ngs" if version.is_none() => version = Some("NGS"),
            _ => {
                return Err(format!("unknown or conflicting asset-search option: {option}").into())
            }
        }
    }
    if limit == 0 {
        return Err("--limit must be positive".into());
    }
    if let Some(root) = &root {
        if !root.is_dir() {
            return Err(format!("data directory does not exist: {}", root.display()).into());
        }
    }
    let index = load_index(Path::new(index))?;
    let matches: Vec<_> = index
        .iter()
        .filter(|item| {
            item.matches(query)
                && version.is_none_or(|filter| item.version.split('|').any(|v| v.trim() == filter))
        })
        .collect();
    for item in matches.iter().take(limit) {
        println!("{} [{}] ({})", item.name(), item.version, item.sheet);
        let archives = item.archives();
        if archives.is_empty() {
            println!("  No supported archive identifiers in this reference row.");
        }
        for archive in archives {
            let status = root
                .as_ref()
                .map(|root| format!(" — {}", local_status(root, &archive.path)))
                .unwrap_or_default();
            println!(
                "  {}: {}{status}",
                archive.labels.join(", "),
                archive.path.display()
            );
        }
    }
    println!(
        "{} matching items; showing {}",
        matches.len(),
        matches.len().min(limit)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_unicode_names_and_collects_deduplicated_archive_roles() {
        let item: ReferenceItem = serde_json::from_str(
            r#"{
            "csvFileName":"example.csv","itemType":"PSO2 | NGS","infos":{
                "English Name":"Training Hunter", "Japanese Name":"ハンター",
                "Icon":"0123456789abcdef0123456789abcdef",
                "Normal Quality":"0123456789abcdef0123456789abcdef",
                "High Quality":"ab\\\\0123456789abcdef0123456789abcd"}}
        "#,
        )
        .unwrap();
        assert!(item.matches("HUNTER"));
        assert!(item.matches("ハンター"));
        assert_eq!(item.name(), "Training Hunter");
        let archives = item.archives();
        assert_eq!(archives.len(), 2);
        assert!(archives
            .iter()
            .any(|a| a.labels == ["Icon", "Normal Quality"]));
        assert!(archives
            .iter()
            .any(|a| a.path == Path::new("win32reboot/ab/0123456789abcdef0123456789abcd")));
    }

    #[test]
    fn rejects_untrusted_paths_and_normalizes_identifiers() {
        for value in [
            "",
            "../some-file",
            "C:\\game\\file",
            "../../0123456789abcdef0123456789abcdef",
            "0123456789abcdef0123456789abcdeg",
            "ab/short",
        ] {
            assert_eq!(archive_path(value), None, "{value}");
        }
        assert_eq!(
            archive_path("ABCDEF0123456789ABCDEF0123456789"),
            Some(PathBuf::from("win32/abcdef0123456789abcdef0123456789"))
        );
    }

    #[test]
    fn distinguishes_missing_and_recognized_local_archives() {
        let root = std::env::temp_dir().join(format!("psp-reference-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("archive"), b"CPK ").unwrap();
        assert!(local_status(&root, Path::new("archive")).contains("Cpk"));
        assert_eq!(local_status(&root, Path::new("missing")), "missing");
        fs::remove_dir_all(root).unwrap();
    }
}
