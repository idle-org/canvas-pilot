#!/usr/bin/env python3
"""Build deterministic example theme ZIPs, or check their exact source bytes."""
import argparse
from pathlib import Path
import io
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for source in sorted((ROOT / 'docs/theme-packs').glob('*/manifest.json')):
        target = source.parent.with_suffix('.zip')
        data = archive(source.read_bytes())
        if args.check:
            if not target.exists() or target.read_bytes() != data:
                raise SystemExit(f'Out-of-date theme pack: {target}')
        else:
            target.write_bytes(data)
        print(target.relative_to(ROOT))


if __name__ == '__main__':
    main()
