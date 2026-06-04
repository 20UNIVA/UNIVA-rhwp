#!/usr/bin/env bash
# 빌드 산출물을 VM 배포 패키지(deploy/rhwp-vm-package/ + .tgz)로 모은다.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/deploy/rhwp-vm-package"

BIN="$ROOT/server/target/release/rhwp-server"
DIST="$ROOT/rhwp-studio/dist"
[ -f "$BIN" ] || { echo "ERROR: $BIN 없음 — deploy/build.sh 먼저 실행"; exit 1; }
[ -d "$DIST" ] || { echo "ERROR: $DIST 없음 — deploy/build.sh 먼저 실행"; exit 1; }

rm -rf "$OUT"
mkdir -p "$OUT"
cp "$BIN" "$OUT/rhwp-server"
cp -r "$DIST" "$OUT/studio"
cp "$ROOT/deploy/.env.example" "$OUT/.env"
cp "$ROOT/deploy/run.sh" "$OUT/run.sh"
chmod +x "$OUT/rhwp-server" "$OUT/run.sh"

tar czf "$ROOT/deploy/rhwp-vm-package.tgz" -C "$ROOT/deploy" rhwp-vm-package

echo "패키지 생성:"
echo "  $OUT/"
echo "  $ROOT/deploy/rhwp-vm-package.tgz  ($(du -h "$ROOT/deploy/rhwp-vm-package.tgz" | cut -f1))"
echo "VM에서: tar xzf rhwp-vm-package.tgz && cd rhwp-vm-package && vi .env && ./run.sh"
