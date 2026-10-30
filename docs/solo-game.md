# Solo game foundation

Run `cargo run --features viewer --bin phantasy-star`, or open the Windows
`phantasy-star.exe`. The original lobby, Hunter avatar and fallback training drone
are procedural; no PSO2 installation or downloaded assets are required.

1. Follow the blue lights to the mission terminal and press E nearby.
2. In the training field, approach and face the target. Hold Space or left mouse
   to attack. Attacks have a 0.55-second cooldown and a 2.6-unit range.
3. The stationary projection counters successful nonlethal hits. Three hits clear
   training, award 100 XP and raise the Hunter to level 2.
4. Follow the yellow lights to the return pad and press E. Returning restores HP.
5. Escape pauses the simulation. Choose New session to reset and train again.

WASD moves relative to the camera; Shift sprints. Right-mouse dragging or the
left/right arrow keys orbit the camera, and the wheel zooms. Movement collides
with walls, props, the terminal and the active target. The camera shortens its
distance around solid props. The original Hunter now has a 17-bone rig with idle, walk, run and slash keyframes.
F2 toggles bones. See [animation](animation.md) for the shared skinning path.

## Optional PSO2 projection

The Windows ZIP includes `Play-Dagan.cmd`. It uses the same verified official
download and checksum as `Load-Dagan.cmd`, then opens the game with the archive
path. No proprietary assets are bundled. See [Classic models](classic-models.md).

You can also pass a supported local model as the first argument. With no argument,
the game checks `local/dagan.ice` and `local/models/dagan/dagan.ice` relative to its
working directory, then uses the original drone if no model can be loaded.
Imported models are grounded and normalized for the training target. A companion
`dagan-motion.ice` beside the selected model enables native idle, damage,
counterattack and death clips; the Windows helpers prepare both archives.

## Validation

Core tests cover movement normalization, invalid input, walls/props, interaction
distance, attack range/facing/cooldown, rewards and the return transition.
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

The current quest is a single stationary training encounter. There are no saves,
inventory/equipment, death/retry flow, enemy navigation, multiplayer or imported player characters. Next work should establish moving enemies and telegraphed attacks,
then expand Hunter combat, reusable quests and persistence before networking.
