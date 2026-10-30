# Architecture

The graphical solo prototype and model viewer share an import/rendering layer.
The solo world owns its rules independently of the window and GPU.

```text
model file -> psp-assets -> geometry -> psp-client viewer
player input -> fixed-step psp-core world -> session commands -> events -> presentation
```

`psp-assets` returns positions and triangle indices without GPU types. Viewer
normalization affects presentation; source coordinates remain intact. The OBJ
importer supports positive and relative vertex/UV indices, preserving UV seams per
triangle corner. Authored normals retain separate corner indices too. It rejects missing geometry,
non-finite coordinates, and invalid position/UV indices. It is an inspection
importer, not a complete OBJ implementation.

`psp-core` owns state transitions. Invalid commands preserve the session. Training
rewards are issued once; the initial quest cannot be replayed. Damage and XP are
placeholders. `world` adds a fixed 60 Hz simulation with camera-independent X/Z
movement, normalized input, sprinting, circle/AABB collision, interaction proximity,
attack range/facing checks, and cooldowns. The client caps catch-up to six ticks
and smooths presentation separately. Core tests verify the complete quest loop
and reject invalid movement inputs. There is no persistence, enemy AI, multiplayer,
or death loop yet. Command/event types are internal APIs, not a network protocol.

`psp-client` separates headless and graphical entry points. Rendering is optional
so import and rule work can run without a display. Macroquad provides rendering
and input for the viewer and solo game. `scene` owns shared model import,
normalization, mesh batching and optional textures. `game` draws the original
lobby/avatar, translates camera-relative input into world movement, and presents
world events. It never mutates health, rewards or collision state directly.

Solid rendering computes directional intensity from authored normals, with flat
face normals as a fallback for missing or zero-length values. A supplied
texture uses OBJ UVs with the V coordinate flipped for image orientation. Faces
without UVs sample the texture at one corner. Meshes are split into 1500-triangle
batches to remain below the renderer's default capacities and 16-bit index limit.

OBJ material names are interned and assigned per triangle. The client resolves
MTL libraries relative to the OBJ file; later libraries override earlier names.
The embedded sample also embeds its original MTL. Only named materials and
three-channel diffuse RGB (`Kd`, range 0–1) are interpreted. Missing libraries,
invalid libraries, and undefined names produce visible warnings and use fallback
colors so geometry can still be inspected. Texture maps, transparency, and other
MTL properties are ignored; an explicit texture image still applies globally.

## Next model-viewer work

1. Extend the validated Classic PSO2 Aqua importer beyond the initial Dagan sample.
   Current scope and actual-model evidence are in [Classic models](classic-models.md).
2. Add more format variants with original/synthetic fixtures. Keep proprietary
   archives and extracted payloads in ignored local storage.
3. Extend material colors with texture maps and transparency as real assets require.
4. Extend the initial Classic AQN/AQM rig support with animation events and broader variants.
5. Add an asset browser and per-file import errors.

Introduce stable entity IDs, serialization/versioning,
and persistence as solo gameplay develops, before designing a hostable server.

## Animation presentation

`psp-assets::rig` owns validated skeletons, local transforms, clip sampling and
source-space skinning. `Scene` applies a separate presentation transform after
skinning and updates existing mesh vertices and lighting without changing UVs.
The original Hunter and imported Dagan use this same path. Hunter locomotion
blends from simulation movement, and attack/reaction clips follow core events.
Rendering never moves collision or issues rewards. Details and format limits are
in [animation](animation.md).
