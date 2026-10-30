# Phantasy Star Portable Rust

A ground-up Rust game inspired by Phantasy Star Online, using PSO2 and Nova as
references. Start with solo play, then introduce a hostable server once the game
is in a good state.

## Start here: playable solo prototype

Download `phantasy-star.exe` from [GitHub Releases](https://github.com/Xzra/Phantasy-Star-Portable-Rust/releases)
and open it to enter the original orbital lobby. No game installation is required.
Walk to the blue mission terminal and press E to enter Hunter training. Defeat the
projection, gain a level, then use the return pad. The ZIP also includes
`Play-Dagan.cmd` to fetch the verified Classic Dagan model for the training target.

```sh
cargo run --features viewer --bin phantasy-star
cargo run --features viewer --bin phantasy-star -- /path/to/dagan.ice
```

WASD moves relative to the camera, Shift sprints, E interacts, and Space or left
mouse attacks. Hold right mouse and drag, or use the arrow keys, to orbit; scroll
to zoom. Escape opens the pause menu, including New session and Quit.

This is a small playable foundation: collision, a chase camera, a stationary
training target with simple counter damage, XP, and one quest. Progress lasts
only for the current session. Enemy AI, skeletal animation, equipment, saving,
and multiplayer remain future work. See [solo gameplay](docs/solo-game.md).

## Model viewer

Windows preview executables are distributed through
[GitHub Releases](https://github.com/Xzra/Phantasy-Star-Portable-Rust/releases).
Run `model-viewer.exe` for the embedded sample, or extract the ZIP for the viewer,
command-line tools, sample assets, and instructions. These previews open OBJ
models and limited Classic PSO2 Aqua/ICE geometry. The ZIP includes `Load-Dagan.cmd`
to fetch a verified Dagan sample from the official patch server and open it locally.
See [Classic model support](docs/classic-models.md) for scope and limitations.

Install stable Rust with Cargo, then run from the repository root:

```sh
cargo run --features viewer --bin model-viewer
cargo run --features viewer --bin model-viewer -- /path/to/model.obj
cargo run --features viewer --bin model-viewer -- /path/to/model.obj --texture /path/to/diffuse.png
cargo run --features viewer --bin model-viewer -- /path/to/classic-model.ice
```

The desktop model viewer includes an original
crystal model, automatic framing, arrow-key orbit, mouse-wheel zoom, Space to
toggle rotation, R to reset, and Escape to quit. W toggles wireframe, T toggles
an explicitly supplied texture, and G toggles the grid. Solid mode uses computed
directional lighting from authored normals, falling back to flat face normals.
A working graphics/display
environment is required. OBJ polygon faces are fan-triangulated; use convex faces
or pre-triangulated models. OBJ UV seams and relative UV indices are supported;
the texture option applies one image to the whole model. Hard edges and separate
normal indices are preserved. Companion MTL files supply diffuse RGB (`Kd`)
colors per material. Missing or malformed MTL files produce warnings and preserve
the geometry. Automatic MTL texture maps, transparency, and animation are not
supported yet. Classic PSO2 Aqua geometry in supported ICE v4 containers can be
opened directly, including its matching primary diffuse DDS texture.
Drop an OBJ, AQP/AQO, or supported ICE onto the window to open it, or press L to reload the current
file after editing it. Import errors appear in the window and preserve the
previous model. The explicitly supplied texture stays selected across model loads.

Headless tools work without graphics dependencies enabled:

```sh
cargo run --bin psp-client -- inspect assets/models/training-crystal.obj
cargo run --bin psp-client -- demo
cargo run --bin psp-client -- catalog /path/to/pso2_bin/data 100000
cargo run --bin psp-client -- asset-search /path/to/itemData.json "Frontier Wing" --classic --limit 5
cargo run --bin psp-client -- ice-list /path/to/plaintext-v4.ice
```

The demo exercises a small lobby → training quest → combat → level-up → lobby
loop without a display. The separate `phantasy-star` binary provides graphical play.
The read-only catalog recognizes ICE/CPK signatures without relying on filenames,
reads only 32 bytes per file, skips symlinks, and reports partial scans when the
file limit is reached (10,000 by default). Unknown files remain unidentified;
this command does not extract or decode game data.
Named discovery reads an external PSO2NGS File Downloader item index and can check
matching archives against a local installation. See [asset references](docs/asset-references.md).
`ice-list` lists entries in plaintext, uncompressed ICE v4 containers. Encrypted,
compressed, flagged, and other-version archives are rejected explicitly. This
does not decode the entries' model or texture formats.

## Framework

| Component | Responsibility |
| --- | --- |
| `crates/assets` | Validated OBJ geometry/UV import, triangulation, bounds |
| `crates/client` | Solo game, shared model rendering, viewer and command-line tools |
| `crates/game-core` | Fixed-step world, movement/collision, solo session, commands/events |
| `tools/inspect_cpk.py` | Read-only local/FTP CPK metadata inspection (Python 3) |
| `assets/models` | Original distributable sample assets |

Keep game rules independent of rendering and file formats. A future authoritative
server can own the session without depending on a graphics library.

## Roadmap (original order)

1. **Model viewer — in progress:** OBJ solid/wireframe modes, one-texture rendering,
   and camera controls exist. Next: game-asset adapters, materials, skeletons/animation.
2. **Working lobby — initial playable version:** movement, chase camera, collision, mission terminal.
3. **Hunter class:** character state, weapon equip, movement/attack animations.
4. **Basic combat and leveling — foundation:** range/facing checks, attack cooldowns, damage and XP; next: mobile enemies and richer combat.
5. **Basic quest — initial training loop:** enter a field, defeat a projection, receive rewards, return.
6. **Hostable server:** move established rules behind authoritative commands.

See [architecture](docs/architecture.md) and [Vita research](docs/vita-assets.md).

## Development

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
python3 -m unittest discover -s tools -p 'test_*.py'
```

For a repeatable graphics smoke check on a machine with a working display:

```sh
mkdir -p local
PSP_VIEWER_CAPTURE=local/viewer.png cargo run --features viewer --bin model-viewer
```

This renders three frames, saves the viewer framebuffer as a PNG, then exits.
The output directory must already exist; capture failures return a nonzero exit.

Cargo.lock is committed for reproducible application builds. CI checks the optional
viewer too. The viewer uses Macroquad; the core has no third-party dependencies.
The archive decoder uses Blowfish and PRS libraries. Game installations, downloaded archives, extracted assets, and
personal reports belong outside the repository or under ignored `local/`.
No PSO2/Nova game files are included. This is a desktop Rust project, not an
existing Vita port or compatibility implementation.

