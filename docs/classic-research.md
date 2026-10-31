# Classic PSO2 download and engine investigation

The research cache uses SEGA's dedicated `patchlist_classic.txt`, not the NGS
`win32reboot` tree. The manifest resolved on 2026-10-31 contains 49,569 Classic
archives, totaling 55.26 GiB. Selected Classic engine modules, default settings,
and license records bring the research download to 49,611 files / 55.31 GiB.
This is a **data research cache**, not a complete playable installation: shared
character assets and launcher/runtime dependencies are not all included.

The live launcher management endpoint resolved these snapshot roots:

- Master: `v74000_rc_83_masterbase`
- Patch: `v74100_rc_75_F2349F13`

`http://patch01.pso2gs.net/patch_prod/patches/management_beta.txt` supplies these
full URLs. The superficially similar management file on `download.pso2.jp`
returned obsolete version references during investigation. Content transfers
use HTTPS and verify both size and the publisher's MD5 for every file. MD5 here
checks agreement with the manifest; it is not a publisher signature.

## Download and resume

Python 3.10+ and the standard library are sufficient. From the repository root:

```sh
# Fetch current manifests and show the plan before downloading content.
python3 tools/download_classic.py local/pso2-classic --refresh-manifests
python3 tools/download_classic.py local/pso2-classic --download

# Resume the same snapshot: rerun the second command, without refreshing manifests.
# Read progress in local/pso2-classic/download-status.json.
```

The downloader saves original installation-relative paths beneath
`local/pso2-classic/pso2_bin/`. It verifies existing files before skipping them,
downloads to temporary files, retries failures, and replaces a destination only
after verification. Ctrl+C preserves verified files. Run one downloader per
cache; the tool does not coordinate multiple writers. `--workers` accepts 1–16
(default 8). Reserve the full planned size plus 1 GiB of free space, even when
resuming. Failed transfers are recorded in `download-failures.json`.

All downloaded content, extracted metadata, and screenshots stay in ignored
`local/`. Game data and engine binaries are not included in source or releases.

## What has actually been read

Static inspection of the downloaded `sub/pso2classic.dll` identifies a 64-bit
Windows PE module, 34,079,432 bytes, importing Direct3D 11, DXGI, input APIs,
and `pso2bridge.dll`. Its PE timestamp reads 2026-10-09; this is header metadata,
not independent proof of a build date. Exported symbols include `CallWinMain`
and NVIDIA NGX entry points. These do not constitute an SDK or source code.
No engine binary was launched or loaded into the Rust client.

The default `data/win32/script/user_default.pso2` is readable Lua-style
configuration, including lock-on and graphics settings. Its readability does
not establish that gameplay logic can be recovered as Lua source.

The archive inventory now decodes supported Blowfish/PRS ICE v4 containers and
outputs entry names, extensions, lengths, groups and eight-byte prefixes as
JSON lines. It writes no extracted payloads. A directory scan is nonrecursive,
skips symlinks and `.download` files, limits input to 128 MiB per archive, and
reports unsupported archives individually without discarding successful results.

```sh
cargo run --bin psp-client -- ice-inventory \
  local/pso2-classic/pso2_bin/data/win32 100000 > local/classic-inventory.jsonl
```

Only already downloaded regular files are examined. The summary explicitly
reports the available count, failures and whether the requested file limit was
reached; an inventory taken during a download is a partial snapshot.

## Confirmed model and motion interoperability

Named archive discovery used the external
[KizKizz reference sheets](https://github.com/KizKizz/pso2ngs_file_downloader).
Each download was then checked against the official current manifest.

| Classic asset | Archive identifier | Verified result |
| --- | --- | --- |
| Nav Rappy model | `997ee9be77f25b43f97a1c85904e0453` | 3,282 vertices, 4,496 triangles, 23 bones; textured viewer capture |
| Nav Rappy motions | `ebb199d0fa68583b921385f5d0118872` | 45 clips, sampled against that model with finite vertex results |
| Grulf model | `3da16b555fce1bb1cc310bf15f5b2fcf` | 3,487 vertices, 3,866 triangles, 37 bones; textured viewer capture |
| Grulf motions | `0631231dd676e3f175bea7df84e8fd7b` | 66 clips, sampled against that model with finite vertex results |
| Hunter sword standard motions | `fbae0c59a79226ab2361ca5fc81a3efc` | 96 AQM entries cataloged; player rig compatibility not yet validated |
| Forest area 0310 templates | resolved from `stage/ln_area_template_0310.ice` | 145 LAT grids decoded |
| Episode 1 lobby templates | `19ee9fef1210b64073f0acd421e6df9f` | 7 LAT grids decoded |

The existing model viewer accepts these supported enemy model/motion pairs
through `--motions` and `--clip`. They have not been integrated as gameplay
enemy types. Our native animation importer does not reproduce the complete
engine's effects, layered controllers or behavior logic.

## Stage layout adapter

```sh
cargo run --bin psp-client -- stage-layout /path/to/template-archive.ice \
  > local/stage-layouts.jsonl
```

The new bounded LAT reader follows NIFL/REL0 pointers and preserves grid
dimensions, occupied tile IDs, raw rotation counts, variants, object variants,
and category bytes. Rows and columns reflect file order. Axis interpretation,
tile spacing, reserved values such as 65535, and world transforms are not
assumed. Validation uses original synthetic fixtures, truncated tables and
malformed pointers, plus the real local Forest/lobby archives.

The binary layout was cross-checked against
[Aqua Library's LAT reader](https://github.com/Shadowth117/PSO2-Aqua-Library/blob/0368cb9cffb4ec406a13955f6d3693472b002370/AquaModelLibrary.Data/PSO2/Aqua/LandAreaTemplate.cs).
No upstream source or game payload is vendored.

Forest tile archives contain terrain, placement, collision-related and geometry
records (`trn`, `set`, `trp`, `trw`, `trv`, `pgd`, `tcb`, `aqp`, `aqn`). Common
archives supply separate DDS textures. The current first-model importer opens
the first Forest tile AQP as a four-vertex helper surface; this is **not** a
rendered Forest scene. Combining the tile parts, resolving their texture and
placement references, and implementing collision are the next substantive steps.
