#!/usr/bin/env python3
"""dic/MANIFEST.jsonが記錄するSHA-256を檢證する。"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path


DATA_DIR = Path(__file__).resolve().parent.parent / "dic"
MANIFEST_PATH = DATA_DIR / "MANIFEST.json"


def main() -> int:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    invalid = False
    for name, expected in manifest["files"].items():
        path = DATA_DIR / name
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != expected:
            print(f"{name}: expected {expected}, got {actual}", file=sys.stderr)
            invalid = True
    return int(invalid)


if __name__ == "__main__":
    raise SystemExit(main())
