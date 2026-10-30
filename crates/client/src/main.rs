use psp_core::{Command, QuestState, Session};
use std::{env, error::Error, fs};

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
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
        [] => println!("Usage: psp-client demo | inspect <model.obj>\nGraphical viewer: cargo run --features viewer --bin model-viewer -- [model.obj]"),
        _ => return Err("expected demo or inspect <model.obj>".into()),
    }
    Ok(())
}
