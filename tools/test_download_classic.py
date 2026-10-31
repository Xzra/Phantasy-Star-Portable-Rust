import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import io

from download_classic import Entry, fetch, parse_manifest, select_entries, patch_bases


class ClassicDownloadTests(unittest.TestCase):
    def test_rejects_unofficial_sources_and_upgrades_official_urls_to_https(self):
        text = ('MasterURL=http://download.pso2.jp/patch_prod/master/patches/\n'
                'PatchURL=http://download.pso2.jp/patch_prod/patch/patches/')
        self.assertEqual(patch_bases(text)['m'], 'https://download.pso2.jp/patch_prod/master/patches/')
        for bad in ['other.example', 'download.pso2.jp.evil.example']:
            with self.assertRaises(ValueError):
                patch_bases(text.replace('download.pso2.jp', bad))

    def test_rejects_manifest_paths_that_escape_cache(self):
        for path in ['../outside.pat', '/outside.pat', 'data/../../outside.pat',
                     'C:/outside.pat', 'data\\outside.pat', 'data//outside.pat']:
            with self.subTest(path=path), self.assertRaises(ValueError):
                parse_manifest(f'{path}\t{"0" * 32}\t1\tm')

    def test_classic_selection_excludes_reboot_and_unlisted_win32(self):
        classic = Entry('data/win32/' + 'a' * 32 + '.pat', '0' * 32, 1, 'm')
        engine = Entry('sub/pso2classic.dll.pat', '0' * 32, 1, 'p')
        other = [Entry(p, '0' * 32, 1, 'm') for p in
                 ['data/win32reboot/aa/file.pat', 'sub/pso2reboot.dll.pat',
                  'data/win32/' + 'b' * 32 + '.pat']]
        self.assertEqual(select_entries([classic], [classic, engine, *other]), [classic, engine])
        with self.assertRaises(ValueError):
            select_entries(other[:1], [])

    def test_conflicting_manifest_entries_rejected(self):
        line = 'file.pat\t' + '0' * 32 + '\t1\tm'
        with self.assertRaises(ValueError):
            parse_manifest(line + '\n' + line.replace('\t1\t', '\t2\t'))

    def test_bad_download_never_replaces_existing_file(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            destination = root / 'pso2_bin/file'
            destination.parent.mkdir()
            destination.write_bytes(b'old')
            entry = Entry('file.pat', hashlib.md5(b'new').hexdigest(), 3, 'm')
            with patch('download_classic.urllib.request.urlopen', side_effect=lambda *a, **k: io.BytesIO(b'bad')), \
                    patch('download_classic.time.sleep'), self.assertRaises(OSError):
                fetch(entry, root, {'m': 'https://example.invalid/'})
            self.assertEqual(destination.read_bytes(), b'old')
            self.assertFalse(destination.with_name('file.download').exists())

    def test_successful_download_and_verified_resume(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            entry = Entry('file.pat', hashlib.md5(b'new').hexdigest(), 3, 'm')
            with patch('download_classic.urllib.request.urlopen', return_value=io.BytesIO(b'new')) as request:
                self.assertEqual(fetch(entry, root, {'m': 'https://example.invalid/'}), 'downloaded')
                self.assertEqual(fetch(entry, root, {'m': 'https://example.invalid/'}), 'cached')
                self.assertEqual(request.call_count, 1)

    def test_refuses_symlinked_cache_directory(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'pso2_bin').symlink_to(root, target_is_directory=True)
            with self.assertRaises(ValueError):
                fetch(Entry('file.pat', '0' * 32, 0, 'm'), root, {})


if __name__ == '__main__':
    unittest.main()
