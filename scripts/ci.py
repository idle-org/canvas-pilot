#!/usr/bin/env python3
"""Current-build coverage gate and badge lifecycle. Python 3.11+ on CI."""
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def build_number():
    number = os.environ.get("BUILD_NUMBER", "local")
    if not number or not all(c.isalnum() or c in "-_" for c in number):
        raise ValueError("Invalid BUILD_NUMBER")
    return number


def render(kind, value):
    subprocess.run([str(ROOT / "scripts/ci-coverage-badges.sh"), kind, "--value", str(value)], check=True)


def config():
    return tomllib.loads((ROOT / "coverage.toml").read_text())


def measure(report, floor, tool_status):
    """Process an independently generated summary before deciding pass/fail."""
    marker = Path(f"coverage/.coverage-ready-{build_number()}")
    marker.unlink(missing_ok=True)
    try:
        data = json.loads(Path(report).read_text())
        records = data["data"]
        if len(records) != 1:
            raise ValueError("expected one complete coverage record")
        lines = records[0]["totals"]["lines"]
        count, covered, percent = lines["count"], lines["covered"], lines["percent"]
        if (type(count) is not int or type(covered) is not int or count <= 0
                or not 0 <= covered <= count or not isinstance(percent, (float, int))
                or not math.isfinite(percent) or not 0 <= percent <= 100
                or abs(percent - 100 * covered / count) > .02):
            raise ValueError("invalid line totals")
        actual = 100 * covered / count
        message = f"Line coverage: {actual:.2f}%; required minimum: {floor:.2f}%"
        print(message, flush=True)
        render("coverage", actual)
        marker.touch()
        Path("coverage/measurement.txt").write_text(f"{build_number()}\n{message}\n")
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        message = f"Line coverage: unavailable; required minimum: {floor:.2f}%; reason: {error}"
        print(message, flush=True)
        Path("coverage/measurement.txt").write_text(f"{build_number()}\n{message}\n")
        print(f"Coverage gate: FAIL (measurement unavailable; tool exit status {tool_status})", flush=True)
        return 1
    if tool_status:
        print(f"Coverage gate: FAIL (coverage tool/test exit status {tool_status})", flush=True)
        return tool_status
    if actual < floor:
        print("Coverage gate: FAIL (below required minimum)", flush=True)
        return 1
    print("Coverage gate: PASS", flush=True)
    return 0


def finalize():
    Path("coverage").mkdir(exist_ok=True)
    build = build_number()
    passed = Path(f"coverage/.tests-passed-{build}")
    ready = Path(f"coverage/.coverage-ready-{build}")
    render("test", "passing" if passed.is_file() else "failing")
    if not ready.is_file():
        render("coverage", "-")
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    render("version", version)
    summary = Path("coverage/measurement.txt")
    lines = summary.read_text().splitlines() if summary.is_file() else []
    if lines and lines[0] == build:
        print("\n".join(lines[1:]))
    else:
        print(f"Line coverage: unavailable; required minimum: {config()['line-floor']:.2f}%; reason: coverage stage did not complete")
    for path in (passed, ready, Path("coverage/summary.json"), summary):
        path.unlink(missing_ok=True)
    for name in ("version", "test", "coverage"):
        path = Path(f"coverage/{name}.svg")
        if path.is_symlink() or not path.is_file() or not path.stat().st_size:
            raise ValueError(f"Invalid badge: {path}")
    if {p.name for p in Path("coverage").glob("*.svg")} != {"version.svg", "test.svg", "coverage.svg"}:
        raise ValueError("Expected exactly three badge SVGs")


def main():
    action = sys.argv[1]
    if action == "init":
        path = Path("coverage")
        if path.is_symlink():
            raise ValueError("Refusing coverage directory symlink")
        if path.exists():
            shutil.rmtree(path)
        path.mkdir()
    elif action == "tests-passed":
        Path(f"coverage/.tests-passed-{build_number()}").touch()
    elif action == "coverage":
        directory = Path("coverage")
        if directory.is_symlink():
            raise ValueError("Refusing coverage directory symlink")
        directory.mkdir(exist_ok=True)
        summary = Path("coverage/summary.json")
        summary.unlink(missing_ok=True)
        status = 1
        try:
            status = subprocess.run(["cargo", "coverage"]).returncode
        except OSError as error:
            print(f"Coverage tool could not start: {error}", flush=True)
        print(f"Coverage tool exit status: {status}", flush=True)
        try:
            return measure(summary, config()["line-floor"], status)
        finally:
            summary.unlink(missing_ok=True)
    elif action == "finalize":
        finalize()
    else:
        raise ValueError(f"Unknown action: {action}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
