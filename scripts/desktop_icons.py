"""Canvas Pilot application and diagnostic artwork."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
APPS = {"canvas-pilot": ("Canvas Pilot", "crates/canvas-pilot/assets", "canvas-pilot")}
ARTWORK = {name: folder for name, (_, folder, _) in APPS.items()} | {
    "midra-probe": "tools/assets/midra-probe",
}
