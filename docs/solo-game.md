# Solo game foundation

Run `cargo run --features viewer --bin phantasy-star`, or open the Windows
`phantasy-star.exe`. The original lobby, Hunter avatar and fallback training drone
are procedural; no PSO2 installation or downloaded assets are required.

1. Follow the blue lights to the mission terminal and press E nearby.
2. In the forest trial, press Q to lock on to the enemy. Left mouse or J attacks
   with a three-part sword combo. A fresh press during the orange ring window
   strengthens the next swing. Hold to repeat untimed attacks.
3. X steps in your movement direction (backward if stationary); Space jumps.
   Avoid the enemy's committed ground wedge. It now chases, telegraphs and lunges.
4. F uses Photon Sweep for 35 PP. Normal hits restore 8 PP; PP also recovers
   between attacks. Clear the encounter for 100 XP and level 2.
5. Approach the return pad and press E. Returning restores HP. If defeated, E
   restarts the encounter with full health and PP, without rewarding XP.
6. Escape pauses. Choose New session to reset and play again.

WASD moves relative to the camera; Shift sprints. Right-mouse dragging or arrows
orbit, and the wheel zooms. Movement collides with walls, props, the terminal and
the active enemy. Lock-on keeps facing and camera on the target while strafing.
The original Hunter has a 17-bone rig, distinct sword clips and attached blade.
F2 toggles bones. See [combat feel](combat-feel.md) for rules and known limits,
and [animation](animation.md) for the shared skinning path.

## Optional PSO2 projection

The Windows ZIP includes `Play-Dagan.cmd`. It uses the same verified official
download and checksum as `Load-Dagan.cmd`, then opens the game with the archive
path. No proprietary assets are bundled. See [Classic models](classic-models.md).

You can also pass a supported local model as the first argument. With no argument,
the game checks `local/dagan.ice` and `local/models/dagan/dagan.ice` relative to its
working directory, then uses the original drone if no model can be loaded.
Imported models are grounded, normalized and placed at the simulated enemy position. A companion
`dagan-motion.ice` beside the selected model enables native movement, windup/attack, damage and death clips; the Windows helpers prepare both archives.

## Validation

Core tests cover movement/collision, delayed contact, combos, timing bonuses, PP,
step evasion, lock-on, chase/telegraph, defeat/retry, rewards and quest transitions.
On a machine with a graphics display:

```sh
mkdir -p local
PSP_GAME_SMOKE=1 PSP_GAME_CAPTURE=local/solo.png cargo run --features viewer --bin phantasy-star
```

This drives ordinary movement and interactions through the full quest, exits
successfully only after returning at level 2, and captures lobby, field and final
framebuffers. It fails after 30 seconds of simulated time. Omit `PSP_GAME_SMOKE`
to capture just the initial scene after three frames. The output folder must exist.

## Next steps

The current quest is one combat encounter with simple local enemy steering. Next
work is richer player/field art, equipment, multiple enemies and quest reuse,
then persistence before networking. There is no navigation mesh or imported PSO2
player character yet.
