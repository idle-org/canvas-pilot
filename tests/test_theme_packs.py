"""Example ZIPs remain reproducible and contain only their editable manifest."""
import importlib.util
import io
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('build_theme_packs', ROOT / 'scripts/build-theme-packs.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ThemePackExamples(unittest.TestCase):
    def test_archives_match_sources_and_rebuild_exactly(self):
        manifests = list((ROOT / 'docs/theme-packs').glob('*/manifest.json'))
        self.assertEqual(len(manifests), 4)
        for source in manifests:
            with self.subTest(pack=source.parent.name):
                expected = source.parent.with_suffix('.zip').read_bytes()
                self.assertEqual(module.archive(source.read_bytes()), expected)
                with zipfile.ZipFile(io.BytesIO(expected)) as archive:
                    self.assertEqual(archive.namelist(), ['manifest.json'])
                    self.assertEqual(archive.read('manifest.json'), source.read_bytes())
