"""Read-only Midra snapshot capture and layer-container SVG import bridge."""
import argparse
import ipaddress
import json
import math
from pathlib import Path
import sys
from urllib.request import HTTPRedirectHandler, ProxyHandler, build_opener
from xml.etree import ElementTree as ET

LIMIT = 32 * 1024 * 1024


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def read_snapshot(host):
    """GET only; explicit IP, no proxies, credentials, redirects or write API."""
    address = ipaddress.ip_address(host)
    authority = f'[{address}]' if address.version == 6 else str(address)
    opener = build_opener(ProxyHandler({}), NoRedirect())
    with opener.open(f'http://{authority}/api/stores/device', timeout=15) as response:
        data = response.read(LIMIT + 1)
    if len(data) > LIMIT:
        raise ValueError('Device snapshot exceeds 32 MiB')
    return json.loads(data)


def preview_bank(snapshot, screen):
    transition = snapshot['device']['transition']['screenList']['items'][screen]
    state = transition['status']['pp']['transition']
    if state not in ('AT_UP', 'AT_DOWN'):
        raise ValueError(f'Screen {screen} is transitioning ({state}); retry when idle')
    return 'DOWN' if state == 'AT_UP' else 'UP'


def number(value, *, positive=False):
    if isinstance(value, bool) or not isinstance(value, (float, int)):
        raise ValueError('Expected numeric device geometry')
    if not math.isfinite(value) or abs(value) > 1_000_000 or (positive and value <= 0):
        raise ValueError('Invalid device geometry')
    return value


def container_svg(snapshot, screen, layers):
    """Export device containers, deliberately not inferred visible/cropped bounds."""
    if not 1 <= len(layers) <= 2 or len(set(layers)) != len(layers):
        raise ValueError('Choose one or two distinct layer IDs to match editor selection')
    bank = preview_bank(snapshot, screen)
    data = snapshot['device']['screenList']['items'][screen]
    size = data['canvas']['status']['size']['pp']
    width, height = (number(size[k], positive=True) for k in ('sizeH', 'sizeV'))
    root = ET.Element('svg', {
        'xmlns': 'http://www.w3.org/2000/svg', 'data-midra-layout-geometry': '1',
        'width': str(width), 'height': str(height), 'viewBox': f'0 0 {width} {height}',
    })
    ET.SubElement(root, 'title').text = f'Midra screen {screen} Preview layer containers'
    for layer_id in layers:
        layer = data['presetList']['items'][bank]['liveLayerList']['items'][layer_id]
        # Never silently treat cropped/masked containers as the final visible box.
        for field in ('crop', 'mask'):
            edges = layer[field]['pp']
            if any(number(edges[edge]) != 0 for edge in ('left', 'right', 'top', 'bottom')):
                raise ValueError(f'Layer {layer_id} has {field}; visible bounds conversion is not verified')
        w, h = (number(layer['size']['pp'][k], positive=True) for k in ('sizeH', 'sizeV'))
        cx, cy = (number(layer['position']['pp'][k]) for k in ('posH', 'posV'))
        ET.SubElement(root, 'rect', {
            'id': f'screen-{screen}-layer-{layer_id}',
            'x': str(cx - w / 2), 'y': str(cy - h / 2),
            'width': str(w), 'height': str(h), 'fill': '#173b50',
        })
    ET.indent(root)
    return ET.tostring(root, encoding='unicode') + '\n'


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--host', help='Device IP address; read-only HTTP GET')
    source.add_argument('--snapshot', type=Path, help='Previously captured device JSON; no network')
    parser.add_argument('--save-snapshot', type=Path, help='Save raw vendor state (may contain private configuration)')
    parser.add_argument('--svg', type=Path, help='Export Preview container geometry for Import SVG')
    parser.add_argument('--screen', default='1')
    parser.add_argument('--layers', nargs='+', help='Explicit device layer IDs, one or two in mapping order')
    args = parser.parse_args(argv)
    try:
        if args.svg and not args.layers:
            raise ValueError('--svg requires --layers; device slots are not editor slots')
        outputs = [p for p in (args.save_snapshot, args.svg) if p is not None]
        if len({p.resolve() for p in outputs}) != len(outputs):
            raise ValueError('Snapshot and SVG require different output files')
        if any(p.exists() for p in outputs):
            raise ValueError('Output already exists; choose a new filename')
        if args.snapshot:
            with args.snapshot.open('rb') as stream:
                raw = stream.read(LIMIT + 1)
            if len(raw) > LIMIT:
                raise ValueError('Snapshot exceeds 32 MiB')
            snapshot = json.loads(raw)
        else:
            snapshot = read_snapshot(args.host)
        bank = preview_bank(snapshot, args.screen)
        svg = container_svg(snapshot, args.screen, args.layers) if args.svg else None
        if args.save_snapshot:
            with args.save_snapshot.open('x', encoding='utf-8') as stream:
                stream.write(json.dumps(snapshot, indent=2, allow_nan=False) + '\n')
        if args.svg:
            with args.svg.open('x', encoding='utf-8') as stream:
                stream.write(svg)
            print('SVG contains layer containers only; source framing, effects and routing are not imported.')
        print(f'Read screen {args.screen}: Preview = {bank}. No device writes.')
        return 0
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f'Midra read/import failed: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
