"""Portable native documents and safe generation for the offline conference kit."""
import csv
import importlib.util
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
import zipfile
import zlib

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / 'scripts/prepare-conference-showcase.py'
spec = importlib.util.spec_from_file_location('conference_showcase', SCRIPT)
kit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kit)


def portrait_png():
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 3, 4, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress((b'\x00' + b'\xff' * 9) * 4)) + chunk(b'IEND', b''))


class ConferenceShowcaseTests(unittest.TestCase):
    def test_native_slide_deck_has_editable_text_and_portable_relationships(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'slides.pptx'
            kit.pptx(kit.slides(), path)
            with zipfile.ZipFile(path) as archive:
                self.assertIsNone(archive.testzip())
                # LibreOffice rejected prefixed OPC roots although the app importer accepted them.
                for name, tag in [('[Content_Types].xml', 'Types'), ('ppt/_rels/presentation.xml.rels', 'Relationships')]:
                    self.assertIn(f'<{tag} xmlns='.encode(), archive.read(name))
                for number in range(1, 4):
                    slide = ET.fromstring(archive.read(f'ppt/slides/slide{number}.xml'))
                    texts = slide.findall(f'.//{{{kit.A}}}t')
                    self.assertGreaterEqual(len(texts), 10)
                    self.assertFalse(slide.findall(f'.//{{{kit.P}}}pic'))
                self.assertIn(b'From signals', archive.read('ppt/slides/slide1.xml'))

    def test_geometry_csv_contract_and_projects_have_no_device_targets(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            kit.save(out/'media/speaker/alex-morgan-synthetic.png', portrait_png())
            kit.save(out/'delivery-jpg-only/program-example.png', portrait_png())
            for name, boxes in kit.LAYOUTS.items():
                kit.geometry_files(out, name, boxes)
                folder = out/'layouts'/name
                with (folder/'layout.csv').open(newline='') as source:
                    rows = list(csv.DictReader(source))
                self.assertEqual(len(rows), len(boxes))
                self.assertTrue(all(row['midra_geometry_version'] == '1' and row['canvas_width'] == '1920' for row in rows))
                self.assertEqual([row['name'] for row in rows], [box[0] for box in boxes])
                project = json.loads((folder/'ready-to-align.pilot.json').read_text())
                self.assertFalse(project['input_proposals'])
                self.assertTrue(all(layer['device_link'] is None for layer in project['layers']))
                self.assertTrue(all(source['source_format'] == 'custom' for source in project['inputs']))
                for layer in project['layers']:
                    if layer['name'] == 'Speaker':
                        self.assertEqual(layer['layer_fit'], 'centered')

    def test_existing_output_is_preserved_and_non_portrait_input_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            marker = out/'keep.txt'; marker.write_text('original')
            result = subprocess.run([sys.executable, str(SCRIPT), '--speaker', str(out/'missing.png'), '--output', str(out)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Refusing to replace', result.stderr)
            self.assertEqual(marker.read_text(), 'original')
            bad = out/'square.png'
            bad.write_bytes(b'\x89PNG\r\n\x1a\n' + b'\x00'*8 + struct.pack('>II', 4, 4))
            with self.assertRaisesRegex(ValueError, '3:4'):
                kit.png_dimensions(bad)
