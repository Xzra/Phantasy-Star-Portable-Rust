#!/usr/bin/env python3
"""Read plaintext CPK metadata locally or over FTP; never extract or upload files."""
import argparse
import ftplib
import json
import struct
from pathlib import Path

MAX_TABLE = 4 * 1024 * 1024


class Source:
    def __init__(self, path, host=None, port=1337):
        self.path, self.host, self.port = path, host, port

    def read(self, offset, size):
        if offset < 0 or not 0 < size <= MAX_TABLE:
            raise ValueError("metadata read exceeds the allowed bounds")
        if self.host:
            # A fresh control connection avoids depending on ABOR behavior after
            # a bounded read. VitaShell does not implement NLST on this device.
            ftp = ftplib.FTP()
            try:
                ftp.connect(self.host, self.port, timeout=10)
                ftp.login()
                ftp.voidcmd("TYPE I")
                with ftp.transfercmd("RETR " + self.path, rest=offset or None) as stream:
                    data = bytearray()
                    while len(data) < size:
                        block = stream.recv(min(65536, size - len(data)))
                        if not block:
                            break
                        data.extend(block)
            finally:
                ftp.close()
            result = bytes(data)
        else:
            with Path(self.path).open("rb") as stream:
                stream.seek(offset)
                result = stream.read(size)
        if len(result) != size:
            raise ValueError(f"short read at {offset}: expected {size}, got {len(result)}")
        return result


def utf_rows(data):
    """Parse bounded, unencrypted CRI @UTF metadata, rejecting unknown layouts."""
    if data[:4] != b"@UTF" or len(data) < 32:
        raise ValueError("expected a plaintext @UTF table")
    table_size, rows_offset, strings_offset, blobs_offset = struct.unpack_from(">4I", data, 4)
    if table_size + 8 != len(data):
        raise ValueError("invalid @UTF table size")
    rows_offset += 8
    strings_offset += 8
    blobs_offset += 8
    columns, row_size, row_count = struct.unpack_from(">HHI", data, 24)
    if not 32 <= rows_offset <= strings_offset <= blobs_offset <= len(data):
        raise ValueError("invalid @UTF section offsets")
    if rows_offset + row_size * row_count > strings_offset or row_count > 100000:
        raise ValueError("invalid @UTF row bounds")

    def string(offset):
        start = strings_offset + offset
        if not strings_offset <= start < blobs_offset:
            raise ValueError("string offset out of bounds")
        end = data.find(b"\0", start, blobs_offset)
        if end < 0:
            raise ValueError("unterminated metadata string")
        return data[start:end].decode("utf-8", errors="replace")

    def value(kind, offset, limit):
        formats = {0: ">B", 1: ">b", 2: ">H", 3: ">h", 4: ">I", 5: ">i",
                   6: ">Q", 7: ">q", 8: ">f", 9: ">d", 10: ">I", 11: ">II"}
        if kind not in formats:
            raise ValueError(f"unsupported @UTF value type {kind}")
        width = struct.calcsize(formats[kind])
        if offset + width > limit:
            raise ValueError("@UTF field crosses its section boundary")
        values = struct.unpack_from(formats[kind], data, offset)
        result = values[0]
        if kind == 10:
            result = string(result)
        elif kind == 11:
            start, length = values
            if blobs_offset + start + length > len(data):
                raise ValueError("blob out of bounds")
            result = {"offset": start, "size": length}
        return result, offset + width

    schema, cursor = [], 32
    for _ in range(columns):
        if cursor + 5 > rows_offset:
            raise ValueError("column descriptor out of bounds")
        flags, name = struct.unpack_from(">BI", data, cursor)
        cursor += 5
        storage, kind = flags & 0xF0, flags & 0x0F
        constant = 0
        if storage == 0x30:
            constant, cursor = value(kind, cursor, rows_offset)
        elif storage not in (0x10, 0x50):
            raise ValueError(f"unsupported @UTF storage {storage:#x}")
        schema.append((string(name), storage, kind, constant))
    rows = []
    for index in range(row_count):
        cursor = rows_offset + index * row_size
        limit = cursor + row_size
        row = {}
        for name, storage, kind, constant in schema:
            if storage == 0x50:
                row[name], cursor = value(kind, cursor, limit)
            else:
                row[name] = constant
        rows.append(row)
    return rows


def read_packet(source, offset, magic):
    header = source.read(offset, 16)
    if header[:4] != magic:
        raise ValueError(f"no plaintext {magic!r} header at {offset}; protected or unsupported data")
    size = struct.unpack_from("<Q", header, 8)[0]
    if not 32 <= size <= MAX_TABLE:
        raise ValueError(f"metadata packet size {size} exceeds supported bounds")
    return utf_rows(source.read(offset + 16, size))


def inspect(source):
    headers = read_packet(source, 0, b"CPK ")
    if len(headers) != 1:
        raise ValueError("expected exactly one CPK header row")
    header = headers[0]
    toc_offset = header.get("TocOffset")
    if not isinstance(toc_offset, int) or toc_offset <= 0:
        raise ValueError("archive does not expose a supported TOC")
    entries = read_packet(source, toc_offset, b"TOC ")
    return {"header": header, "entries": entries}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", help="local CPK path or absolute FTP path")
    parser.add_argument("--host", help="FTP host; anonymous, read-only access")
    parser.add_argument("--port", type=int, default=1337)
    parser.add_argument("--output", type=Path, help="write a JSON metadata report")
    args = parser.parse_args()
    try:
        report = inspect(Source(args.path, args.host, args.port))
        text = json.dumps(report, indent=2, ensure_ascii=False)
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(text + "\n", encoding="utf-8")
            print(f"Indexed {len(report['entries'])} entries: {args.output}")
        else:
            print(text)
    except (OSError, ValueError, EOFError, ftplib.Error, struct.error) as error:
        parser.exit(1, f"Cannot inspect CPK: {error}\n")


if __name__ == "__main__":
    main()
