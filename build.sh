#!/usr/bin/env bash
# Build the native library, stage it into the HAP, package, install, launch.
#
# The staging copy is the point. Running hvigor on its own packages whatever
# .so is already sitting in deveco/entry/libs/, so a Rust change can appear to
# build fine and then not be in the app — which cost an afternoon of debugging
# a "JS TypeError" that was really a napi function missing from a stale
# library.
set -euo pipefail

cd "$(dirname "$0")"
export SPLASH_FRONTEND_DIR="${SPLASH_FRONTEND_DIR:-}"
source ~/ohos-sdk/env-deveco.sh >/dev/null 2>&1
export OHOS_SDK_NATIVE="$OHOS_BASE_SDK_HOME/21/native"
export DEVECO_SDK_HOME="$DEVECO_HOME/sdk"
export NODE_HOME="$DEVECO_HOME/tools/node"
export PATH="$NODE_HOME/bin:$HOME/ohos-sdk/ohos-base-deveco/21/toolchains:$PATH"

DEVICE="${DEVICE:-5ZGYD25B13020968}"

echo "==> cargo"
cargo build --target aarch64-unknown-linux-ohos --release 2>&1 | grep -E "^(error|warning: unused var)" -A4 || true
if ! cargo build --target aarch64-unknown-linux-ohos --release 2>&1 | grep -q "Finished\|Compiling\|error"; then :; fi
cargo build --target aarch64-unknown-linux-ohos --release >/dev/null 2>&1 || {
  echo "cargo failed"; cargo build --target aarch64-unknown-linux-ohos --release 2>&1 | tail -30; exit 1;
}

echo "==> stage .so"
cp target/aarch64-unknown-linux-ohos/release/libsplash_oh.so deveco/entry/libs/arm64-v8a/

echo "==> hvigor"
( cd deveco && node "$DEVECO_HOME/tools/hvigor/bin/hvigorw.js" \
    assembleHap --mode module -p product=default -p buildMode=release --no-daemon 2>&1 \
    | grep -E "BUILD SUCCESSFUL|BUILD FAILED|Error Message" ) || { echo "hvigor failed"; exit 1; }

if [ "${1:-}" = "--build-only" ]; then
  exit 0
fi

echo "==> install"
HAP=deveco/entry/build/default/outputs/default/splash_oh-default-signed.hap
hdc -t "$DEVICE" file send "$HAP" /data/local/tmp/s.hap >/dev/null 2>&1
hdc -t "$DEVICE" shell bm install -p /data/local/tmp/s.hap 2>&1 | tail -1

if [ "${1:-}" = "--no-launch" ]; then
  exit 0
fi

echo "==> launch"
hdc -t "$DEVICE" shell aa force-stop com.example.myapplication >/dev/null 2>&1 || true
hdc -t "$DEVICE" shell hilog -r >/dev/null 2>&1 || true
hdc -t "$DEVICE" shell power-shell wakeup >/dev/null 2>&1 || true
sleep 1
hdc -t "$DEVICE" shell uinput -T -m 660 2400 660 900 300 >/dev/null 2>&1 || true
sleep 2
hdc -t "$DEVICE" shell aa start -a EntryAbility -b com.example.myapplication >/dev/null 2>&1
echo "launched"
