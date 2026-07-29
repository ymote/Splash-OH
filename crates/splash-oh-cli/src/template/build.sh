#!/usr/bin/env bash
# Build this app and install it on a connected phone.
#
# The app shell -- the ArkTS entry point, the DevEco project, the Rust bridge --
# lives in a Splash-OH checkout, not in here. This project is the frontend and
# your own native code; the shell is what turns them into a HAP.
#
# Point SPLASH_OH at that checkout, or keep one beside this directory.
set -euo pipefail
cd "$(dirname "$0")"
APP_DIR="$PWD"

SPLASH_OH="${SPLASH_OH:-$APP_DIR/../Splash-OH}"
if [ ! -x "$SPLASH_OH/build.sh" ]; then
  cat >&2 <<MSG
This needs a Splash-OH checkout to build against, and there is none at:
  $SPLASH_OH

Either clone one beside this project:
  git clone https://github.com/ymote/Splash-OH.git "$APP_DIR/../Splash-OH"

or point at an existing one:
  SPLASH_OH=/path/to/Splash-OH ./build.sh
MSG
  exit 1
fi

if [ ! -d dist ]; then
  echo "no dist/ yet — run: npm install && npm run build" >&2
  exit 1
fi

# SPLASH_DEV_SERVER passes straight through, so a live-reload build is
# `SPLASH_DEV_SERVER=http://127.0.0.1:5173 ./build.sh` from here too.
exec env SPLASH_FRONTEND_DIR="$APP_DIR/dist" "$SPLASH_OH/build.sh" "$@"
