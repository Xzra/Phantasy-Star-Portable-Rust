"""Synthetic fixtures only; no game archive data is checked in."""
import struct
import tempfile
import unittest
from pathlib import Path

from inspect_cpk import MAX_TABLE, Source, inspect, probe_entries, utf_rows


def table(columns, values):
    """Build a small @UTF table of per-row u64 fields for boundary tests."""
    strings = b"fixture\0"
    schema = bytearray()
    for name in columns:
        schema.extend(struct.pack(">BI", 0x56, len(strings)))
        strings += name.encode() + b"\0"
    rows_start = 32 + len(schema)
    rows = b"".join(struct.pack(">" + "Q" * len(columns), *row) for row in values)
    strings_start = rows_start + len(rows)
    end = strings_start + len(strings)
    header = b"@UTF" + struct.pack(">5IHHI", end - 8, rows_start - 8,
                                     strings_start - 8, end - 8, 0,
                                     len(columns), 8 * len(columns), len(values))
    return header + schema + rows + strings


def packet(magic, data):
    return magic + struct.pack("<IQ", 0xFF, len(data)) + data


class MetadataTests(unittest.TestCase):
    def test_reads_multiple_rows(self):
        self.assertEqual(utf_rows(table(["FileSize"], [(123,), (456,)])),
                         [{"FileSize": 123}, {"FileSize": 456}])

    def test_local_archive_header_and_toc(self):
        header = packet(b"CPK ", table(["TocOffset", "Files"], [(256, 2)]))
        toc = packet(b"TOC ", table(["FileSize"], [(123,), (456,)]))
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.cpk"
            path.write_bytes(header.ljust(256, b"\0") + toc)
            report = inspect(Source(path))
            self.assertEqual(report["header"]["Files"], 2)
            self.assertEqual(len(report["entries"]), 2)

    def test_rejects_truncated_and_corrupt_tables(self):
        original = table(["size"], [(123,)])
        for length in range(len(original)):
            with self.subTest(length=length), self.assertRaises(ValueError):
                utf_rows(original[:length])
        for offset in (8, 12, 16, 28):
            corrupted = bytearray(original)
            struct.pack_into(">I", corrupted, offset, 0xFFFFFFFF)
            with self.subTest(offset=offset), self.assertRaises(ValueError):
                utf_rows(corrupted)

    def test_rejects_unterminated_string_and_unknown_storage(self):
        corrupted = bytearray(table(["size"], [(123,)]))
        corrupted[-1] = ord("x")
        with self.assertRaises(ValueError):
            utf_rows(corrupted)
        corrupted = bytearray(table(["size"], [(123,)]))
        corrupted[32] = 0x76
        with self.assertRaises(ValueError):
            utf_rows(corrupted)

    def test_bounded_reads_and_protected_header(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "protected.cpk"
            path.write_bytes(b"x" * 16)
            source = Source(path)
            for offset, size in [(-1, 1), (0, 0), (0, MAX_TABLE + 1), (0, 17)]:
                with self.subTest(offset=offset, size=size), self.assertRaises(ValueError):
                    source.read(offset, size)
            with self.assertRaisesRegex(ValueError, "plaintext"):
                inspect(source)

    def test_probes_use_toc_base_and_preserve_size_mismatches(self):
        prefix = bytearray(32)
        prefix[:4] = b"ICE\0"
        struct.pack_into("<II", prefix, 8, 4, 128)
        struct.pack_into("<I", prefix, 28, 128)
        report = {"header": {"ContentOffset": 512, "TocOffset": 256},
                  "entries": [{"FileOffset": 256, "FileSize": 128, "FileName": "test.ice"}]}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.cpk"
            path.write_bytes(b"\0" * 512 + prefix)
            samples = probe_entries(Source(path), report, 1)
            self.assertEqual(samples[0]["absolute_offset"], 512)
            self.assertTrue(samples[0]["is_ice"])
            self.assertTrue(samples[0]["size_matches_toc"])
            report["entries"][0]["FileSize"] = 256
            self.assertFalse(probe_entries(Source(path), report, 1)[0]["size_matches_toc"])


if __name__ == "__main__":
    unittest.main()
