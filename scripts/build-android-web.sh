#!/usr/bin/env bash
# Builds the web app for Android (Termux) and writes riverlab-web-<frontend tree hash>.tar.gz.
# Next.js's bundler has no Android build, so the phone downloads this prebuilt bundle (plain
# JavaScript run by Node) instead; publish it on the android-web branch. See docs/android.md.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/dist}"
TREE="$(git -C "$ROOT" rev-parse HEAD:frontend)"
if ! git -C "$ROOT" diff --quiet HEAD -- frontend; then
  echo "frontend/ has uncommitted changes; commit first so the bundle matches its version" >&2
  exit 1
fi
cd "$ROOT/frontend"
rm -rf .next
npm run build
STAGE="$(mktemp -d)"
mkdir -p "$STAGE/riverlab-web/.next"
cp -r .next/standalone/. "$STAGE/riverlab-web/"
cp -r .next/static "$STAGE/riverlab-web/.next/static"
[ -d public ] && cp -r public "$STAGE/riverlab-web/"
# sharp (image optimisation) ships Linux x64 binaries and is unused here.
rm -rf "$STAGE/riverlab-web/node_modules/@img"
mkdir -p "$OUT"
tar czf "$OUT/riverlab-web-$TREE.tar.gz" -C "$STAGE" riverlab-web
rm -rf "$STAGE"
echo "$OUT/riverlab-web-$TREE.tar.gz"
