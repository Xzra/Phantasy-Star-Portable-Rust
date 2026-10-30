use psp_core::{Command, QuestState, Session};
use std::{env, error::Error, fs};

mod references;

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [command, index, query, options @ ..] if command == "asset-search" => references::search(index, query, options)?,
        [command] if command == "demo" => {
            let mut session = Session::default();
            println!("Hunter enters the lobby: {:?}", session.hunter());
            for event in session.execute(Command::StartQuest)? { println!("{event:?}"); }
            while session.quest() == QuestState::Active {
                for event in session.execute(Command::Attack)? { println!("{event:?}"); }
            }
            session.execute(Command::ReturnToLobby)?;
            println!("Back in {:?}: {:?}", session.location(), session.hunter());
        }
        [command, path] if command == "inspect" => {
            let model = psp_assets::Model::from_obj(&fs::read_to_string(path)?)?;
            println!("{} vertices, {} triangles, bounds {:?}", model.positions.len(), model.triangles.len(), model.bounds());
        }
        [command, path] if command == "catalog" => print_catalog(path, 10000)?,
        [command, path, limit] if command == "catalog" => print_catalog(path, limit.parse()?)?,
        [] => println!("Usage: psp-client demo | inspect <model.obj> | catalog <directory> [file-limit]\n  asset-search <itemData.json> <query> [--root data-directory] [--limit N] [--classic | --ngs]\nGraphical viewer: cargo run --features viewer --bin model-viewer -- [model.obj]"),
        _ => return Err("unknown command or missing arguments; run psp-client without arguments for usage".into()),
    }
    Ok(())
}

fn print_catalog(path: &str, limit: usize) -> Result<(), Box<dyn Error>> {
    use psp_assets::catalog::{catalog, AssetKind};
    let catalog = catalog(std::path::Path::new(path), limit)?;
    let mut ice = 0;
    let mut cpk = 0;
    for entry in &catalog.entries {
        match entry.kind {
            AssetKind::Ice { .. } => ice += 1,
            AssetKind::Cpk => cpk += 1,
            AssetKind::Unknown => continue,
        }
        println!("{:?}\t{}\t{:?}", entry.kind, entry.size, entry.path);
    }
    println!(
        "Scanned {} files: {ice} ICE, {cpk} CPK, {} unknown",
        catalog.entries.len(),
        catalog.entries.len() - ice - cpk
    );
    if catalog.limit_reached {
        eprintln!("Partial catalog: reached {limit} files. Increase the optional file limit to scan more.");
    }
    Ok(())
}
