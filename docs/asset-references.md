# Named asset discovery

[KizKizz's PSO2NGS File Downloader](https://github.com/KizKizz/pso2ngs_file_downloader)
provides searchable item metadata and reference sheets. Its index includes
Classic PSO2, NGS, and shared entries; its extraction integration invokes Zamboni.
It helps locate containers, but does not establish Vita/PC model compatibility.

The `asset-search` command independently reads an external copy of that project's
`json/itemData.json`. No upstream code, index, icons, or game files are vendored.
The referenced project is GPL-3.0; consult its repository for its distribution terms.

## Search a local index

Get `json/itemData.json` from the linked repository, or use an existing checkout:

```sh
cargo run --bin psp-client -- asset-search /path/to/itemData.json "Frontier Wing" --classic --limit 5
cargo run --bin psp-client -- asset-search /path/to/itemData.json "Rappy" --ngs --limit 5
cargo run --bin psp-client -- asset-search /path/to/itemData.json "Frontier Wing" \
  --classic --root /path/to/pso2_bin/data
```

Search matches names, paths, categories, reference-sheet names, and archive
identifiers, case-insensitively. Japanese text is supported. `--classic` and
`--ngs` include shared `PSO2 | NGS` entries; omit both to search all rows.
`--limit` defaults to 20. An empty query or zero limit is an error.

Supported archive references are exact hexadecimal identifiers:

| Reference shape | Relative installation path |
| --- | --- |
| 32 hexadecimal characters | `win32/<identifier>` |
| 2 characters + separator + 30 characters | `win32reboot/<prefix>/<remainder>` |

Windows and Unix separators are accepted. Arbitrary paths from the index are
never used for local-file lookup. Multiple roles pointing at the same archive
are grouped. These identifiers name files; they are **not** a verification of
the current file's content checksum. The index can contain stale references.

With `--root`, supply the installation's **data directory**, not `pso2_bin`.
The tool checks only the displayed matches, reads up to 32 bytes of each candidate,
and reports missing files or their size and recognized ICE/CPK signature. Other
directory layouts and localized variants may require a future mapping option.
No game files are downloaded, extracted, moved, or changed.

## Verified reference

Reviewed upstream commit `4c8a124a45ce0e0617bac424c27f1cba8fe75037` (2026-10-30).
Its index contained 86,073 rows. A Classic search for `Frontier Wing` found four
rows and returned distinct normal-quality, icon, and sound archive references
for the first row. This verifies index interoperability, not archive availability
or successful model import.

## Inspect supported ICE containers

```sh
cargo run --bin psp-client -- ice-list /path/to/archive.ice
```

The Rust reader currently accepts plaintext, uncompressed ICE v4 only. It checks
the declared archive size, both groups, entry bounds, filename boundaries, payload
lengths, and unexpected trailing data. Entry payloads borrow the original input;
the CLI lists names, sizes, groups, and payload offsets without writing files.
The CLI limits input to 256 MiB. CRC values are not verified. Encrypted, compressed,
flagged, and other-version containers are unsupported and return errors.

The layout was cross-checked against
[rust-ages-ice-archive's reader](https://github.com/HybridEidolon/rust-ages-ice-archive/blob/94b6b2e54776a2c45a54475bd5b75128067e116c/src/read.rs).
That library is not a dependency. A later, separate bounded v4 decoder adapts its
MIT-licensed key derivation and supports Blowfish/PRS; see
[Classic model support](classic-models.md). Tests use synthetic archives.

One 3,856-byte Vita translation entry was read into ignored local storage and
successfully inspected: group 2, `calor.text`, 3,440 bytes at offset `0x1a0`.
This establishes working container inspection on that sample, not support for
encrypted PC archives, NIFL model parsing, or rendering game geometry. Next,
extend the [validated Classic model adapter](classic-models.md). The graphical
viewer accepts OBJ, limited Classic Aqua, and supported ICE v4 model archives.
