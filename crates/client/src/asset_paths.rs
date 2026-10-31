//! Discover optional caches relative to the executable as well as the launch directory.
use std::path::{Path, PathBuf};
pub fn model(executable_dir: &Path, working_dir: &Path) -> Option<PathBuf> {
    [
        executable_dir.join("local/dagan.ice"),
        working_dir.join("local/dagan.ice"),
        working_dir.join("local/models/dagan/dagan.ice"),
    ]
    .into_iter()
    .find(|p| p.is_file())
}
pub fn motions(model: &Path, executable_dir: &Path, working_dir: &Path) -> Option<PathBuf> {
    [
        model.with_file_name("dagan-motion.ice"),
        executable_dir.join("local/dagan-motion.ice"),
        working_dir.join("local/dagan-motion.ice"),
        working_dir.join("local/models/dagan/dagan-motion.ice"),
    ]
    .into_iter()
    .find(|p| p.is_file())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_executable_cache_when_launch_directory_is_elsewhere() {
        let root = std::env::temp_dir().join(format!("psp-motion-paths-{}", std::process::id()));
        std::fs::create_dir_all(root.join("exe/local")).unwrap();
        std::fs::create_dir_all(root.join("cwd")).unwrap();
        std::fs::write(root.join("exe/local/dagan.ice"), b"fixture").unwrap();
        std::fs::write(root.join("exe/local/dagan-motion.ice"), b"fixture").unwrap();
        let model = model(&root.join("exe"), &root.join("cwd")).unwrap();
        assert_eq!(model, root.join("exe/local/dagan.ice"));
        assert_eq!(
            motions(&model, &root.join("exe"), &root.join("cwd")).unwrap(),
            root.join("exe/local/dagan-motion.ice")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
