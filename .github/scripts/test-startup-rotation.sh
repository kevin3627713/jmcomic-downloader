#!/usr/bin/env bash
set -euo pipefail

TEST_DIR=$(mktemp -d)
APP="$TEST_DIR/JMStartupRotationTest.app"
SIMULATOR=""
LAUNCH_PID=""
cleanup() {
  if [ -n "$LAUNCH_PID" ]; then
    kill "$LAUNCH_PID" >/dev/null 2>&1 || true
    wait "$LAUNCH_PID" 2>/dev/null || true
  fi
  if [ -n "$SIMULATOR" ]; then
    xcrun simctl shutdown "$SIMULATOR" >/dev/null 2>&1 || true
    xcrun simctl delete "$SIMULATOR" >/dev/null 2>&1 || true
  fi
  rm -rf "$TEST_DIR"
}
trap cleanup EXIT
mkdir -p "$APP"

cat > "$APP/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>com.lanyeeee.startup-rotation-test</string>
<key>CFBundleExecutable</key><string>JMStartupRotationTest</string>
<key>CFBundleName</key><string>JMStartupRotationTest</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
<key>CFBundleShortVersionString</key><string>1.0</string>
<key>MinimumOSVersion</key><string>14.0</string>
<key>UIDeviceFamily</key><array><integer>1</integer></array>
<key>UISupportedInterfaceOrientations</key><array>
<string>UIInterfaceOrientationPortrait</string>
<string>UIInterfaceOrientationLandscapeLeft</string>
<string>UIInterfaceOrientationLandscapeRight</string>
</array>
</dict></plist>
PLIST

xcrun --sdk iphonesimulator swiftc \
  -sdk "$(xcrun --sdk iphonesimulator --show-sdk-path)" \
  -target "$(uname -m)-apple-ios14.0-simulator" -parse-as-library \
  src-tauri/plugins/ios/Sources/StartupRotation.swift \
  scripts/ios-startup-rotation-test.swift \
  -o "$APP/JMStartupRotationTest"
codesign --force --sign - "$APP"

read -r RUNTIME DEVICE_TYPE < <(xcrun simctl list devices available --json | jq -r '
  [.devices | to_entries[] | .key as $runtime | .value[] |
   select(.isAvailable == true and (.name | startswith("iPhone"))) |
   [$runtime, .deviceTypeIdentifier]][0] | @tsv')
if [ -z "$RUNTIME" ] || [ -z "$DEVICE_TYPE" ]; then
  echo "No available iPhone simulator runtime."
  exit 1
fi
SIMULATOR=$(xcrun simctl create "JMStartupRotationTest" "$DEVICE_TYPE" "$RUNTIME")
xcrun simctl boot "$SIMULATOR"
xcrun simctl bootstatus "$SIMULATOR" -b
xcrun simctl install "$SIMULATOR" "$APP"
xcrun simctl launch --console "$SIMULATOR" com.lanyeeee.startup-rotation-test \
  > "$TEST_DIR/process.log" 2>&1 &
LAUNCH_PID=$!
CONTAINER=$(xcrun simctl get_app_container "$SIMULATOR" com.lanyeeee.startup-rotation-test data)
RESULT="$CONTAINER/Documents/startup-rotation-result.json"
for ((attempt = 0; attempt < 60; attempt++)); do
  if [ -f "$RESULT" ]; then break; fi
  sleep 0.5
done
cat "$TEST_DIR/process.log"
if [ ! -f "$RESULT" ]; then
  echo "Simulator test did not produce a result."
  exit 1
fi
cat "$RESULT"
jq -e '.status == "passed" and .modalDismissed and .portrait and .webviewFillsRoot and .orientations == [4, 1]' "$RESULT"
echo "Native startup rotation passed: landscape, portrait, modal cleanup, full WebView, JS viewport, and one-time execution."
