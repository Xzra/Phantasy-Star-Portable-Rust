//! Headless exporter; Macroquad is used for shared CPU model generation only.
use std::path::PathBuf;
fn run() -> Result<(), String> {
    let mut project = psp_client::project::default_path();
    let mut output = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--project" {
            project = PathBuf::from(args.next().ok_or("--project needs a JSON path")?);
        } else if arg == "--output" {
            if output.is_some() {
                return Err("Use --output once".into());
            }
            output = Some(PathBuf::from(
                args.next().ok_or("--output needs a new folder")?,
            ));
        } else {
            return Err(
                "Usage: unreal-export --project path/project.json --output NEW_FOLDER".into(),
            );
        }
    }
    if !project.is_file() {
        return Err(format!(
            "Project file does not exist: {}",
            project.display()
        ));
    }
    let document = psp_client::project::ProjectStore::open(project)?.saved;
    let path =
        psp_client::unreal_export::export(&document, &output.ok_or("--output is required")?)?;
    println!("UE 5.7.4 migration kit exported to {}. Open PhantasyStar.uproject and read MIGRATION.md. Gameplay port remains separate.",path.display());
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Export failed: {e}");
        std::process::exit(1);
    }
}
