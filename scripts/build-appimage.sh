#!/usr/bin/env bash
# Package a built Flutter linux bundle as an AppImage.
# Usage: scripts/build-appimage.sh <bundle-dir> <output.AppImage> <x86_64|arm64>
set -euo pipefail

BUNDLE="${1:?usage: build-appimage.sh <bundle-dir> <output> <arch>}"
OUT="${2:?missing output path}"
ARCH="${3:?missing arch}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APPDIR="$(mktemp -d)/self26.AppDir"
trap 'rm -rf "$(dirname "$APPDIR")"' EXIT

mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps"
cp -a "$BUNDLE/." "$APPDIR/usr/bin/"
cp "$REPO_ROOT/packaging/linux/self26.desktop" "$APPDIR/usr/share/applications/gitlab.httpanimations.self26.desktop"
cp "$REPO_ROOT/packaging/linux/self26.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/self26.png"
ln -sf usr/share/icons/hicolor/256x256/apps/self26.png "$APPDIR/self26.png"
ln -sf usr/share/applications/gitlab.httpanimations.self26.desktop "$APPDIR/self26.desktop"
cat > "$APPDIR/AppRun" <<'RUN'
#!/bin/sh
exec "$(dirname "$0")/usr/bin/self26_frontend" "$@"
RUN
chmod +x "$APPDIR/AppRun"

if [ "$ARCH" = "arm64" ]; then
  TOOL_ARCH=aarch64
else
  TOOL_ARCH=x86_64
fi
curl -fsSL --retry 3 -o /tmp/appimagetool \
  "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$TOOL_ARCH.AppImage"
chmod +x /tmp/appimagetool
mkdir -p "$(dirname "$OUT")"
ARCH="$TOOL_ARCH" /tmp/appimagetool "$APPDIR" "$OUT"
