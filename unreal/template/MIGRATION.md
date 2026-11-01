# Open this project in Unreal Engine 5.7.4

This is an **asset and scene transition kit**, with original source assets and
an editor setup script. It does not run or automatically translate Rust gameplay.
The current Rust game and its saves remain separate and usable.

1. Extract the complete kit into a writable folder. Use UE **5.7.4**.
2. Double-click `Open-Unreal.cmd` or `PhantasyStar.uproject`. Allow the enabled
   Python and Interchange editor plugins to load. No C++ module is required.
3. Save/close any unrelated unsaved work in the editor.
4. Choose **Tools → Phantasy: Import migration kit**. Wait for import to finish.
   If the menu is unavailable, use **Tools → Execute Python Script** and select
   `Content/Python/import_phantasy.py` from this kit.
5. The script imports native editable assets under `/Game/PhantasyMigration`,
   creates `Maps/Lobby` and `Maps/Field`, then opens Lobby.
6. Open a Hunter Skeletal Mesh to inspect its bones/materials, or an imported
   Animation Sequence to edit a motion. The three variants share a skeleton.
   Select props in either map to move, scale, recolor or replace them.

Opening the project only adds the menu; it never automatically rebuilds levels.
The importer refuses an existing migration destination. Re-export into a **new
folder**, or import another snapshot using the Python console:

```python
import import_phantasy
import_phantasy.run('/Game/PhantasyMigration_2')
```

Your existing imported assets and edited maps stay intact. A failed import leaves
its partial assets and writes a report to `Saved/PhantasyImportReport.json`; read
that report and the Output Log before choosing a fresh destination. No generated
`.uasset` or `.umap` files are supplied: your UE editor creates them locally.

## What crosses over

- Three original Hunter meshes with the 17-bone rig, material palettes, grip,
  default appearance and body proportion preset.
- Sword idle/walk/run, three combo clips and three PAs; Partisan and Knuckles
  idle/walk/run, three combo clips and matching PA. Attack timing and animation
  energy use the exported project's settings. World-dependent jump/step/hurt
  pose overrides remain in Rust and need UE animation states.
- Original Forest Crawler mesh and 19-bone rig, in bind pose. Its procedural AI
  motion still needs a UE animation implementation.
- Original lobby and forest decorative scenery as static meshes.
- Simple editable floor/boundaries/base-obstacle boxes and all project props.
  Interaction locations are labeled TargetPoint markers, not functioning NPCs.
- `Data/project.json`, `Data/balance.json` and `Data/manifest.json`: editable design
  references with converted movement speeds, gravity, attack ranges and timing.

GLB files are glTF 2.0: right-handed Y-up metres. Let Interchange perform its
normal coordinate/unit conversion; use import scale **1**, not 100. Native scene
primitives use `[100*x, -100*z, 100*y]` centimetres and sizes `[width, depth, height]`.
Do not add another global rotation/scale on import. A Hunter should be roughly
human height; compare the native 100 cm Engine cube if checking alignment.
Decorative scenery has no collision; the floor and separate boxes do. Actor
previews are staging aids, not playable Characters or Animation Blueprints.

No proprietary PSO2 models, archives or motions are exported. This kit doesn't
read a game installation or extract Dagan. All included geometry is original.

## Port gameplay in this order

| Rust system | Unreal implementation | Starting reference |
| --- | --- | --- |
| Movement/camera | Character Blueprint, CharacterMovement, spring arm/camera, Enhanced Input | `Data/balance.json`, `world.rs` |
| Player animations | Animation Blueprint with idle/walk/run blendspace and attack montages | Imported sequences, `hunter.rs` |
| Attack timing | Montage notifies at exported hit times, authority-side range/angle checks | `balance.json`, `weapon.rs`, `world.rs` |
| Weapon/PA equip | Blueprint weapon/item definitions, compatible PA arrays, equipment component | `weapon.rs`, `inventory.rs` |
| PP and damage | Character attributes/component and validated attack activation | `world.rs` |
| Enemy AI | AIController, behavior/state machine, collision and damage windows | `world.rs`, `animation.rs` |
| Lobby/trial travel | Interactable Blueprints replacing the markers, Open Level or streaming | `world.rs`, the two imported maps |
| Shop/inventory | UMG widgets plus item/wallet transactions | `inventory.rs`, `shop.rs`, current UI modules |
| Character creator | UMG presets + material parameters / mesh selection | `Data/project.json`, `character.rs` |
| Saving | SaveGame Blueprint/C++ schema and explicit migration if importing old profiles | `profile.rs`, `saves.rs` |

The Rust filenames above refer to `crates/` in the repository or the release's
separate source ZIP. The migration kit does not compile or run those files.

Begin with a UE Character Blueprint that uses Hunter_Sword, CharacterMovement
speeds from `balance.json`, a camera and Enhanced Input. Connect imported idle,
walk and run clips with an Animation Blueprint. Make that Character the default
pawn in your own GameMode. Next, turn the sword clips into montages and add
attack contact notifies. Keep damage checks in gameplay, not the importer.
Map-specific marker labels show where to add the mission terminal, shop, enemy
and return interaction. Rust settings and JSON are references; this first kit
has no runtime JSON loader or playable combat/inventory/save implementation.

The first complete playable Rust game remains in the main release. Rebuild or
extend the exporter from the source workspace using:

```sh
cargo run --locked --features viewer --bin unreal-export -- --project projects/starter/project.json --output local/MyUnrealSnapshot
```

The Rust editor's **Unreal** tab creates the same kit from your currently saved
project. The exporter refuses an existing output folder to preserve UE edits.

## Verification and scope

The exporter can run without a graphics window. Its GLBs are checked against the
official Khronos validator, and tests check skeleton/skin/animation data, finite
values, duration scaling, coordinate conversion, project validation and refusal
to overwrite an existing export. Python tests preflight the kit and exercise the
importer's orchestration with a test double.

Unreal Engine is not installed in the build environment, so native UE import,
visual alignment, material appearance and gameplay are **not end-to-end tested
there**. The setup code targets the documented UE 5.7 editor APIs; the local
import report checks returned mesh/animation counts and records actual asset
paths. Bring any Output Log error or import report back for a targeted fix.

## Official references

- [Interchange asset import](https://dev.epicgames.com/documentation/en-us/unreal-engine/importing-assets-using-interchange-in-unreal-engine?application_version=5.7)
- [UE 5.7 InterchangeManager API](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/InterchangeManager?application_version=5.7)
- [UE 5.7 mesh pipeline](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/InterchangeGenericMeshPipeline?application_version=5.7)
- [Editor Python setup and execution](https://dev.epicgames.com/documentation/en-us/unreal-engine/scripting-the-unreal-editor-using-python?application_version=5.7)
- [glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
