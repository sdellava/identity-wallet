#!/usr/bin/env bash
set -euo pipefail

APP_ID="com.impierce.identity_wallet"
WORKDIR="${WORKDIR:-$HOME/dev/identity-wallet}"
NODE_DIR="${NODE_DIR:-$HOME/.local/node/node-v24.14.0-linux-x64}"
ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$ANDROID_HOME/ndk/26.1.10909125}"
WINDOWS_ANDROID_HOME="${WINDOWS_ANDROID_HOME:-/mnt/c/Users/sdellava/AppData/Local/Android/Sdk}"
APK_NAME="${APK_NAME:-app-universal-debug-fast.apk}"
INSTALL="${INSTALL:-1}"
RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-1.90.0-x86_64-unknown-linux-gnu}"
TAURI_ANDROID_TARGET="${TAURI_ANDROID_TARGET:-aarch64}"

usage() {
  cat <<EOF
Usage: bash scripts/android-build-install.sh [--no-install] [--clean]

Environment overrides:
  WORKDIR               WSL build copy directory. Default: $HOME/dev/identity-wallet
  NODE_DIR              Node.js directory. Default: $HOME/.local/node/node-v24.14.0-linux-x64
  ANDROID_HOME          WSL Android SDK. Default: $HOME/Android/Sdk
  ANDROID_NDK_ROOT      Android NDK path. Default: $HOME/Android/Sdk/ndk/26.1.10909125
  WINDOWS_ANDROID_HOME  Windows Android SDK path mounted in WSL.
  APK_NAME              Copied APK filename in ./debug.
  RUSTUP_TOOLCHAIN      Rust toolchain. Default: 1.90.0-x86_64-unknown-linux-gnu
  TAURI_ANDROID_TARGET  Android ABI target. Default: aarch64
  INSTALL=0             Build only.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-install)
      INSTALL=0
      shift
      ;;
    --clean)
      rm -rf "$WORKDIR"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! -x "$NODE_DIR/bin/node" ]]; then
  echo "Node not found at $NODE_DIR/bin/node" >&2
  exit 1
fi

export PATH="$NODE_DIR/bin:$HOME/.cargo/bin:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$PATH"
export ANDROID_HOME
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_ROOT
export ANDROID_NDK="$ANDROID_NDK_ROOT"
export RUSTUP_TOOLCHAIN
export CMAKE_GENERATOR="${CMAKE_GENERATOR:-Ninja}"
export PUBLIC_DEV_MODE_MENU_EXPANDED="${PUBLIC_DEV_MODE_MENU_EXPANDED:-false}"
export PUBLIC_DEV_SHOW_CURRENT_ROUTE="${PUBLIC_DEV_SHOW_CURRENT_ROUTE:-false}"
export PUBLIC_STYLE_SAFE_AREA_INSETS="${PUBLIC_STYLE_SAFE_AREA_INSETS:-false}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

mkdir -p "$WORKDIR"

echo "==> Syncing source to $WORKDIR"
rsync -a --delete \
  --exclude ".git" \
  --exclude "node_modules" \
  --exclude "target" \
  --exclude "debug" \
  --exclude "unime/src-tauri/gen/android/app/build" \
  --exclude "vendor/tauri-plugin-keystore/android/build" \
  --exclude "vendor/tauri-plugin-keystore/android/.tauri" \
  "$SOURCE_ROOT/" "$WORKDIR/"

cd "$WORKDIR"

if [[ ! -d node_modules || ! -x node_modules/.bin/tauri ]]; then
  echo "==> Installing JS dependencies"
  CI=true corepack pnpm install --frozen-lockfile
fi

echo "==> Building Android debug APK"
corepack pnpm --filter unime tauri android build --debug --target "$TAURI_ANDROID_TARGET" --apk true --aab false

APK="$(find "$WORKDIR/unime/src-tauri/gen/android/app/build/outputs/apk" -type f -name '*debug*.apk' | sort | head -n 1)"
if [[ ! -f "$APK" ]]; then
  echo "APK not found under $WORKDIR/unime/src-tauri/gen/android/app/build/outputs/apk" >&2
  exit 1
fi

mkdir -p "$SOURCE_ROOT/debug"
cp "$APK" "$SOURCE_ROOT/debug/$APK_NAME"
echo "==> APK copied to $SOURCE_ROOT/debug/$APK_NAME"

if [[ "$INSTALL" != "1" ]]; then
  echo "==> Build complete; install skipped"
  exit 0
fi

install_with_adb() {
  local adb_cmd="$1"
  local apk_arg="$2"

  "$adb_cmd" start-server >/dev/null 2>&1 || true
  if "$adb_cmd" devices | awk 'NR > 1 && $2 == "device" { found = 1 } END { exit found ? 0 : 1 }'; then
    "$adb_cmd" install -r -d "$apk_arg"
    "$adb_cmd" shell am force-stop "$APP_ID" >/dev/null 2>&1 || true
    "$adb_cmd" shell monkey -p "$APP_ID" 1 >/dev/null
    return 0
  fi

  return 1
}

echo "==> Installing APK on connected device"
if command -v adb >/dev/null 2>&1 && install_with_adb adb "$SOURCE_ROOT/debug/$APK_NAME"; then
  echo "==> Installed and launched with WSL adb"
  exit 0
fi

ADB_EXE="$WINDOWS_ANDROID_HOME/platform-tools/adb.exe"
if [[ -x "$ADB_EXE" ]]; then
  APK_WIN="$(wslpath -w "$SOURCE_ROOT/debug/$APK_NAME")"
  if install_with_adb "$ADB_EXE" "$APK_WIN"; then
    echo "==> Installed and launched with Windows adb.exe"
    exit 0
  fi
fi

echo "No connected adb device found." >&2
echo "APK is ready at: $SOURCE_ROOT/debug/$APK_NAME" >&2
exit 1
