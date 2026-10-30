# Architecture

The viewer is the active milestone from the README; the small solo simulation
establishes a boundary that later lobby/combat work can use.

```text
model file -> psp-assets -> geometry -> psp-client viewer
player input -> command -> psp-core session -> events -> presentation
```

`psp-assets` returns positions and triangle indices without GPU types. Viewer
normalization affects presentation; source coordinates remain intact. The OBJ
importer supports positive and relative vertex/UV indices, preserving UV seams per
triangle corner. Authored normals retain separate corner indices too. It rejects missing geometry,
non-finite coordinates, and invalid position/UV indices. It is an inspection
importer, not a complete OBJ implementation.

`psp-core` owns state transitions. Invalid commands preserve the session. Training
rewards are issued once; the initial quest cannot be replayed. Damage and XP are
placeholders. There is no persistence, real-time simulation, multiplayer, or death
loop yet. Command/event types are internal APIs, not a network protocol.

`psp-client` separates headless and graphical entry points. Rendering is optional
so import and rule work can run without a display. Macroquad provides rendering
and input for the initial viewer. Reassess engine needs when textured animation
and lobby collision are concrete.

Solid rendering computes directional intensity from authored normals, with flat
face normals as a fallback for missing or zero-length values. A supplied
texture uses OBJ UVs with the V coordinate flipped for image orientation. Faces
without UVs sample the texture at one corner. Meshes are split into 1500-triangle
batches to remain below the renderer's default capacities and 16-bit index limit.

## Next model-viewer work

1. Identify readable source geometry and payload versions from a game installation
   or user-provided export; filenames alone do not establish compatibility.
2. Add importer adapters with original/synthetic fixtures. Keep proprietary
   archives and extracted payloads in ignored local storage.
3. Extend single-texture rendering with material assignments.
4. Add skeleton/animation import after validating an actual model format.
5. Add an asset browser and per-file import errors.

Introduce stable entity IDs, fixed simulation ticks, serialization/versioning,
and persistence as solo gameplay develops, before designing a hostable server.
