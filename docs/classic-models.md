# First real PSO2 model

The viewer has been validated with Classic PSO2's Dagan model:

- Archive identifier: `15d422cd829ec44a9584cf019d9f9014` (`en_soldierant_a.ice`).
- Download size: 706,216 bytes; encrypted ICE v4 with PRS-compressed groups.
- Geometry: `en_ant_a.aqp`, first AFP model/LOD (`en_ant_a_l1.aqo`).
- Rendered geometry: 2,950 vertices, 3,356 nondegenerate triangles.
- Primary diffuse texture: `en_ant_a_d.dds`, 512 × 512, DXT5.
- SHA-256: `4f99143916c24cdbda9fc9be4fc4c8c2e268119f66269de37d13c177b9e76f96`.

The official patch-server snapshot URL used for validation is embedded in
`tools/Load-Dagan.cmd`. The script downloads it only when run, verifies the hash,
and opens the local archive. No game files are committed or included in releases.
The snapshot URL can eventually be retired by the publisher.

## Run

Windows preview ZIP: extract it to a writable folder and run `Load-Dagan.cmd`.

From source, with your own supported files:

```sh
cargo run --bin psp-client -- inspect /path/to/model.ice
cargo run --features viewer --bin model-viewer -- /path/to/model.ice
cargo run --features viewer --bin model-viewer -- /path/to/model.aqp --texture /path/to/diffuse.dds
```

The importer recognizes files by their magic bytes, so opaque game filenames work
when opening ICE directly. It selects the first AQP/AQO entry, imports its first
AFP model, and chooses a same-stem `_d.dds` texture if available. An explicit
`--texture` overrides the archive's texture. PNG and legacy DDS DXT1/3/5 are supported.

## Implemented scope

Classic Aqua object types `0xC2A` and `0xC31` with float32 positions, normals,
and primary UVs; indexed triangle lists and strips; separate mesh vertex sets.
Degenerate strip faces are removed without resetting winding parity. Aqua UVs are
normalized to the renderer's OBJ-style convention. All addresses and table spans
are checked against the REL0 chunk, and import counts are limited.

The decoder supports standard ICE v4 with optional Blowfish encryption and Modern
PRS compression, bounded to 128 MiB input and 64 MiB per expanded group. Decoded
entry names, sizes, and ranges are validated before model import. CRCs are not
verified. The original `ice-list` command remains a plaintext-only inspector;
`inspect` and the graphical viewer use the decoding pipeline.

The model-format layout was checked against
[PSO2-Aqua-Library](https://github.com/Shadowth117/PSO2-Aqua-Library/tree/0368cb9cffb4ec406a13955f6d3693472b002370/AquaModelLibrary.Data/PSO2/Aqua).
No source from that library is vendored. ICE key derivation is adapted from the
MIT-licensed ages-ice-archive reader; its notice is in `THIRD_PARTY_NOTICES.md`.

This is a static geometry preview. Skeleton transforms, animation, complete
material/texture assignment, alpha blending, game shaders, NGS/Nova geometry,
half/packed vertex formats, Oodle, and other ICE versions remain unsupported.
Tests contain only original synthetic geometry and archive data. Real-model
validation files and captured frames stay under ignored `local/`.
