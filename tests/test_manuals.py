import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("manuals", ROOT / "scripts/manuals.py")
manuals = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(manuals)


class ManualTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        return path

    def book(self):
        self.write("docs/manuals/example/book.toml", '''[book]
title = "Example"
description = "Example manual"
src = "src"
[manual]
icon = "docs/icon.png"
''')
        self.write("docs/manuals/example/SUMMARY.md", "# Summary\n\n"
                   "- [Intro](../../tool/README.md)\n- [Details](../../tool/detail.md)\n")
        self.write("docs/icon.png", "image fixture")
        self.write("docs/tool/README.md", '# Intro\n\n[Detail](detail.md#values)\n'
                   '[Other](../other.md)\n![Icon](../icon.png)\n'
                   '<img src="../icon.png">\n[Data](../sample.json)\n'
                   '```md\n[Literal](missing.md)\n```\n')
        self.write("docs/tool/detail.md", "# Details\n\n## Values\n\n[Home](README.md)\n")
        self.write("docs/other.md", "# Other")
        self.write("docs/sample.json", '{"value": 42}')
        return manuals.books(self.root)[0]

    def test_stage_rebases_chapters_assets_and_repository_links_without_copying_prose(self):
        book = self.book()
        before = (self.root / "docs/tool/README.md").read_bytes()
        stage = self.root / "stage"
        manuals.stage_book(book, stage, self.root)
        intro = (stage / "src/index.md").read_text()
        self.assertIn("](docs/tool/detail.html#values)", intro)
        self.assertIn(manuals.REPOSITORY + "docs/other.md", intro)
        self.assertIn('src="docs/icon.png"', intro)
        self.assertIn("[Data](docs/sample.json)", intro)
        self.assertIn("[Literal](missing.md)", intro)
        self.assertIn("[Home](../../index.html)", (stage / "src/docs/tool/detail.md").read_text())
        self.assertEqual((stage / "src/docs/sample.json").read_bytes(),
                         (self.root / "docs/sample.json").read_bytes())
        self.assertFalse((stage / "src/docs/other.md").exists())
        self.assertNotIn("[manual]", (stage / "book.toml").read_text())
        self.assertEqual((self.root / "docs/tool/README.md").read_bytes(), before)

    def test_missing_duplicate_outside_and_symlinked_sources_are_rejected(self):
        self.book()
        summary = self.root / "docs/manuals/example/SUMMARY.md"
        for links in ("../../missing.md", "../../../../outside.md"):
            summary.write_text(f"- [Bad]({links})\n")
            with self.assertRaises(ValueError):
                manuals.books(self.root)
        summary.write_text("- [One](../../tool/README.md)\n- [Again](../../tool/README.md)\n")
        with self.assertRaises(ValueError):
            manuals.books(self.root)
        summary.write_text("- [Link](../../linked.md)\n")
        (self.root / "docs/linked.md").symlink_to(self.root / "docs/tool/README.md")
        with self.assertRaises(ValueError):
            manuals.books(self.root)

    def site(self):
        site = self.root / "site"
        self.write("site/index.html", '<a href="example/index.html">Example</a>')
        self.write("site/example/index.html", '<h1 id="intro">Intro</h1>'
                   '<a href="../index.html">Manuals</a><a href="#intro">Intro</a>'
                   '<a href="downloads/file.json">File</a>')
        self.write("site/example/downloads/file.json", '{"value": 42}')
        return site, [{"slug": "example"}]

    def test_rendered_validation_checks_fragments_downloads_and_subpath_404_base(self):
        site, books = self.site()
        page = self.write("site/example/404.html", '<base href="/manuals/example/">'
                          '<h1 id="missing">Missing</h1><a href="404.html#missing">Missing</a>')
        manuals.validate_site(site, books)
        for url in ("index.html#missing", "absent.json", "../../outside.html"):
            page.write_text(f'<a href="{url}">Invalid</a>')
            with self.assertRaises(ValueError):
                manuals.validate_site(site, books)
        page.unlink()
        (site / "example/downloads/file.json").unlink()
        with self.assertRaises(ValueError):
            manuals.validate_site(site, books)

    @unittest.skipUnless(shutil.which("rsync"), "rsync is required for publication integration")
    def test_publish_replaces_only_owned_manual_preserving_shared_landing_page(self):
        site, books = self.site()
        destination = self.root / "public/manuals"
        self.write("public/manuals/example/stale.html", "stale")
        sibling = self.write("public/manuals/unrelated-book/keep.html", "keep")
        root_file = self.write("public/manuals/keep.txt", "keep")
        self.write("public/manuals/index.html", "old")
        manuals.publish(site, destination, books, {"BRANCH_NAME": "main"})
        self.assertFalse((destination / "example/stale.html").exists())
        self.assertEqual((destination / "index.html").read_text(), "old")
        self.assertEqual((destination / "example/downloads/file.json").read_bytes(),
                         (site / "example/downloads/file.json").read_bytes())
        self.assertEqual(sibling.read_text(), "keep")
        self.assertEqual(root_file.read_text(), "keep")
        (destination / "index.html").unlink()
        manuals.publish(site, destination, books, {"BRANCH_NAME": "main"})
        self.assertFalse((destination / "index.html").exists())

    def test_publish_rejects_non_main_pr_incomplete_output_and_missing_mount(self):
        site, books = self.site()
        destination = self.root / "manuals"
        for env in ({}, {"BRANCH_NAME": "feature"}, {"BRANCH_NAME": "main", "CHANGE_ID": "3"}):
            with self.assertRaises(ValueError):
                manuals.publish(site, destination, books, env)
        with self.assertRaises(ValueError):
            manuals.publish(site, destination, books + [{"slug": "absent"}], {"BRANCH_NAME": "main"})
        self.assertFalse(destination.exists())
        with self.assertRaises(ValueError):
            manuals.publish(site, self.root / "missing-mount/manuals", books, {"BRANCH_NAME": "main"})
        self.assertFalse((self.root / "missing-mount").exists())

    def test_publish_rejects_symlinked_output_and_destination_before_writing(self):
        site, books = self.site()
        destination = self.root / "manuals"
        destination.mkdir()
        outside = self.root / "outside"
        outside.mkdir()
        (destination / "example").symlink_to(outside, target_is_directory=True)
        with self.assertRaises(ValueError):
            manuals.publish(site, destination, books, {"BRANCH_NAME": "main"})
        self.assertEqual(list(outside.iterdir()), [])
        (destination / "example").unlink()
        (site / "escape").symlink_to(outside, target_is_directory=True)
        with self.assertRaises(ValueError):
            manuals.publish(site, destination, books, {"BRANCH_NAME": "main"})
        self.assertEqual(list(destination.iterdir()), [])

    def test_canvas_manifest_stages_real_assets_and_every_midra_starter(self):
        books = manuals.books()
        self.assertEqual({book["slug"] for book in books},
                         {"canvas-pilot"})
        for book in books:
            manuals.stage_book(book, self.root / book["slug"])
        assets = ROOT / "docs/templates/canvas-pilot"
        for original in assets.iterdir():
            if original.suffix == ".md":
                continue
            copied = self.root / "canvas-pilot/src/docs/templates/canvas-pilot" / original.name
            self.assertEqual(original.read_bytes(), copied.read_bytes())

    def test_failed_build_does_not_leave_a_publishable_partial_or_old_site(self):
        self.book()
        self.write("docs/manuals/mdbook-version", "0.5.4\n")
        self.write("build/manuals/site/manuals/index.html", "old")
        tool = self.write("fake-mdbook", '#!/bin/sh\n'
                          'if [ "$1" = "--version" ]; then echo "mdbook v0.5.4"; else exit 7; fi\n')
        tool.chmod(0o755)
        output = self.root / "build/manuals"
        with self.assertRaises(subprocess.CalledProcessError):
            manuals.build(str(tool), self.root, output)
        self.assertFalse(output.exists())
        self.assertEqual(list(output.parent.iterdir()), [])

    def test_rsync_failure_keeps_previous_landing_page(self):
        site, books = self.site()
        destination = self.root / "manuals"
        landing = self.write("manuals/index.html", "old landing")
        with patch.object(manuals.subprocess, "run",
                          side_effect=subprocess.CalledProcessError(23, "rsync")):
            with self.assertRaises(subprocess.CalledProcessError):
                manuals.publish(site, destination, books, {"BRANCH_NAME": "main"})
        self.assertEqual(landing.read_text(), "old landing")


if __name__ == "__main__":
    unittest.main()
