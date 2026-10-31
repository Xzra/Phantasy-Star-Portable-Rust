# Weapons and PA inventory

Tab or I opens the paused inventory. The left pane lists owned weapon instances;
the right pane lists compatible owned PA discs and the weapon holding each disc. Selecting
a weapon inspects its slot without immediately equipping it.
A white pointer with a cyan outline appears above inventory and pause panels,
using its tip as the click point. Closing the menu restores the system cursor.

- Click a weapon, or use Up/Down, to select it. The mouse wheel over the weapon
  list also selects items; longer lists page with selection.
- E or **Equip weapon** equips the selected weapon.
- Click a PA disc, or use 1/2/3, to assign it to the selected weapon.
- 0 or **Remove PA** returns that weapon's PA disc to the bag.
- Tab, I, Escape, Enter, or **Return** closes the inventory.
- RMB / F activates the equipped weapon's PA during play.

Each weapon remembers its own slot. A disc has one owner: assigning it to another
compatible weapon moves it out of the previous slot, while any displaced disc returns to the
bag. Removing or transferring discs never consumes them or restores PP.

## Starter items

The preview supplies these original items so all inventory operations can be
tried immediately. They use the Hunter's original sword mesh, with different
blade colors and attack bonuses; these are not imported PSO2 weapon models.

| Weapon | Bonus | Level 1 attack | Blade color | Initial PA |
| --- | --- | --- | --- | --- |
| Training Sword | +0 | 12 | Cyan | Photon Sweep |
| Hunter Saber | +3 | 15 | Gold | Rising Cleave |
| Photon Edge | +6 | 18 | Violet | Piercing Thrust |

All three PA discs are owned, initially slotted as shown. They remain inventory
items while slotted, so the initial bag contains 6 of 32 items. The HUD displays
the equipped weapon, current attack power and its PA. Weapon bonuses apply to
normal and PA damage before their respective multipliers.

## Additional weapon classes

Buy these weapons and their separate PA discs from Mira. Open inventory, select
the purchased weapon, press E to equip it, then click its compatible disc or
press 1 to slot it. RMB / F activates the slotted art. Purchased weapons begin
with an empty slot. Existing saves keep their starter gear unchanged.

| Class | Weapon | Buy | Normal reach | Combo duration | Compatible PA | Disc price |
| --- | --- | --- | --- | --- | --- | --- |
| Partisan | Recruit Partisan (+2 ATK) | 180 | 3.7 | 0.58 / 0.60 / 0.78 s | Assault Thrust (25 PP) | 100 |
| Knuckles | Recruit Knuckles (+2 ATK) | 160 | 1.95 | 0.32 / 0.34 / 0.46 s | Meteor Fist (25 PP) | 80 |

Partisans use a long shaft and green photon tip with narrow forward thrusts.
Knuckles equip both hands, use a raised guard and alternate jab, cross and
uppercut. Assault Thrust reaches 4.8 units; Meteor Fist hits within 2.2 units.
Both use authored poses on the original Hunter rig. Sword PAs remain exclusive
to swords; incompatible assignments are rejected before moving any disc.
Each complete weapon-and-disc pair costs less than the starting 300 Meseta.

## Simulation rules

`psp-core::inventory::Inventory` owns unique weapon instance IDs, the disc list,
capacity and equipment validation. Callers receive read-only slices; weapon
switches and disc transfers validate ownership before changing anything. Grant
operations reject full bags and duplicate PA discs without mutating the bag.
Separate copies of the same weapon kind receive separate IDs and slots.

`World` exposes inventory inspection, weapon switching and PA assignment.
Equipment changes clear queued PA input. An active swing captures its weapon class, PA
and weapon power when it begins: pausing to equip another weapon cannot change
the damage of a swing already in progress. The new loadout applies on subsequent
attacks. Inventory menus do not advance combat or regenerate PP.

Inventory and slots survive retry, return/redeployment and closing the game.
Mira's [lobby shop](lobby-shop.md) buys and sells owned items with saved Meseta.
Progress autosaves; F5 saves immediately. Reopening or **Return to lobby / keep
gear** restores the character in a fresh lobby. See [character saves](saving.md).
Random item drops and disc levels are not implemented in this version.

Tests cover instance identity, slot retention, single-owner transfers, unowned
items, full bags, weapon bonuses, active-swing snapshots, and reversible sword
tints that leave armor unchanged. Graphical trials exercise each starter weapon
with its own PA through three waves and extraction.

For isolated graphical checks, `PSP_SMOKE_CLASS=partisan` or `knuckles` adds and
equips that class with its matching PA in a diagnostic session. Combine with
`PSP_GAME_SMOKE=1` for a full three-wave trial, or `PSP_GAME_PREVIEW=grip` and
`PSP_GAME_CAPTURE=/path/to/frame.png` for a close view. Diagnostics do not use the
real save unless `PSP_SAVE_PATH` explicitly points at an isolated test profile.
