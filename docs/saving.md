# Character saves

The game automatically saves level, experience, Meseta, owned weapon instances,
PA discs, the equipped weapon and all PA assignments. F5 or **Save progress** in
the pause menu saves immediately. Normal changes autosave after about a second;
the Quit button and ordinary window close also flush progress.

Reopening restores the character in the lobby with full HP and PP. Enemy states,
current trial waves, position, HP, attack buffers and trial statistics are not
stored. An interrupted trial must be started again; loading never replays a
trial reward. **Return to lobby / keep gear** replaces the old destructive
New session button. The prototype's progression cap is level 100.

## Location

- Windows: `%LOCALAPPDATA%\PhantasyStarPortableRust\profile.json`
- Linux: `$XDG_STATE_HOME/phantasy-star-portable-rust/profile.json`, falling
  back to `~/.local/state/phantasy-star-portable-rust/profile.json`.
- `PSP_SAVE_PATH` overrides the complete path, useful for isolated tests or
  a portable profile. Use one game window per profile.

A previous valid version is kept alongside the save as `profile.json.bak`.
Writes use a separate temporary file, sync its contents, and rename it into
place. The previous primary is backed up before replacement. Validation rejects
unknown schema versions, unknown items, invalid level/XP, excessive money,
duplicate IDs/discs, impossible assignments, missing equipped weapons and bags
over 32 items. Input is bounded to 64 KiB.

Invalid or unreadable existing saves are preserved and saving is disabled for
that run. The game opens paused with a save-unavailable status. Changes detected
in the save file before a subsequent write also stop saving. Write failures are
reported in the pause menu; F5 can retry ordinary I/O failures.

To recover a previous backup, close the game, preserve the current primary under
another filename, then copy the `.bak` file to the primary path before reopening.
There is no automatic overwrite of a rejected or newer-version save.

## Implementation and validation

The core exposes a validated `Profile` and `InventoryState` without a serialization
dependency. The client maps these to a versioned JSON record with stable item
names. Core tests cover profile reconstruction, gear relationships, progression
bounds and the level cap. File tests cover round trips, autosave delay, backup
retention, invalid/external data preservation and failed-write behavior.

Preview/capture/smoke environments disable loading and saving by default, so they
cannot change a real character. Supplying an explicit `PSP_SAVE_PATH` enables
isolated persistence checks. A graphical verification completed the shop/trial
loop, saved a level-2 character with 50 Meseta and purchased weapon #5, then
restarted the process and confirmed the same profile without duplicate rewards.
