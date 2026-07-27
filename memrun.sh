#!/usr/bin/env bash
# Measure RAM for one app on one implementation, in a fresh process.
#
#   ./memrun.sh <app-index 0..3> <rust|arkts>
#
# One implementation per launch is not optional. Measuring both in a single
# process showed whoever ran second refilling pages the first had freed but the
# allocator retained, which made it look nearly free — and in one ordering,
# negative. Flipping the order was the only way to notice, because either run
# on its own looks like a clean result.
set -euo pipefail
cd "$(dirname "$0")"

APP_IDX="${1:-0}"
WHICH="${2:-rust}"
DEVICE="${DEVICE:-5ZGYD25B13020968}"
export PATH="$HOME/ohos-sdk/ohos-base-deveco/21/toolchains:$PATH"

F=deveco/entry/src/main/ets/pages/Index.ets
sed -i '' "s/^const MEM_MODE: boolean = .*/const MEM_MODE: boolean = true;/" $F
sed -i '' "s/^const MEM_APP: number = .*/const MEM_APP: number = $APP_IDX;/" $F
if [ "$WHICH" = "rust" ]; then
  sed -i '' "s/^const MEM_RUST: boolean = .*/const MEM_RUST: boolean = true;/" $F
else
  sed -i '' "s/^const MEM_RUST: boolean = .*/const MEM_RUST: boolean = false;/" $F
fi

./build.sh --no-launch >/dev/null 2>&1

hdc -t "$DEVICE" shell aa force-stop com.example.myapplication >/dev/null 2>&1 || true
hdc -t "$DEVICE" shell hilog -r >/dev/null 2>&1 || true
hdc -t "$DEVICE" shell power-shell wakeup >/dev/null 2>&1 || true
sleep 1
hdc -t "$DEVICE" shell uinput -T -m 660 2400 660 900 300 >/dev/null 2>&1 || true
sleep 2
hdc -t "$DEVICE" shell aa start -a EntryAbility -b com.example.myapplication >/dev/null 2>&1
# Keep the display awake without touching the app's own surface.
for _ in 1 2 3 4 5; do
  sleep 4
  hdc -t "$DEVICE" shell power-shell wakeup >/dev/null 2>&1 || true
done
hdc -t "$DEVICE" shell hilog -x 2>/dev/null | grep -oE "RAM .{0,70}" || echo "(no output)"
