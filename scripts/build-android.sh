#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VERSION="$(python3 -c "import json; print(json.load(open('src-tauri/tauri.conf.json'))['version'])")"
echo "Building Gym Manager ${VERSION} for Android"

# Generate a clean Android project from the current Tauri config.
rm -rf src-tauri/gen/android
bun tauri android init --ci

MAIN_DIR="src-tauri/gen/android/app/src/main"
JAVA_DIR="${MAIN_DIR}/java/com/gym_manager/dc"
OVERRIDES="src-tauri/android-overrides"

mkdir -p "$JAVA_DIR" "${MAIN_DIR}/res/xml"
cp "$OVERRIDES/MainActivity.kt" "$JAVA_DIR/MainActivity.kt"
cp "$OVERRIDES/PdfPlugin.kt" "$JAVA_DIR/PdfPlugin.kt"
cp "$OVERRIDES/AndroidManifest.xml" "$MAIN_DIR/AndroidManifest.xml"
cp "$OVERRIDES/file_paths.xml" "${MAIN_DIR}/res/xml/file_paths.xml"

python3 scripts/patch-android-gradle.py

# This also runs the frontend build and embeds the frontend into libgym_manager_lib.so.
bun tauri android build --apk --target aarch64

APK="$(find src-tauri/gen/android/app/build/outputs/apk -name '*release*.apk' ! -name '*signed*' | head -n 1)"
if [ -z "$APK" ]; then
  APK="$(find src-tauri/gen/android/app/build/outputs/apk -name '*release*.apk' | head -n 1)"
fi
if [ -z "$APK" ]; then
  echo "No release APK found" >&2
  exit 1
fi

mkdir -p artifacts
KEYSTORE="${ANDROID_KEYSTORE_PATH:-$ROOT_DIR/android-ci.keystore}"

if [ -n "${ANDROID_KEYSTORE_BASE64:-}" ]; then
  printf '%s' "$ANDROID_KEYSTORE_BASE64" | base64 -d > "$KEYSTORE"
  KS_PASS="${ANDROID_KEYSTORE_PASSWORD:?ANDROID_KEYSTORE_PASSWORD is required}"
  KEY_ALIAS="${ANDROID_KEY_ALIAS:?ANDROID_KEY_ALIAS is required}"
  KEY_PASS="${ANDROID_KEY_PASSWORD:?ANDROID_KEY_PASSWORD is required}"
else
  if [ ! -f "$KEYSTORE" ]; then
    keytool -genkeypair -v \
      -keystore "$KEYSTORE" \
      -alias androiddebugkey \
      -keyalg RSA \
      -keysize 2048 \
      -validity 10000 \
      -storepass android \
      -keypass android \
      -dname "CN=Android Debug,O=Android,C=US"
  fi
  KS_PASS=android
  KEY_ALIAS=androiddebugkey
  KEY_PASS=android
fi

APKSIGNER="$(find "${ANDROID_HOME:?ANDROID_HOME is required}/build-tools" -name apksigner | sort -V | tail -n 1)"
if [ -z "$APKSIGNER" ]; then
  echo "apksigner not found" >&2
  exit 1
fi

OUT="artifacts/GymManager-android-arm64.apk"
"$APKSIGNER" sign \
  --ks "$KEYSTORE" \
  --ks-key-alias "$KEY_ALIAS" \
  --ks-pass "pass:${KS_PASS}" \
  --key-pass "pass:${KEY_PASS}" \
  --out "$OUT" \
  "$APK"

"$APKSIGNER" verify --verbose "$OUT"

{
  echo "APP_VERSION=${VERSION}"
  echo "ANDROID_APK=${OUT}"
} >> "${GITHUB_ENV:-/dev/null}"

if [ -n "${GITHUB_OUTPUT:-}" ]; then
  {
    echo "version=${VERSION}"
    echo "apk=${OUT}"
  } >> "$GITHUB_OUTPUT"
fi

echo "APK ready: ${OUT}"
