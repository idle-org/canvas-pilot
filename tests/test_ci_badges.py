import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("badge_ci", ROOT / "scripts/ci.py")
ci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci)


class BadgeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.cwd = Path.cwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, self.cwd)
        Path("coverage").mkdir()

    def render(self, kind, *args):
        return subprocess.run([str(ROOT / "scripts/ci-coverage-badges.sh"), kind, *args],
                              capture_output=True, text=True)

    def test_badge_values_colors_sizes_and_xml_escaping(self):
        cases = [("version", "1.2.3", "#007ec6"), ("version", "1.2.3-very-long-preview.100", "#007ec6"),
                 ("test", "passing", "#4c1"), ("test", "failing", "#e05d44"),
                 ("test", "pending", "#9f9f9f"), ("coverage", "-", "#9f9f9f"),
                 ("coverage", "30", "#e05d44"), ("coverage", "75.4", "#dfb317"),
                 ("coverage", "99", "#4c1")]
        for kind, value, color in cases:
            with self.subTest(kind=kind, value=value):
                self.assertEqual(self.render(kind, "--value", value).returncode, 0)
                path = Path(f"coverage/{kind}.svg")
                ET.parse(path)
                self.assertIn(color, path.read_text())
        result = self.render("version", "--value", 'a<&>"', "--text", "build & release",
                             "--size", "12", "--height", "30", "--label-width", "160",
                             "--value-width", "120", "--color", "#123abc", "--suffix", "!")
        self.assertEqual(result.returncode, 0)
        doc = ET.parse("coverage/version.svg").getroot()
        self.assertEqual(doc.attrib["width"], "280")
        self.assertIn('a<&>"!', "".join(doc.itertext()))
        for kind in ("version", "test", "coverage"):
            self.assertEqual(self.render(kind).returncode, 0)
            ET.parse(f"coverage/{kind}.svg")

    def test_invalid_renderer_input_preserves_output(self):
        self.assertEqual(self.render("coverage", "--value", "80").returncode, 0)
        before = Path("coverage/coverage.svg").read_bytes()
        for args in [("--value", "101"), ("--value", "-1"), ("--value", "NaN"),
                     ("--height", "2"), ("--size", "99"), ("--color", "red"),
                     ("--text", "a\nb"), ("--label-width", "0"), ("--unknown", "x")]:
            self.assertNotEqual(self.render("coverage", *args).returncode, 0)
            self.assertEqual(Path("coverage/coverage.svg").read_bytes(), before)
        self.assertNotEqual(self.render("nonsense").returncode, 0)

    def report(self, covered):
        path = Path("coverage/summary.json")
        path.write_text(json.dumps({"data": [{"totals": {"lines": {
            "count": 10000, "covered": covered, "percent": covered / 100}}}]}))
        return path

    def test_gate_threshold_tool_failure_and_cleanup_keep_measurement(self):
        for covered, tool, expected in [(9000, 0, 0), (8000, 0, 0), (7999, 0, 1), (8500, 7, 7)]:
            with self.subTest(covered=covered, tool=tool), patch.dict(os.environ, {"BUILD_NUMBER": "42"}):
                output = io.StringIO()
                with contextlib.redirect_stdout(output):
                    result = ci.measure(self.report(covered), 80, tool)
                self.assertEqual(result, expected)
                text = output.getvalue()
                self.assertLess(text.index("Line coverage:"), text.index("Coverage gate:"))
                self.assertIn("required minimum: 80.00%", text)
                self.assertTrue(Path("coverage/.coverage-ready-42").is_file())
                measured = Path("coverage/coverage.svg").read_bytes()
                with contextlib.redirect_stdout(io.StringIO()):
                    ci.finalize()
                self.assertEqual(Path("coverage/coverage.svg").read_bytes(), measured)
                public = self.root / "public"
                public.mkdir(exist_ok=True)
                self.assertEqual(self.publish(public, "version", "test", "coverage").returncode, 0)
                self.assertEqual((public / "idle-org/canvas-pilot/main/coverage.svg").read_bytes(), measured)
                self.assertFalse(Path("coverage/.coverage-ready-42").exists())
                self.assertIn("failing", Path("coverage/test.svg").read_text())

    def test_coverage_from_fresh_checkout_reports_tool_failure_without_traceback(self):
        Path("coverage").rmdir()

        def unavailable(command):
            self.assertTrue(Path("coverage").is_dir())
            raise OSError("cargo unavailable")

        with patch.object(ci.sys, "argv", ["ci.py", "coverage"]), \
                patch.object(ci.subprocess, "run", side_effect=unavailable), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(ci.main(), 1)
        self.assertIn("unavailable", Path("coverage/measurement.txt").read_text())
        self.assertFalse(Path("coverage/summary.json").exists())

    def test_invalid_missing_and_stale_reports_are_neutral(self):
        with patch.dict(os.environ, {"BUILD_NUMBER": "43"}):
            for content in ("", "{}", '{"data":[]}', '{"data":[{"totals":{"lines":{"count":0,"covered":0,"percent":0}}}]}'):
                path = Path("coverage/summary.json")
                path.write_text(content)
                with contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(ci.measure(path, 80, 0), 1)
                    ci.finalize()
                self.assertIn("-%", Path("coverage/coverage.svg").read_text())
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(ci.measure(Path("missing.json"), 80, 1), 1)
            Path("coverage/.coverage-ready-42").touch()
            Path("coverage/.tests-passed-43").touch()
            with contextlib.redirect_stdout(io.StringIO()):
                ci.finalize()
            self.assertIn("passing", Path("coverage/test.svg").read_text())
            self.assertIn("-%", Path("coverage/coverage.svg").read_text())

    def publish(self, root, *names):
        return subprocess.run([str(ROOT / "scripts/publish-badges.sh"), str(root), *names],
                              capture_output=True, text=True, umask=0o077)

    def test_publication_modes_atomic_replace_and_legacy_symlink(self):
        for kind in ("version", "test", "coverage"):
            self.assertEqual(self.render(kind).returncode, 0)
        public = self.root / "public"
        public.mkdir()
        self.assertEqual(self.publish(public, "version", "test", "coverage").returncode, 0)
        destination = public / "idle-org/canvas-pilot/main"
        for directory in (public / "idle-org", destination.parent, destination):
            self.assertEqual(stat.S_IMODE(directory.stat().st_mode), 0o755)
        for path in destination.iterdir():
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o644)
        self.assertEqual(self.publish(public, "version", "test", "coverage").returncode, 0)
        old = self.root / "old"
        destination.rename(old)
        destination.symlink_to(old, target_is_directory=True)
        self.assertEqual(self.publish(public, "coverage").returncode, 0)
        self.assertFalse(destination.is_symlink())
        self.assertTrue((old / "coverage.svg").is_file())
        for args in [("other",), ("test", "test")]:
            self.assertNotEqual(self.publish(public, *args).returncode, 0)
        self.assertNotEqual(self.publish("relative", "test").returncode, 0)
        Path("coverage/test.svg").unlink()
        Path("coverage/test.svg").symlink_to(old / "test.svg")
        self.assertNotEqual(self.publish(public, "test").returncode, 0)
        other = self.root / "other"
        other.mkdir()
        (other / "idle-org").symlink_to(public / "idle-org", target_is_directory=True)
        self.assertNotEqual(self.publish(other, "coverage").returncode, 0)

    def test_readme_urls_and_exact_archive_selection(self):
        text = (ROOT / "README.md").read_text()
        for kind in ("version", "test", "coverage"):
            self.assertEqual(text.count(f"https://assets.dandaedre.com/coverage/idle-org/canvas-pilot/main/{kind}.svg"), 1)
        self.assertIn("artifacts: 'coverage/version.svg,coverage/test.svg,coverage/coverage.svg'",
                      (ROOT / "Jenkinsfile").read_text())

    def test_workspace_version_and_fresh_build_lifecycle(self):
        with patch.dict(os.environ, {"BUILD_NUMBER": "44"}):
            Path("coverage/stale.svg").write_text("old")
            Path("coverage/.tests-passed-43").touch()
            with patch("sys.argv", ["ci.py", "init"]):
                ci.main()
            self.assertEqual(list(Path("coverage").iterdir()), [])
            with contextlib.redirect_stdout(io.StringIO()):
                ci.finalize()
            version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
            self.assertIn(version, Path("coverage/version.svg").read_text())
            self.assertIn("failing", Path("coverage/test.svg").read_text())
            self.assertIn("-%", Path("coverage/coverage.svg").read_text())
            self.assertEqual({p.name for p in Path("coverage").iterdir()},
                             {"version.svg", "test.svg", "coverage.svg"})
