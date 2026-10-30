# Vita asset investigation

Read-only observations from the user's Vita FTP installation, 2026-10-30:

| Location | Observation |
| --- | --- |
| `ux0:/app/PCSG00141/data/vita/pso2.cpk` | 2,195,894,512 bytes; sampled prefix did not expose a plaintext `CPK ` signature |
| `ux0:/app/PCSG00141/data/vita/sy_script.fpk` | 6,292,357 bytes; format not inspected |
| `ux0:/rePatch/PCSG00141/data/vita/patches/999.cpk` | 15,928,280 bytes; readable `CPK ` and `@UTF` metadata |

The translation archive indexes 66 `.ice` entries, with directory names including
`actor`, `calor`, and `clientorder`. These are text-oriented names; no model has
been imported from them. Header metadata reports TOC offset 2048 and content
offset 6144. The header's file count matches the 66 parsed TOC rows.

The base archive's sampled bytes alone do not identify a supported format. This
may reflect the installation's protection/view through FTP; that is a hypothesis,
not a confirmed diagnosis. The translation archive does not establish that the
base archive is readable or that Vita and PC payloads share identical layouts.

## Reproduce metadata inspection

Python 3 standard library only; no extraction, uploads, or remote writes:

```sh
python3 tools/inspect_cpk.py \
  /ux0:/rePatch/PCSG00141/data/vita/patches/999.cpk \
  --host 10.0.0.7 --port 1337 --output local/vita-patch-index.json

python3 tools/inspect_cpk.py /path/to/readable.cpk --output local/cpk-index.json
```

The tool reads CPK and TOC metadata packets, limits each read to 4 MiB, checks
section/row bounds, and rejects unsupported or non-plaintext headers. FTP uses
anonymous login and fresh connections for bounded reads because VitaShell listing
and abort behavior differs from a full FTP server. Reports are ignored by Git.

This is a metadata inspector, not a CPK extractor or ICE decoder. File offsets
remain in their original TOC form; validate the container's offset base before
payload access. Encrypted UTF tables, ITOC-only archives, compressed entries,
and arbitrary game formats are not supported.

Next, compare the upcoming PC installation with this layout and locate readable
model payloads. The original OBJ sample keeps viewer work testable meanwhile.
