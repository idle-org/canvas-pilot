#!/usr/bin/env python3
"""Build deterministic example theme ZIPs, or check their exact source bytes."""
import argparse
from pathlib import Path
import io
import json
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def archive(manifest):
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w') as output:
        info = zipfile.ZipInfo('manifest.json', (2026, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o100644 << 16
        output.writestr(info, manifest)
    return buffer.getvalue()


def collection(sources):
    """One normal v1 pack: the existing importer installs every contained theme."""
    themes = []
    notices = []
    for source in sorted(sources):
        manifest = json.loads(source.read_bytes())
        for theme in manifest['themes']:
            themes.append({**theme, 'id': manifest['id'] + '-' + theme['id']})
        notices.append(manifest['id'] + '\n' + manifest.get('notices', ''))
    if not 1 <= len(themes) <= 16:
        raise ValueError('The collection must respect the v1 limit of 16 themes')
    manifest = dict(version=1, id='extra-themes', revision=1,
                    themes=themes, notices='\n\n'.join(notices))
    return (json.dumps(manifest, indent=2) + '\n').encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    sources = sorted((ROOT / 'docs/theme-packs').glob('*/manifest.json'))
    outputs = [(source.parent.with_suffix('.zip'), source.read_bytes()) for source in sources]
    outputs.append((ROOT / 'docs/theme-packs/extra-themes.zip', collection(sources)))
    for target, manifest in outputs:
        data = archive(manifest)
        if args.check:
            if not target.exists() or target.read_bytes() != data:
                raise SystemExit(f'Out-of-date theme pack: {target}')
        else:
            target.write_bytes(data)
        print(target.relative_to(ROOT))


if __name__ == '__main__':
    main()
