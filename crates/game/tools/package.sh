#!/usr/bin/env bash
# B9 — release build + archives for judges.
#   bash crates/game/tools/package.sh
# macOS: dist/JuniorAstronaut.app (+ zip). Linux/Windows: run the same script on that OS
# (on Windows use Git Bash; needs Visual Studio Build Tools "Desktop development with C++").
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"
cargo build --release -p game
DIST="$ROOT/dist"
rm -rf "$DIST" && mkdir -p "$DIST"
case "$(uname -s)" in
  Darwin)
    APP="$DIST/JuniorAstronaut.app"
    mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
    cp target/release/game "$APP/Contents/MacOS/game"
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
  <key>CFBundleName</key><string>Junior Astronaut</string>
  <key>CFBundleDisplayName</key><string>Junior Astronaut: Outpost</string>
  <key>CFBundleIdentifier</key><string>org.spaceapps.junior-astronaut</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleExecutable</key><string>game</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
    (cd "$DIST" && zip -qry JuniorAstronaut-macos.zip JuniorAstronaut.app)
    echo "Done: $APP and dist/JuniorAstronaut-macos.zip"
    ;;
  Linux)
    mkdir -p "$DIST/JuniorAstronaut-linux" && cp target/release/game "$DIST/JuniorAstronaut-linux/" && cp -R assets "$DIST/JuniorAstronaut-linux/"
    (cd "$DIST" && tar czf JuniorAstronaut-linux.tar.gz JuniorAstronaut-linux) && echo "Done: dist/JuniorAstronaut-linux.tar.gz"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    mkdir -p "$DIST/JuniorAstronaut-windows" && cp target/release/game.exe "$DIST/JuniorAstronaut-windows/" && cp -R assets "$DIST/JuniorAstronaut-windows/"
    (cd "$DIST" && powershell -Command "Compress-Archive -Path JuniorAstronaut-windows -DestinationPath JuniorAstronaut-windows.zip") && echo "Done: dist/JuniorAstronaut-windows.zip"
    ;;
esac
