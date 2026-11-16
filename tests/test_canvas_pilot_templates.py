"""Download wrappers must preserve the native starter files byte for byte."""
import pathlib
import unittest
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1] / "docs/templates/canvas-pilot"


class CanvasPilotTemplateDownloads(unittest.TestCase):
    def test_all_templates_archive_preserves_names_and_original_bytes(self):
        names = {path.name for path in ROOT.glob("canvas-pilot-*-2-layers.*")}
        self.assertEqual(len(names), 22)
        with zipfile.ZipFile(ROOT / "canvas-pilot-all-templates.zip") as archive:
            self.assertEqual(set(archive.namelist()), names)
            self.assertEqual(len(archive.namelist()), len(names))
            self.assertIsNone(archive.testzip())
            for name in names:
                self.assertEqual(archive.read(name), (ROOT / name).read_bytes())


if __name__ == "__main__":
    unittest.main()
