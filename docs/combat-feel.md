# Classic-inspired combat pass

The first training build showed models and animation, but its stationary enemy,
immediate damage and repeated slash did not create an action-game rhythm. This
pass focuses on responsive combat, a camera that holds a target, and clear
feedback. It is a prototype interpretation, with original tuning and art.

The reference is SEGA's [Classic PSO2 basic combat guide](https://pso2.jp/players/manual/pso2/battle/action/basic/):
timed follow-up attacks, lock-on, normal-hit PP recovery, photon arts, jumping,
and evasion. It is not an exact reconstruction of Classic or NGS mechanics.

## Controls

| Input | Action |
| --- | --- |
| WASD | Camera-relative movement; strafe while locked on |
| Shift | Sprint |
| Left mouse / J | Normal sword combo; holding repeats untimed attacks |
| Q | Toggle lock-on to the current enemy |
| X + movement | Dodge step; X without direction steps backward |
| Space | Jump; attacks can be used in the air |
| F | Original Photon Sweep, costing 35 PP |
| E | Terminal, return pad, or retry after defeat |
| RMB drag / arrows / wheel | Orbit / zoom camera |
| Escape | Pause, new session, quit |

## Combat rules

Three normal swings have distinct clips, durations (0.50 / 0.52 / 0.68 seconds),
and powers (12 / 16 / 22 at level 1). Damage is evaluated once at the swing's
contact time (0.18 / 0.20 / 0.28 seconds), using range and facing. Missing does not
deal damage. Input can buffer the next swing. A fresh press during the orange
ring window (64–88% of the current normal swing) makes the next swing a Just
Attack with 30% more power, rounded down. Holding does not grant this bonus.
The combo resets after a 0.4-second gap after recovery and cycles after stage 3.

PP starts at 100, recovers at 5 per second while not attacking, and normal hits
restore 8 PP. Photon Sweep costs 35 PP when it starts, deals 36 damage at level 1
on contact, and has a 3.5-unit range. The normal range is 2.6. The third normal
hit and sweep briefly stagger the enemy. Dodge cancels an attack without a PP
refund. Input during hit-stop is retained for the next simulation tick.

The step travels for 0.23 seconds with a 0.65-second cooldown. Its early portion
avoids attacks; movement still collides with props, map boundaries and the enemy.
Jumping uses simple vertical physics on a flat floor; this is not platforming or
aerial enemy navigation. Brief hit-stop, attached sword trails, damage numbers,
and slight camera shake support impact feedback.

The enemy has 144 HP and approaches at 2.8 units/second. At close range it commits
to a direction, telegraphs for 0.65 seconds, lunges, checks its attack arc once,
and recovers for 0.85 seconds. A connected strike deals 18 damage; hitting the
enemy no longer causes automatic counter damage. Its native movement, attack,
damage and death clips follow these simulation states. The ground wedge shows
the committed direction. This is one local steering enemy, without a navigation
mesh or multiple targets. It can get held up by complex prop arrangements.

Defeat stops player actions. E restarts the encounter with full health, PP and
enemy HP, without awarding XP. Victory awards 100 XP once, raises the Hunter to
level 2, and unlocks the return pad. Progress remains session-only.

## Presentation and validation

The field is an original forest clearing with grass-colored ground, rocks and
distant trees. The Hunter has beveled armor geometry on the original rig. The
compact HUD separates HP/PP, enemy health, objective, lock-on, and action palette.
This is still placeholder player/environment art, not imported PSO2 field or
player-character assets.

Core tests cover contact timing, one hit per swing, range/facing, timed combos,
PP, dodge/cooldown, collision, chase/telegraph/defeat/retry, jumping and rewards.
The graphical smoke driver follows the moving enemy, locks on, uses normal and
photon attacks, dodges windups, and returns to the lobby. It uses the same input
and simulation paths as player controls. Run it as described in
[solo gameplay](solo-game.md), with or without the optional Dagan archives.
