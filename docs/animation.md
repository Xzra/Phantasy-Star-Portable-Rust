# Rigging and animation

The Hunter and optional Classic Dagan projection now use a shared CPU skeletal
animation path. Meshes retain source positions, per-vertex joint indices and up
to four normalized weights. Each local pose is composed through the hierarchy;
`animated_global * inverse_bind` supplies the skinning palette. Animation runs in
presentation and never changes collision, damage or quest rewards.

## Hunter

The original Hunter is a rigid-weighted, 17-bone model with hips, spine, chest,
head, shoulders, elbows, hands, hips/knees/feet and a right-hand photon blade.
Its mesh, colors and keyframes are generated from original code and need no
external game files. Idle, walk, run, three sword swings and three Photon Art clips share the same sampler as
imported motion. Movement speed blends locomotion states; distance advances the
gait, and upper-body attack overlays follow the simulation contact phases. Jump, step and
hurt poses reflect the current player state.
Escape pauses simulation and animation. F2 toggles joint visualization.

The sword has a dedicated right-palm grip socket. Its handle intersects the
closed glove and the blade points through the knuckles; the socket's bind
rotation, mesh and trail endpoints use the same transform. Tests verify grip
contact across every authored clip. Piercing Thrust compensates at the wrist
as the elbow extends, keeping the point aligned with the forward thrust.

## Classic Dagan

The verified model ICE includes a 33-bone AQN skeleton and four mesh palettes.
The importer prefers the first AFP model/LOD's same-name AQN, then the archive
model's same-name AQN. It reads float32 four-component weights, four byte joint
indices, and the VSET's u16 bone palette. Bind matrices supply consistent source
space, without guessing Euler rotation order. Invalid optional skeletons produce
warnings and preserve the static geometry.

Dagan's separate motion archive is `en_soldierant.ice`, hash identifier
`42c9afc07d6eb8a45a56fab381893948`. Its validated official snapshot is about 1.87 MiB;
SHA-256 is `e5e57e0684c2a9a948e4bfe70570efc3a7723870c37134b947ba7de010db4629`.
The setup helpers download/checksum it alongside the model. No archives, textures,
or proprietary motion data are shipped with the source or release.

Classic NIFL AQM position, quaternion rotation and scale channels are supported,
including constant tracks and 16-/32-bit packed frame timings. Tracks map by bone
name, unanimated bones retain their bind pose, positions/scales interpolate
linearly and rotations use shortest-path quaternion interpolation. Looping and
clamped playback are separate. Reads stay inside REL0, with bounds/count/finite
value checks and a two-million-key limit per library.

The actual Dagan archive supplies **44 supported clips**. Its additive
`30010_dmg_add` variant is skipped with an explicit warning. The game selects native movement, windup/strike/recovery, front damage and death
from the enemy AI state, with short pose blends. Clip phases fit the prototype
telegraph and attack timings. The world owns damage and movement; native root
X/Z translation is held in place, then the simulated enemy transform is applied. The full unmodified clip timing and root
motion can be examined in the viewer.

## Run and inspect

Windows ZIP: run `Play-Dagan.cmd` for the game or `Load-Dagan.cmd` for the animated
viewer. The viewer starts playing idle; A pauses/resumes, N selects the next clip,
and B toggles bones. W plus B makes internal joints easier to inspect.

```sh
cargo run --features viewer --bin phantasy-star -- local/dagan.ice local/dagan-motion.ice
cargo run --features viewer --bin model-viewer -- local/dagan.ice --motions local/dagan-motion.ice --clip 10010_mov_f
cargo run --bin psp-client -- motion-list local/dagan.ice local/dagan-motion.ice
```

The game finds `dagan-motion.ice` next to the selected model, in the executable's
`local/` cache, or in the working directory's known cache locations. This also
works when a shortcut launches the executable from a different directory.
With no motion archive, supported Dagan rigs now use original procedural joint
poses for idle, scuttling, windup, strike, recovery, recoil and collapse. The
fallback is basic motion, not extracted PSO2 keyframes. No static bind-pose fallback
is silently presented as an animated monster. The HUD indicates the active motion
source and points to `Play-Dagan.cmd` when the full set is missing. The viewer
accepts explicit archives and allows cycling through all supported clips.
`motion-list` samples four points of every clip, skins the actual model, and
rejects non-finite results. For a reproducible viewer capture, set
`PSP_VIEWER_CAPTURE=local/pose.png` and `PSP_ANIMATION_TIME=0.4`.

## Evidence and limits

Synthetic tests verify source-space bind reconstruction, parent transforms,
weighted skinning, mid-frame quaternion sampling, loop/clamp behavior, packed
timings, mesh palette resolution and malformed-data rejection. A Hunter test
checks locomotion, paused poses, repeated skinning without cumulative deformation,
and agreement between skinned positions and rendered triangle corners. Actual
Dagan clips and full graphical training runs are validated locally; Windows CI
also downloads, verifies and samples the real clips.

The binary layouts were checked against the same pinned
[PSO2-Aqua-Library reference](https://github.com/Shadowth117/PSO2-Aqua-Library/tree/0368cb9cffb4ec406a13955f6d3693472b002370/AquaModelLibrary.Data/PSO2/Aqua).
No library source is vendored. Current support targets ordinary Classic skeletal
clips with parent-before-child AQN nodes. Additive clips, inheritance flags,
retargeting, IK, animation events, packed/NGS vertices, salon scaling rules,
full materials and GPU skinning remain future work. An imported PSO2 player
character is not included; the controllable Hunter is original.

## Original creature without downloads

The standalone game now includes a 19-bone Forest Crawler with four articulated
legs, body, head, antennae and a core. Its original beveled mesh and joint poses
use the same skinning path as Dagan. It replaces the floating, bobbing placeholder
drone. Its legs scuttle, the body crouches during windup, extends on strike,
recoils from hits, and curls/rolls on death. Death stays visible for 1.25 seconds
even when no native motion archive exists. Pausing freezes the animation.

Procedural poses are sampled from bind data every frame, without accumulating
vertex deformation. Locomotion follows actual travel, so blocked movement does
not keep playing a walking cycle. Native animation remains preferred per clip.
Tests verify action-state deformation of weighted vertices, finite/repeatable
poses, fallback death duration and discovery from a separate launch directory.
