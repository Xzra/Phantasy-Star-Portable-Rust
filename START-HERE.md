# Your game and engine

Start with the **engine Windows ZIP** from GitHub Releases. Extract the entire
ZIP into a folder you can edit. Open `START-HERE.html` for the quickstart guide, or follow these steps. Rust is only needed when changing source.

## First five minutes

1. Double-click **Start-Editor.cmd**. The starter project opens in the workbench.
2. **Character**: choose the default appearance for new saves and playtests.
3. **Animation**: preview any Hunter clip, pause, scrub, show bones and adjust
   animation energy. **Gameplay** changes movement, enemies, PP and weapons.
4. **Scene**: add solid props to the lobby or field and edit their position,
   dimensions and color. Yellow outlines show selection; cyan circles mark
   protected interactions. Invalid placements cannot be saved or played.
5. **Save**, then **Play / F6**. The game opens with your saved project.
   Close the game to return to editing. Playtests do not use your character save.

**Ctrl+Z / Ctrl+Y** undo and redo edits. **Ctrl+S** saves. Reload rereads the JSON;
if there are unsaved edits, click Reload again to discard them. A previous valid
project stays beside the file as `project.json.bak`. Files changed outside the
editor cannot be overwritten until you reload them.

## Play your character

Double-click **phantasy-star.exe**. A new save opens character creation. Existing
saves retain their progression and receive the original appearance. Game
characters and editor defaults are different: changing the default does not
replace the appearance or equipment in a player's existing save.

## Make your own project

Copy `projects/starter` to a new folder. Drag the copied `project.json` onto
**Start-Editor.cmd**. Its name and values are readable JSON; the editor shows
which file it is editing. Keep the rest of the engine package together.

## Change the code

The engine ZIP includes **source/** with the complete Rust workspace and
original assets. Install Rust and its Windows C++ prerequisites first, then
open **source/Build-Engine.cmd** to compile and start your source editor.
[The engine guide](docs/engine-guide.md) walks through installation, file locations,
examples, validation and packaging. GitHub's automatic source ZIP is also usable.

This is a focused editor for this game: character presets, shared motion preview,
combat tuning and prop placement. Adding weapon classes, new gameplay rules,
mesh topology, new authored poses, quests or terrain still means editing Rust.
It does not yet have general scene graphs, arbitrary imported-model placement,
scripting, animation curve/bone editing or a one-click standalone exporter.

## Move editing to Unreal 5.7.4

The **Unreal** tab exports a fresh asset/scene kit beside your current project.
Open that kit's `PhantasyStar.uproject` and follow `MIGRATION.md`. A ready-exported
starter kit is also published as `phantasy-star-unreal-5.7.4.zip`. Meshes, rigs,
authored player clips, scenery, props and balance references transfer; gameplay,
UI and saving still need Blueprint/C++ implementations in Unreal.
