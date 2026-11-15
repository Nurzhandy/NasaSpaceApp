#!/usr/bin/env bash
# Release build + ready-to-play archives (used by .github/workflows/release.yml).
#   bash crates/game/tools/package.sh
# Output in dist/ (override with DIST=...):
#   macOS   → SPACE-EXPLORERS-macos.zip      ("SPACE EXPLORERS.app"; MAC_UNIVERSAL=1 builds Intel + Apple Silicon)
#   Windows → SPACE-EXPLORERS-windows.zip    (folder with "SPACE EXPLORERS.exe" + assets; run from Git Bash)
#   Linux   → SPACE-EXPLORERS-linux.tar.gz   (folder with ./space-explorers + assets)
# Windows needs Visual Studio Build Tools "Desktop development with C++".
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"
DIST="${DIST:-$ROOT/dist}"
NAME="SPACE EXPLORERS"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' crates/game/Cargo.toml | head -1)"
rm -rf "$DIST" && mkdir -p "$DIST"

case "$(uname -s)" in
  Darwin)
    if [ "${MAC_UNIVERSAL:-0}" = "1" ]; then
      cargo build --release -p game --target aarch64-apple-darwin
      cargo build --release -p game --target x86_64-apple-darwin
      BIN="$DIST/game-universal"
      lipo -create -output "$BIN" target/aarch64-apple-darwin/release/game target/x86_64-apple-darwin/release/game
    else
      cargo build --release -p game
      BIN="target/release/game"
    fi
    APP="$DIST/$NAME.app"
    mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
    cp "$BIN" "$APP/Contents/MacOS/game"
    rm -f "$DIST/game-universal"
    cp -R assets "$APP/Contents/Resources/assets"
    # app icon from the generated pixel art
    ICONSET="$DIST/icon.iconset"; mkdir -p "$ICONSET"
    for s in 16 32 64 128 256 512; do
      sips -z $s $s assets/sprites/app_icon.png --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
      sips -z $((s*2)) $((s*2)) assets/sprites/app_icon.png --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
    done
    iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/icon.icns" && rm -rf "$ICONSET"
    cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>$NAME</string>
  <key>CFBundleDisplayName</key><string>$NAME</string>
  <key>CFBundleIdentifier</key><string>org.spaceapps.space-explorers</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleExecutable</key><string>game</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
    # ad-hoc signature: without it macOS calls a downloaded app "damaged"
    codesign --force --deep --sign - "$APP"
    (cd "$DIST" && ditto -c -k --norsrc --noextattr --noqtn --noacl --keepParent "$NAME.app" SPACE-EXPLORERS-macos.zip)
    echo "Done: $DIST/SPACE-EXPLORERS-macos.zip"
    ;;
  Linux)
    cargo build --release -p game
    OUT="$DIST/SPACE-EXPLORERS"
    mkdir -p "$OUT" && cp target/release/game "$OUT/space-explorers" && cp -R assets "$OUT/"
    (cd "$DIST" && tar czf SPACE-EXPLORERS-linux.tar.gz SPACE-EXPLORERS)
    echo "Done: $DIST/SPACE-EXPLORERS-linux.tar.gz"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    cargo build --release -p game
    OUT="$DIST/$NAME"
    mkdir -p "$OUT" && cp target/release/game.exe "$OUT/$NAME.exe" && cp -R assets "$OUT/"
    (cd "$DIST" && powershell -NoProfile -Command "Compress-Archive -Path '$NAME' -DestinationPath 'SPACE-EXPLORERS-windows.zip'")
    echo "Done: $DIST/SPACE-EXPLORERS-windows.zip"
    ;;
  *)
    echo "Unsupported OS: $(uname -s)" >&2; exit 1
    ;;
esac
