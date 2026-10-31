#!/usr/bin/env python3
"""Download SEGA's Classic-only manifest into a local research cache (not an installer)."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
import hashlib
import http.client
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import threading
import time
import urllib.request
import urllib.parse


def patch_bases(management):
    values = dict(line.split('=', 1) for line in management.splitlines() if '=' in line)
    bases = {}
    for key, setting in [('m', 'MasterURL'), ('p', 'PatchURL')]:
        url = values[setting].replace('http://', 'https://', 1)
        if not re.fullmatch(r'https://download\.pso2\.jp/patch_prod/[A-Za-z0-9_]+/patches/', url):
            raise ValueError('only official SEGA patch URLs are accepted')
        bases[key] = url
    return bases


def refresh_manifests(root):
    # The launcher's HTTP endpoint supplies current full URLs; the similarly named
    # download-host management file can describe an obsolete, unavailable version.
    def read(url, limit):
        request = urllib.request.Request(url, headers={'User-Agent': 'AQUA_HTTP'})
        with urllib.request.urlopen(request, timeout=45) as response:
            data = response.read(limit + 1)
        if len(data) > limit:
            raise ValueError('manifest exceeds its read limit')
        return data.decode('utf-8-sig')

    management = read('http://patch01.pso2gs.net/patch_prod/patches/management_beta.txt', 16384)
    bases = patch_bases(management)
    classic = read(bases['p'] + 'patchlist_classic.txt', 64 * 1024**2)
    all_files = read(bases['p'] + 'patchlist_all.txt', 64 * 1024**2)
    select_entries(parse_manifest(classic), parse_manifest(all_files))
    root.mkdir(parents=True, exist_ok=True)
    for name, text in [('management_beta.txt', management), ('patchlist_classic.txt', classic),
                       ('patchlist_all.txt', all_files)]:
        temp = root / (name + '.tmp')
        temp.write_text(text, encoding='utf-8')
        temp.replace(root / name)


class PersistentOpener:
    """One HTTPS connection per worker; retain urllib's proxy support when configured."""
    def __init__(self):
        self.local = threading.local()

    def __call__(self, request, timeout=45):
        if urllib.request.getproxies():
            return urllib.request.urlopen(request, timeout=timeout)
        url = urllib.parse.urlsplit(request.full_url)
        connection = getattr(self.local, 'connection', None)
        if connection is None or getattr(self.local, 'host', None) != url.netloc:
            if connection:
                connection.close()
            connection = http.client.HTTPSConnection(url.netloc, timeout=timeout)
            self.local.connection = connection
            self.local.host = url.netloc
        try:
            connection.request('GET', url.path, headers=dict(request.header_items()))
            response = connection.getresponse()
            if response.status != 200:
                response.close()
                raise OSError(f'HTTP {response.status}')
            return response
        except (OSError, http.client.HTTPException):
            connection.close()
            self.local.connection = None
            raise


@dataclass(frozen=True)
class Entry:
    path: str
    checksum: str
    size: int
    source: str


def parse_manifest(text):
    entries = {}
    for number, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        fields = line.split('\t')
        if len(fields) != 4:
            raise ValueError(f'line {number}: expected four tab-separated fields')
        path, checksum, size, source = fields
        parts = PurePosixPath(path).parts
        if (not parts or path.startswith('/') or '\\' in path or ':' in path
                or any(p in ('..', '.') for p in path.split('/'))
                or not path.endswith('.pat') or any(not p for p in path.split('/'))):
            raise ValueError(f'line {number}: unsafe archive path')
        if not re.fullmatch('[0-9a-fA-F]{32}', checksum) or not size.isdecimal():
            raise ValueError(f'line {number}: invalid checksum or size')
        if source not in ('m', 'p') or int(size) > 4 * 1024**3:
            raise ValueError(f'line {number}: unsupported source or file size')
        entry = Entry(path, checksum.lower(), int(size), source)
        if path in entries and entries[path] != entry:
            raise ValueError(f'line {number}: conflicting duplicate')
        entries[path] = entry
    return list(entries.values())


def select_entries(classic, all_files):
    # The dedicated list is authoritative. Do not assume all win32 files are Classic.
    if not classic or any(not re.fullmatch(r'data/win32/[0-9a-f]{32}\.pat', e.path)
                          for e in classic):
        raise ValueError('expected a nonempty Classic-only archive manifest')
    support = {'sub/pso2classic.dll.pat', 'sub/pso2bridge.dll.pat',
               'sub/pso2.exe.pat', 'sub/version.ver.pat'}
    # Config and licensing data help document the engine without downloading reboot.
    return classic + [e for e in all_files if e.path in support
                      or e.path.startswith(('data/license/', 'data/win32/script/'))]


def verified(path, entry):
    if not path.is_file() or path.stat().st_size != entry.size:
        return False
    digest = hashlib.md5()  # Publisher's file checksum, not a security signature.
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest() == entry.checksum


def fetch(entry, root, bases, opener=None):
    destination = root / 'pso2_bin' / entry.path.removesuffix('.pat')
    # Refuse cache symlinks instead of allowing writes outside the requested root.
    if any(p.is_symlink() for p in [destination, *destination.parents]):
        raise ValueError('cache contains a symlink')
    if verified(destination, entry):
        return 'cached'
    destination.parent.mkdir(parents=True, exist_ok=True)
    temp = destination.with_name(destination.name + '.download')
    if temp.is_symlink():
        raise ValueError('temporary file is a symlink')
    error = None
    for attempt in range(3):
        try:
            request = urllib.request.Request(bases[entry.source] + entry.path,
                                             headers={'User-Agent': 'AQUA_HTTP'})
            with (opener or urllib.request.urlopen)(request, timeout=45) as response, temp.open('wb') as out:
                remaining = entry.size
                while remaining:
                    block = response.read(min(1024 * 1024, remaining))
                    if not block:
                        raise ValueError('truncated download')
                    out.write(block)
                    remaining -= len(block)
                if response.read(1):
                    raise ValueError('download larger than manifest size')
            if not verified(temp, entry):
                raise ValueError('publisher checksum mismatch')
            temp.replace(destination)
            return 'downloaded'
        except (OSError, ValueError, http.client.HTTPException) as exc:
            error = exc
            temp.unlink(missing_ok=True)
            if attempt < 2:
                time.sleep(attempt + 1)
    raise OSError(f'{entry.path}: {error}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path, help='cache with saved patchlist_classic.txt and patchlist_all.txt')
    parser.add_argument('--workers', type=int, default=8, choices=range(1, 17))
    parser.add_argument('--download', action='store_true', help='without this, only report the download plan')
    parser.add_argument('--refresh-manifests', action='store_true', help='fetch current official manifests before planning')
    args = parser.parse_args()
    root = args.root.resolve()
    if args.refresh_manifests:
        refresh_manifests(root)
    classic = parse_manifest((root / 'patchlist_classic.txt').read_text())
    entries = select_entries(classic, parse_manifest((root / 'patchlist_all.txt').read_text()))
    bases = patch_bases((root / 'management_beta.txt').read_text())
    total = sum(e.size for e in entries)
    print(f'{len(entries)} files, {total / 1024**3:.2f} GiB; {len(classic)} Classic archives', flush=True)
    if not args.download:
        return
    if shutil.disk_usage(root).free < total + 1024**3:
        raise OSError('insufficient free space for the complete cache and temporary files')
    # Inspect engine support first; then small archives make early research possible.
    entries.sort(key=lambda e: (e.path.startswith('data/win32/') and '/script/' not in e.path, e.size))
    started = time.monotonic()
    status = {'total_files': len(entries), 'total_bytes': total, 'verified_files': 0,
              'verified_bytes': 0, 'downloaded_files': 0, 'cached_files': 0, 'failed_files': 0,
              'complete': False, 'sources': bases}
    failures = []
    stop = threading.Event()
    opener = PersistentOpener()

    def report():
        status['elapsed_seconds'] = round(time.monotonic() - started)
        temp = root / 'download-status.json.tmp'
        temp.write_text(json.dumps(status, indent=2) + '\n')
        temp.replace(root / 'download-status.json')
        print(f"{status['verified_files']}/{len(entries)} verified; "
              f"{status['verified_bytes'] / 1024**3:.2f} GiB; "
              f"{status['failed_files']} failures", flush=True)

    def worker(entry):
        if stop.is_set():
            return 'cancelled'
        return fetch(entry, root, bases, opener)

    report()
    try:
        with ThreadPoolExecutor(max_workers=args.workers) as pool:
            pending = {pool.submit(worker, e): e for e in entries}
            last_report = time.monotonic()
            try:
                for future in as_completed(pending):
                    entry = pending[future]
                    try:
                        result = future.result()
                        if result == 'cancelled':
                            continue
                        status['verified_files'] += 1
                        status['verified_bytes'] += entry.size
                        status[result + '_files'] += 1
                    except (OSError, ValueError) as exc:
                        failures.append(str(exc))
                        status['failed_files'] += 1
                        print(str(exc), flush=True)
                    if time.monotonic() - last_report >= 10:
                        report()
                        last_report = time.monotonic()
            except KeyboardInterrupt:
                stop.set()
                for future in pending:
                    future.cancel()
                raise
    finally:
        status['complete'] = status['verified_files'] == len(entries)
        (root / 'download-failures.json').write_text(json.dumps(failures, indent=2) + '\n')
        report()
    if not status['complete']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
