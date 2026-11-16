#!/usr/bin/env python3
"""Stage, build, check and publish the project's mdBook manuals (Python 3.11+)."""

import argparse
import html
from html.parser import HTMLParser
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib
from urllib.parse import quote, unquote, urljoin, urlsplit

ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "docs/manuals"
OUTPUT = ROOT / "build/manuals"
REPOSITORY = "https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/src/branch/main/"
DOWNLOADS = {".png", ".jpg", ".jpeg", ".svg", ".gif", ".json", ".csv", ".dxf", ".dwg", ".zip",
             ".html", ".drawio", ".pptx", ".fig", ".figcraft", ".ai", ".txt"}
LINK = re.compile(r"(?P<start>\]\()(?P<url>[^\s)]+)(?P<end>\))"
                  r"|(?P<attr>(?:href|src)=[\"'])(?P<html>[^\"']+)(?P<quote>[\"'])")


def source_path(root, base, path):
    """Resolve an existing source inside the repository, never following a symlink."""
    target = Path(os.path.abspath(base / unquote(path)))
    relative = target.relative_to(root)
    if target.resolve() != target or not target.exists():
        raise ValueError(f"Missing or symlinked manual source: {relative}")
    return relative


def chapter_path(path):
    # Avoid mdBook's special handling of README.md when rewriting links.
    return path.with_name("index.md") if path.name == "README.md" else path


def books(root=ROOT):
    result = []
    for config in sorted((root / "docs/manuals").glob("*/book.toml")):
        slug = config.parent.name
        if not re.fullmatch(r"[a-z][a-z0-9-]+", slug):
            raise ValueError(f"Invalid manual directory: {slug}")
        data = tomllib.loads(config.read_text())
        summary = config.with_name("SUMMARY.md").read_text()
        chapters = {}

        def entry(match):
            path = source_path(root, config.parent, match[1])
            if path.suffix != ".md" or not (root / path).is_file() or path in chapters:
                raise ValueError(f"Invalid or duplicate chapter: {path}")
            chapters[path] = chapter_path(path) if chapters else Path("index.md")
            return f"]({chapters[path].as_posix()})"

        staged_summary = re.sub(r"\]\(([^)]+)\)", entry, summary)
        if not chapters:
            raise ValueError(f"Empty manual: {slug}")
        icon = source_path(root, root, data["manual"]["icon"])
        result.append(dict(slug=slug, config=config, data=data, chapters=chapters,
                           summary=staged_summary, icon=icon))
    if not result:
        raise ValueError("No manual configurations found")
    return result


def stage_book(book, destination, root=ROOT):
    src = destination / "src"
    src.mkdir(parents=True)
    # mdBook sees only its own configuration, not the staging metadata.
    config = book["config"].read_text().split("[manual]", 1)[0]
    (destination / "book.toml").write_text(config)
    (src / "SUMMARY.md").write_text(book["summary"])
    for name in book["data"].get("output", {}).get("html", {}).get("additional-css", []):
        relative = Path(name)
        if relative.is_absolute() or ".." in relative.parts:
            raise ValueError(f"Stylesheet must be relative to the book: {name}")
        source = source_path(root, book["config"].parent, name)
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(root / source, target)


    def copy_asset(path):
        target = src / path
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(root / path, target)

    copy_asset(book["icon"])
    for original, staged in book["chapters"].items():
        def rewrite(url):
            parsed = urlsplit(html.unescape(url))
            if parsed.scheme or parsed.netloc or not parsed.path:
                return url
            path = source_path(root, (root / original).parent, parsed.path)
            suffix = ("?" + parsed.query if parsed.query else "")
            suffix += ("#" + parsed.fragment if parsed.fragment else "")
            if path in book["chapters"]:
                target = book["chapters"][path].with_suffix(".html")
            elif (root / path).is_file() and path.suffix.lower() in DOWNLOADS:
                copy_asset(path)
                target = path
            else:
                # Out-of-book documentation/code stays a working repository link.
                return REPOSITORY + quote(path.as_posix()) + suffix
            return quote(os.path.relpath(target, staged.parent).replace(os.sep, "/")) + suffix

        def replace(match):
            if match["url"] is not None:
                return match["start"] + rewrite(match["url"]) + match["end"]
            return match["attr"] + rewrite(match["html"]) + match["quote"]

        # Commands and examples must retain their literal paths and markup.
        chunks = re.split(r"(^```[^\n]*\n.*?^```[^\n]*(?:\n|$))",
                          (root / original).read_text(), flags=re.M | re.S)
        text = "".join(chunk if i % 2 else LINK.sub(replace, chunk)
                       for i, chunk in enumerate(chunks))
        portal = os.path.relpath("../index.html", staged.parent).replace(os.sep, "/")
        text = f'<p><a href="{portal}">All manuals</a></p>\n\n' + text
        target = src / staged
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)


class Page(HTMLParser):
    def __init__(self, text):
        super().__init__(convert_charrefs=True)
        self.links = []
        self.ids = set()
        self.base = None
        self.feed(text)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == "base":
            self.base = attrs.get("href")
            return
        for key in ("id", "name"):
            if key in attrs:
                self.ids.add(attrs[key])
        for key in ("href", "src"):
            if key in attrs:
                self.links.append(attrs[key])


def validate_site(site, manuals):
    """Check real rendered pages, including chapter fragments and downloads."""
    site = site.absolute()
    required = [site / "index.html"] + [site / b["slug"] / "index.html" for b in manuals]
    if any(not path.is_file() for path in required):
        raise ValueError("Manual output is incomplete: missing landing page or book index")
    if any(path.is_symlink() for path in [site, *site.rglob("*")]):
        raise ValueError("Manual output must not contain symlinks")
    pages = {p: Page(p.read_text()) for p in site.rglob("*.html")}
    count = 0
    for file, page in pages.items():
        for url in page.links:
            location = "/manuals/" + file.relative_to(site).as_posix()
            parsed = urlsplit(urljoin(page.base or location, url))
            if parsed.scheme or parsed.netloc:
                continue
            path = unquote(parsed.path)
            if not path.startswith("/manuals/"):
                raise ValueError(f"Link escapes manual site: {file}: {url}")
            dest = Path(os.path.abspath(site / path.removeprefix("/manuals/")))
            if not dest.is_relative_to(site):
                raise ValueError(f"Link escapes manual site: {file}: {url}")
            if dest.is_dir():
                dest /= "index.html"
            if not dest.exists():
                raise ValueError(f"Broken manual link: {file}: {url}")
            fragment = unquote(parsed.fragment)
            if fragment and dest in pages and fragment not in pages[dest].ids:
                raise ValueError(f"Broken manual anchor: {file}: {url}")
            count += 1
    print(f"Validated {len(pages)} HTML pages and {count} local links/assets.", flush=True)


def landing_page(manuals):
    cards = []
    for book in manuals:
        info = book["data"]["book"]
        cards.append(f'<a class="card" href="{book["slug"]}/index.html">'
                     f'<img src="{book["slug"]}/{book["icon"].as_posix()}" alt="">'
                     f'<h2>{html.escape(info["title"])}</h2>'
                     f'<p>{html.escape(info["description"])}</p></a>')
    return '''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Canvas Pilot manual</title><style>
:root { color-scheme: light dark; font-family: system-ui, sans-serif; }
body { max-width: 1000px; margin: 3rem auto; padding: 0 1.5rem; line-height: 1.5; }
main { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 1rem; }
.card { color: inherit; text-decoration: none; border: 1px solid #8888; border-radius: .8rem; padding: 1.5rem; }
.card:hover, .card:focus-visible { outline: 2px solid #7777dd; }
.card img { width: 64px; height: 64px; } h2 { font-size: 1.25rem; }
</style></head><body><h1>Canvas Pilot manual</h1>
<p>Guides and reference for local document, show and media workflows.</p><main>
''' + "\n".join(cards) + "\n</main></body></html>\n"


def build(mdbook, root=ROOT, output=OUTPUT):
    manuals = books(root)
    version = (root / "docs/manuals/mdbook-version").read_text().strip()
    actual = subprocess.check_output([mdbook, "--version"], text=True).strip()
    if actual != f"mdbook v{version}":
        raise ValueError(f"Expected mdbook v{version}, got {actual}")
    if output.is_symlink():
        raise ValueError("Refusing symlinked manual build directory")
    if output.exists():
        shutil.rmtree(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".manuals-", dir=output.parent) as temp:
        work = Path(temp)
        site = work / "site/manuals"
        site.mkdir(parents=True)
        for book in manuals:
            source = work / "sources" / book["slug"]
            stage_book(book, source, root)
            subprocess.run([mdbook, "build", str(source), "--dest-dir", str(site / book["slug"])],
                           check=True)
            # mdBook's 404 <base> otherwise sends its own heading links to index.html.
            not_found = site / book["slug"] / "404.html"
            not_found.write_text(not_found.read_text().replace('href="#', 'href="404.html#'))
        (site / "index.html").write_text(landing_page(manuals))
        validate_site(site, manuals)
        work.rename(output)
    print(f"Manuals ready: {output / 'site/manuals/index.html'}", flush=True)


def safe_destination(path):
    if not path.is_absolute():
        raise ValueError("Publication destination must be absolute")
    for component in [*reversed(path.parents), path]:
        if component.is_symlink() or (component.exists() and not component.is_dir()):
            raise ValueError(f"Unsafe publication directory: {component}")


def publish(site, destination, manuals, environment=None):
    environment = os.environ if environment is None else environment
    if environment.get("BRANCH_NAME") != "main" or environment.get("CHANGE_ID"):
        raise ValueError("Only non-PR main builds may publish manuals")
    safe_destination(destination)
    # A missing mount is an infrastructure error, not a reason to create /mnt/books.
    if not destination.parent.is_dir():
        raise ValueError(f"Publication parent/mount is missing: {destination.parent}")
    for book in manuals:
        safe_destination(destination / book["slug"])
    if (destination / "index.html").is_symlink() or (destination / "index.html").is_dir():
        raise ValueError("Unsafe manual landing-page destination")
    validate_site(site, manuals)
    destination.mkdir(exist_ok=True)
    # Delete stale pages only within each explicitly owned book, never the shared root.
    for book in manuals:
        target = destination / book["slug"]
        target.mkdir(exist_ok=True)
        subprocess.run(["rsync", "-a", "--delete", str(site / book["slug"]) + "/",
                        str(target) + "/"], check=True)
    # The shared landing page belongs to beffroi-tools; publish only our book.
    print(f"Published {len(manuals)} manuals to {destination}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    builder = commands.add_parser("build")
    builder.add_argument("--mdbook", default="mdbook")
    commands.add_parser("check")
    publisher = commands.add_parser("publish")
    publisher.add_argument("--destination", type=Path, default=Path("/mnt/books/manuals"))
    args = parser.parse_args()
    try:
        if args.command == "build":
            build(args.mdbook)
        elif args.command == "check":
            validate_site(OUTPUT / "site/manuals", books())
        else:
            publish(OUTPUT / "site/manuals", args.destination, books())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Manuals: {error}\n")


if __name__ == "__main__":
    main()
