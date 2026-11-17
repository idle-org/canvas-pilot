"""Example ZIPs remain reproducible and contain only their editable manifest."""
import importlib.util
import io
import json
from pathlib import Path
import unittest
from unittest import mock
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('build_theme_packs', ROOT / 'scripts/build-theme-packs.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ThemePackExamples(unittest.TestCase):
    def test_rebuild_does_not_depend_on_zlib_or_host_platform(self):
        sources = sorted((ROOT / 'docs/theme-packs').glob('*/manifest.json'))
        manifests = [source.read_bytes() for source in sources]
        manifests.extend(module.collection(sources, name) for name in module.COLLECTIONS)
        for manifest in manifests:
            expected = module.archive(manifest)
            with mock.patch.object(zipfile, 'zlib', None), mock.patch('sys.platform', 'win32'):
                self.assertEqual(module.archive(manifest), expected)
            with zipfile.ZipFile(io.BytesIO(expected)) as archive:
                info = archive.getinfo('manifest.json')
                self.assertEqual(info.compress_type, zipfile.ZIP_STORED)
                self.assertEqual(info.create_system, 3)
                self.assertEqual(archive.read(info), manifest)

    def test_archives_match_sources_and_rebuild_exactly(self):
        manifests = list((ROOT / 'docs/theme-packs').glob('*/manifest.json'))
        self.assertEqual(len(manifests), 17)
        for source in manifests:
            with self.subTest(pack=source.parent.name):
                expected = source.parent.with_suffix('.zip').read_bytes()
                self.assertEqual(module.archive(source.read_bytes()), expected)
                with zipfile.ZipFile(io.BytesIO(expected)) as archive:
                    self.assertEqual(archive.namelist(), ['manifest.json'])
                    self.assertEqual(archive.read('manifest.json'), source.read_bytes())

    def test_collections_partition_every_theme_and_preserve_notices(self):
        sources = list((ROOT / 'docs/theme-packs').glob('*/manifest.json'))
        assigned = set()
        for name, members in module.COLLECTIONS.items():
            self.assertFalse(assigned & members)
            assigned.update(members)
            data = module.collection(sources, name)
            self.assertEqual(data, module.collection(reversed(sources), name))
            self.assertEqual(module.archive(data), (ROOT / f'docs/theme-packs/{name}.zip').read_bytes())
            combined = json.loads(data)
            self.assertEqual(combined['id'], name)
            self.assertEqual(len(combined['themes']), len(members))
            self.assertLessEqual(len(combined['themes']), 16)
            self.assertEqual(len({t['id'] for t in combined['themes']}), len(members))
            for source in sources:
                if source.parent.name not in members:
                    continue
                manifest = json.loads(source.read_bytes())
                self.assertIn(manifest['notices'], combined['notices'])
                for theme in manifest['themes']:
                    self.assertIn({**theme, 'id': manifest['id'] + '-' + theme['id']}, combined['themes'])
        self.assertEqual(assigned, {source.parent.name for source in sources})
        self.assertIn('ravenswatch', module.COLLECTIONS['video-game-themes'])
        self.assertEqual(len(module.COLLECTIONS['video-game-themes']), 6)
        self.assertEqual(len(module.COLLECTIONS['classic-themes']), 11)
        self.assertFalse((ROOT / 'docs/theme-packs/extra-themes.zip').exists())

    def test_missing_or_uncategorized_collection_manifest_is_rejected(self):
        sources = list((ROOT / 'docs/theme-packs').glob('*/manifest.json'))
        with self.assertRaisesRegex(ValueError, 'required collection manifest'):
            module.collection((source for source in sources if source.parent.name != 'dispatch'), 'video-game-themes')
        with self.assertRaisesRegex(ValueError, 'no collection assignment'):
            module.collection(sources + [ROOT / 'docs/theme-packs/unknown/manifest.json'], 'classic-themes')
