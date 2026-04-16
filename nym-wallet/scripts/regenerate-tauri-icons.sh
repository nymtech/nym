#!/usr/bin/env bash
# Regenerate macOS/Windows icon bundles from the 1024x1024 master in src-tauri/icons/.
# Master file: app-icon-source.png (padded per Apple-style safe zone). Edit that asset, then run:
#   ./scripts/regenerate-tauri-icons.sh
# Requires: python3 with Pillow (`pip install pillow`) for tray_icon.png resize.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/src-tauri/icons/app-icon-source.png"
yarn --cwd "$ROOT" tauri icon "$SRC" -o "$ROOT/src-tauri/icons"
rm -rf "$ROOT/src-tauri/icons/android" "$ROOT/src-tauri/icons/ios"
rm -f "$ROOT/src-tauri/icons"/Square*.png "$ROOT/src-tauri/icons/StoreLogo.png"
python3 - <<PY
from PIL import Image
from pathlib import Path
icons = Path("$ROOT/src-tauri/icons")
src = Image.open(icons / "app-icon-source.png").convert("RGBA")
src.resize((128, 128), Image.Resampling.LANCZOS).save(icons / "tray_icon.png")
PY
