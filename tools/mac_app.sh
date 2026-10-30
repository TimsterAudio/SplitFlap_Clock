#!/bin/sh
# Makes the Mac app, for Apple silicon and Intel, from the two release
# builds, and zips it with a read-me as <out>/splitflap-clock-mac.zip:
#
#     cargo build --release --target aarch64-apple-darwin
#     cargo build --release --target x86_64-apple-darwin
#     tools/mac_app.sh out
#
# The app is signed "ad hoc", for running on any Mac, which Apple silicon
# needs; it isn't signed by Apple, so the read-me says how to open it.
set -eu
out=${1:-out}
work=$(mktemp -d)
folder="$work/Split-Flap Clock"
app="$folder/Split-Flap Clock.app"
mkdir -p "$app/Contents/MacOS" "$out"
lipo -create -output "$app/Contents/MacOS/splitflap-clock" \
  target/aarch64-apple-darwin/release/splitflap-clock \
  target/x86_64-apple-darwin/release/splitflap-clock
ver=$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Split-Flap Clock</string>
  <key>CFBundleDisplayName</key><string>Split-Flap Clock</string>
  <key>CFBundleIdentifier</key><string>com.timstersaudioworks.splitflapclock</string>
  <key>CFBundleExecutable</key><string>splitflap-clock</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$ver</string>
  <key>CFBundleVersion</key><string>$ver</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
plutil -lint "$app/Contents/Info.plist"
codesign --force --deep --sign - "$app"
cat > "$folder/READ ME FIRST.txt" <<'README'
Split-Flap Clock for Mac

1. Drag "Split-Flap Clock" into Applications.
2. The first time, right-click it and choose Open, then Open again.
   It isn't signed by Apple, so a plain double-click is refused.
   (On newer macOS: try to open it once, then go to System Settings,
   Privacy & Security, and click Open Anyway.)
3. If macOS says it is damaged, open Terminal and run:
     xattr -cr "/Applications/Split-Flap Clock.app"
   then open it again.

Right-click the clock for its menu and Settings. Settings live in
~/Library/Application Support/splitflap-clock. To show it on every
desktop, right-click it in the Dock: Options, Assign To, All Desktops.
README
file "$app/Contents/MacOS/splitflap-clock"
(cd "$work" && ditto -c -k --sequesterRsrc --keepParent "Split-Flap Clock" splitflap-clock-mac.zip)
mv "$work/splitflap-clock-mac.zip" "$out/"
rm -rf "$work"
