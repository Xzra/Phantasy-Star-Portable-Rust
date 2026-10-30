# Phantasy Star Portable Rust

A ground-up Rust game inspired by Phantasy Star Online, using PSO2 and Nova as
references. Start with solo play, then introduce a hostable server once the game
is in a good state.

## Start here: model viewer

Install stable Rust with Cargo, then run from the repository root:

```sh
cargo run --features viewer --bin model-viewer
cargo run --features viewer --bin model-viewer -- /path/to/model.obj
cargo run --features viewer --bin model-viewer -- /path/to/model.obj --texture /path/to/diffuse.png
```

The first milestone is a desktop model viewer. It includes an original
crystal model, automatic framing, arrow-key orbit, mouse-wheel zoom, Space to
toggle rotation, R to reset, and Escape to quit. W toggles wireframe, T toggles
an explicitly supplied texture, and G toggles the grid. Solid mode uses computed
flat face lighting. A working graphics/display
environment is required. OBJ polygon faces are fan-triangulated; use convex faces
or pre-triangulated models. OBJ UV seams and relative UV indices are supported;
the texture option applies one image to the whole model. Material files, authored
normals, animation, and proprietary game models are not supported yet.

Headless tools work without graphics dependencies enabled:

```sh
cargo run --bin psp-client -- inspect assets/models/training-crystal.obj
cargo run --bin psp-client -- demo
cargo run --bin psp-client -- catalog /path/to/pso2_bin/data 100000
```

The demo exercises a small lobby → training quest → combat → level-up → lobby
loop. It is groundwork for later milestones, not a playable graphical lobby.
The read-only catalog recognizes ICE/CPK signatures without relying on filenames,
reads only 32 bytes per file, skips symlinks, and reports partial scans when the
file limit is reached (10,000 by default). Unknown files remain unidentified;
this command does not extract or decode game data.

## Framework

| Component | Responsibility |
| --- | --- |
| `crates/assets` | Validated OBJ geometry/UV import, triangulation, bounds |
| `crates/client` | Desktop viewer and command-line entry point |
| `crates/game-core` | Rendering-independent solo session, commands, events |
| `tools/inspect_cpk.py` | Read-only local/FTP CPK metadata inspection (Python 3) |
| `assets/models` | Original distributable sample assets |

Keep game rules independent of rendering and file formats. A future authoritative
server can own the session without depending on a graphics library.

## Roadmap (original order)

1. **Model viewer — in progress:** OBJ solid/wireframe modes, one-texture rendering,
   and camera controls exist. Next: game-asset adapters, materials, skeletons/animation.
2. **Working lobby:** player movement, camera, collision, quest interaction.
3. **Hunter class:** character state, weapon equip, movement/attack animations.
4. **Basic combat and leveling:** real-time hit detection, damage, enemies, XP.
5. **Basic quest:** load a field, complete an objective, receive rewards, return.
6. **Hostable server:** move established rules behind authoritative commands.

See [architecture](docs/architecture.md) and [Vita research](docs/vita-assets.md).

## Development

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
python3 -m unittest discover -s tools -p 'test_*.py'
```

Cargo.lock is committed for reproducible application builds. CI checks the optional
viewer too. The viewer uses Macroquad; the core and OBJ importer have no third-party
dependencies. Game installations, downloaded archives, extracted assets, and
personal reports belong outside the repository or under ignored `local/`.
No PSO2/Nova game files are included. This is a desktop Rust project, not an
existing Vita port or compatibility implementation.

