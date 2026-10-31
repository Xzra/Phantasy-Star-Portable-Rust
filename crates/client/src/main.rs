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
                for event in session.execute(if session.enemy_health()==0 {Command::NextWave}else{Command::Attack})? { println!("{event:?}"); }
            }
            session.execute(Command::ReturnToLobby)?;
            println!("Back in {:?}: {:?}", session.location(), session.hunter());
        }
        [command, model, motion] if command == "motion-list" => {
            let imported=psp_assets::import::ImportedModel::parse(&read_model(model)?)?;
            let rig=imported.rig.ok_or("model has no supported skeleton/skin")?;
            println!("{} bones, {} weighted vertices",rig.skeleton.bones.len(),rig.skin.len());
            let library=rig.load_clips(&read_model(motion)?)?;
            for warning in library.warnings {eprintln!("Warning: {warning}");}
            for clip in library.clips {
                let mut max_change=0.0_f32;
                for time in [0.0,clip.duration*0.25,clip.duration*0.5,clip.duration*0.75] {
                    let palette=rig.skeleton.palette(&clip.sample(&rig.skeleton,time,false));
                    for (p,weights) in imported.model.positions.iter().zip(&rig.skin) {
                        let source=(*p).into();
                        let skinned=psp_assets::rig::skin_point(source,weights,&palette);
                        if !skinned.is_finite() {return Err("non-finite animated vertex".into());}
                        max_change=max_change.max(source.distance(skinned));
                    }
                }
                println!("{}: {:.3}s, {} tracks, max sampled displacement {:.4}",clip.name,clip.duration,clip.tracks.len(),max_change);
            }
        }
        [command, path] if command == "inspect" => {
            let file=fs::File::open(path)?;
            use std::io::Read;
            if file.metadata()?.len()>128*1024*1024 { return Err("model input exceeds 128 MiB".into()); }
            let mut bytes=Vec::new();
            file.take(128*1024*1024+1).read_to_end(&mut bytes)?;
            let imported=psp_assets::import::ImportedModel::parse(&bytes)?;
            let model=imported.model;
            println!("{} vertices, {} triangles, bounds {:?}", model.positions.len(), model.triangles.len(), model.bounds());
            if let Some(rig)=&imported.rig { println!("{} skeleton bones, {} skin vertices",rig.skeleton.bones.len(),rig.skin.len()); }
            for warning in &imported.warnings { eprintln!("Warning: {warning}"); }
            if imported.diffuse_texture.is_some() { println!("Archive primary diffuse texture available"); }
        }
        [command, path] if command == "ice-list" => {
            use std::io::Read;
            const LIMIT: u64 = 256 * 1024 * 1024;
            let file = fs::File::open(path)?;
            if file.metadata()?.len() > LIMIT { return Err("ICE archive exceeds the 256 MiB inspection limit".into()); }
            let mut data = Vec::new();
            file.take(LIMIT + 1).read_to_end(&mut data)?;
            if data.len() as u64 > LIMIT { return Err("ICE archive exceeds the 256 MiB inspection limit".into()); }
            let archive = psp_assets::ice::IceArchive::parse(&data)?;
            for entry in &archive.entries {
                println!("group {}\t{}\t{} bytes\t{}\toffset {:#x}", entry.group, entry.extension, entry.data.len(), entry.name, entry.offset);
            }
            println!("{} entries (plaintext, uncompressed ICE v4)", archive.entries.len());
        }
        [command, path] if command == "catalog" => print_catalog(path, 10000)?,
        [command, path, limit] if command == "catalog" => print_catalog(path, limit.parse()?)?,
        [] => println!("Usage: psp-client demo | inspect <model.obj> | motion-list <model.ice> <motions.ice> | ice-list <archive.ice> | catalog <directory> [file-limit]\n  asset-search <itemData.json> <query> [--root data-directory] [--limit N] [--classic | --ngs]\nGraphical viewer: cargo run --features viewer --bin model-viewer -- [model.obj]"),
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

fn read_model(path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    use std::io::Read;
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > 128 * 1024 * 1024 {
        return Err("input exceeds 128 MiB".into());
    }
    let mut bytes = Vec::new();
    file.take(128 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    Ok(bytes)
}
