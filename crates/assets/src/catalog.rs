//! Identify containers by bytes, including PC files whose names have no extension.
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Ice { version: u32 },
    Cpk,
    Unknown,
}

impl AssetKind {
    pub fn from_prefix(bytes: &[u8]) -> Self {
        match bytes.get(..4) {
            Some(b"ICE\0") if bytes.len() >= 12 => Self::Ice {
                version: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            },
            Some(b"CPK ") => Self::Cpk,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct CatalogEntry {
    pub path: PathBuf,
    pub size: u64,
    pub kind: AssetKind,
}

#[derive(Debug)]
pub struct Catalog {
    pub entries: Vec<CatalogEntry>,
    /// True when scanning stopped at the requested file limit; results are partial.
    pub limit_reached: bool,
}

/// Walk without following symlinks. Reads at most 32 bytes per regular file.
/// The file limit bounds work on large installations, including unknown files.
pub fn catalog(root: &Path, limit: usize) -> io::Result<Catalog> {
    if limit == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "file limit must be positive",
        ));
    }
    if !fs::symlink_metadata(root)?.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "catalog root must be a directory, not a symlink",
        ));
    }
    let mut result = Catalog {
        entries: Vec::new(),
        limit_reached: false,
    };
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                directories.push(entry.path());
            } else if kind.is_file() {
                if result.entries.len() == limit {
                    result.limit_reached = true;
                    return Ok(result);
                }
                let path = entry.path();
                let mut file = fs::File::open(&path)?;
                let mut prefix = Vec::new();
                (&mut file).take(32).read_to_end(&mut prefix)?;
                result.entries.push(CatalogEntry {
                    path,
                    size: file.metadata()?.len(),
                    kind: AssetKind::from_prefix(&prefix),
                });
            }
        }
    }
    result.entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_magic_without_trusting_extensions() {
        let mut ice = [0; 32];
        ice[..4].copy_from_slice(b"ICE\0");
        ice[8..12].copy_from_slice(&4_u32.to_le_bytes());
        assert_eq!(AssetKind::from_prefix(&ice), AssetKind::Ice { version: 4 });
        assert_eq!(AssetKind::from_prefix(b"CPK "), AssetKind::Cpk);
        for length in 0..12 {
            assert_eq!(AssetKind::from_prefix(&ice[..length]), AssetKind::Unknown);
        }
        assert_eq!(
            AssetKind::from_prefix(b"not an archive"),
            AssetKind::Unknown
        );
    }

    #[test]
    fn scans_nested_files_and_reports_partial_results() {
        // A process-specific fixture directory prevents parallel test collisions.
        let root = std::env::temp_dir().join(format!("psp-catalog-{}", std::process::id()));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("opaque-name"), b"CPK ").unwrap();
        fs::write(root.join("nested/unknown"), b"other").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&root, root.join("nested/loop")).unwrap();
            std::os::unix::fs::symlink(root.join("opaque-name"), root.join("linked-file")).unwrap();
        }
        let result = catalog(&root, 10).unwrap();
        assert_eq!(result.entries.len(), 2);
        assert!(!result.limit_reached);
        assert!(result.entries.iter().any(|e| e.kind == AssetKind::Cpk));
        let partial = catalog(&root, 1).unwrap();
        assert_eq!(partial.entries.len(), 1);
        assert!(partial.limit_reached);
        assert!(catalog(&root, 0).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
