import contextlib
import copy
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from xml.etree import ElementTree as ET

from tools import midra_probe as probe


def snapshot():
    layer = {'position': {'pp': {'posH': 1436, 'posV': 540}},
             'size': {'pp': {'sizeH': 967, 'sizeV': 544}},
             'crop': {'pp': dict.fromkeys(('left', 'right', 'top', 'bottom'), 0)},
             'mask': {'pp': dict.fromkeys(('left', 'right', 'top', 'bottom'), 0)}}
    screen = {'canvas': {'status': {'size': {'pp': {'sizeH': 1920, 'sizeV': 1080}}}},
              'presetList': {'items': {'UP': {'liveLayerList': {'items': {'1': layer}}},
                                       'DOWN': {'liveLayerList': {'items': {'1': copy.deepcopy(layer)}}}}}}
    screen['presetList']['items']['DOWN']['liveLayerList']['items']['1']['position']['pp']['posH'] = 100
    return {'device': {'screenList': {'items': {'2': screen}},
                       'transition': {'screenList': {'items': {'2': {'status': {'pp': {'transition': 'AT_DOWN'}}}}}}}}


class MidraProbeTests(unittest.TestCase):
    def test_preview_bank_and_half_pixel_container(self):
        data = snapshot()
        root = ET.fromstring(probe.container_svg(data, '2', ['1']))
        rect = root.find('{http://www.w3.org/2000/svg}rect')
        self.assertEqual(rect.attrib['x'], '952.5')
        self.assertEqual(rect.attrib['y'], '268.0')
        self.assertEqual(root.attrib['data-midra-layout-geometry'], '1')
        data['device']['transition']['screenList']['items']['2']['status']['pp']['transition'] = 'AT_UP'
        self.assertEqual(probe.preview_bank(data, '2'), 'DOWN')
        self.assertIn('x="-383.5"', probe.container_svg(data, '2', ['1']))

    def test_reject_transition_crop_mask_and_bad_selection(self):
        for field in ('crop', 'mask'):
            data = snapshot()
            data['device']['screenList']['items']['2']['presetList']['items']['UP']['liveLayerList']['items']['1'][field]['pp']['left'] = 18350
            with self.assertRaisesRegex(ValueError, field):
                probe.container_svg(data, '2', ['1'])
        for ids in ([], ['1', '1'], ['1', '2', '3']):
            with self.assertRaises(ValueError):
                probe.container_svg(snapshot(), '2', ids)
        data = snapshot()
        data['device']['transition']['screenList']['items']['2']['status']['pp']['transition'] = 'EFFECT_FROM_UP'
        with self.assertRaisesRegex(ValueError, 'transitioning'):
            probe.preview_bank(data, '2')

    def test_reject_invalid_numbers(self):
        for value in (True, '12', float('nan'), float('inf'), 1000001):
            with self.assertRaises(ValueError):
                probe.number(value)
        with self.assertRaises(ValueError):
            probe.number(0, positive=True)

    def test_network_is_get_only_and_bounded(self):
        with patch.object(probe, 'build_opener') as factory:
            response = factory.return_value.open.return_value.__enter__.return_value
            response.read.return_value = json.dumps(snapshot()).encode()
            self.assertEqual(probe.read_snapshot('192.0.2.1'), snapshot())
            factory.return_value.open.assert_called_once_with('http://192.0.2.1/api/stores/device', timeout=15)
            response.read.assert_called_once_with(probe.LIMIT + 1)
            response.read.return_value = b'x' * (probe.LIMIT + 1)
            with self.assertRaisesRegex(ValueError, '32 MiB'):
                probe.read_snapshot('192.0.2.1')
        with self.assertRaises(ValueError):
            probe.read_snapshot('example.org/path')
        self.assertIsNone(probe.NoRedirect().redirect_request(None, None, 302, '', {}, 'http://elsewhere'))

    def test_offline_import_and_no_overwrite(self):
        with tempfile.TemporaryDirectory() as folder, contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            source = Path(folder) / 'source.json'
            output = Path(folder) / 'import.svg'
            source.write_text(json.dumps(snapshot()))
            argv = ['--snapshot', str(source), '--svg', str(output), '--screen', '2', '--layers', '1']
            self.assertEqual(probe.main(argv), 0)
            saved = output.read_bytes()
            self.assertEqual(probe.main(argv), 1)
            self.assertEqual(output.read_bytes(), saved)
            self.assertEqual(probe.main(['--snapshot', str(source), '--svg', str(source)]), 1)
