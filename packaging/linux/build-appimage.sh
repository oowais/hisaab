#!/usr/bin/env bash
# Packs the release binary (`vp run tauri build`, target/release/hisaab) into an
# AppImage, for AppImage managers such as the CachyOS Package Installer (Add from GitHub,
# Check for updates). The binary carries the UI; WebKitGTK, GTK and the tray
# library come from the system (Arch: webkit2gtk-4.1, gtk3,
# libayatana-appindicator), as with the .deb. Tauri's own AppImage bundles
# them instead, which is large and breaks on rolling distros' newer Mesa.
# Usage, from the repo root:
#   packaging/linux/build-appimage.sh <path to appimagetool> <version> <output file>
set -euo pipefail

TOOL="$(realpath "$1")"
VERSION="$2"
OUT="$(realpath -m "$3")"
cd "$(dirname "$0")/../.."

BIN=target/release/hisaab
APPDIR=target/AppDir
ID=com.owaisahmed.hisaab

[ -x "$BIN" ] || { echo "No $BIN: run vp run tauri build first" >&2; exit 1; }

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
cp "$BIN" "$APPDIR/usr/bin/hisaab"
# X-AppImage-Version: the version AppImage managers show.
cat > "$APPDIR/$ID.desktop" <<EOF
[Desktop Entry]
Name=Hisaab
Comment=Private, local-first tracker for subscriptions and bills
Exec=hisaab
Icon=$ID
Categories=Office;Finance;
Terminal=false
Type=Application
StartupWMClass=$ID
X-AppImage-Version=$VERSION
EOF
cp src-tauri/icons/icon.png "$APPDIR/$ID.png"
ln -s "$ID.png" "$APPDIR/.DirIcon"
cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/hisaab" "$@"
EOF
chmod +x "$APPDIR/AppRun"

# Extract-and-run: appimagetool is itself an AppImage, and CI runners have no FUSE.
APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 "$TOOL" --no-appstream "$APPDIR" "$OUT"
