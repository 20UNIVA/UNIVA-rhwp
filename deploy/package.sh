#!/usr/bin/env bash
# 빌드 산출물을 VM 배포 패키지(deploy/rhwp-vm-package/ + .tgz)로 모은다.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/deploy/rhwp-vm-package"

BIN="$ROOT/rhwp-server/target/release/rhwp-server"
DIST="$ROOT/rhwp-studio/dist"
[ -f "$BIN" ] || { echo "ERROR: $BIN 없음 — deploy/build.sh 먼저 실행"; exit 1; }
[ -d "$DIST" ] || { echo "ERROR: $DIST 없음 — deploy/build.sh 먼저 실행"; exit 1; }

rm -rf "$OUT"
mkdir -p "$OUT"
cp "$BIN" "$OUT/rhwp-server"
cp -r "$DIST" "$OUT/studio"
# 환경 .env — .env.example 은 git ignored. 빌드 머신 로컬에 있다면 복사.
if [ -f "$ROOT/deploy/.env.example" ]; then
  cp "$ROOT/deploy/.env.example" "$OUT/.env"
else
  echo "WARNING: deploy/.env.example 없음 — 패키지에 .env 미포함 (VM 에서 직접 .env 작성 필요)"
fi
# 환경별 .env.*.example — 있는 환경만 복사. 모두 있으면 run.sh dev/testing/staging/prod 모두 가능.
# .env.{env} 실 사용 파일도 같이 — bake-app.sh / template_add_app.sh 의 .env.{env} 검증
# 통과용. 빌드 머신에 .env.dev 등이 박혀 있어야 그 환경으로 띄울 수 있다.
for env in dev testing staging prod; do
  if [ -f "$ROOT/deploy/.env.$env.example" ]; then
    cp "$ROOT/deploy/.env.$env.example" "$OUT/.env.$env.example"
  fi
  if [ -f "$ROOT/deploy/.env.$env" ]; then
    cp "$ROOT/deploy/.env.$env" "$OUT/.env.$env"
  fi
done
# systemd 유닛 — 있다면 복사 (rdocx 패턴 정합).
[ -d "$ROOT/deploy/systemd" ] && cp -r "$ROOT/deploy/systemd" "$OUT/systemd"
cp "$ROOT/deploy/run.sh" "$OUT/run.sh"
chmod +x "$OUT/rhwp-server" "$OUT/run.sh"

# .build-info — VM 안에서 이 패키지가 어느 commit·시점에서 만들어졌는지 한 줄로 확인.
# 사용: ssh ubuntu@VM "cat /opt/app/rhwp/.build-info"
{
    echo "app=rhwp"
    echo "git_commit=$(cd "$ROOT" && git rev-parse HEAD 2>/dev/null || echo unknown)"
    echo "git_short=$(cd "$ROOT" && git rev-parse --short HEAD 2>/dev/null || echo unknown)"
    echo "git_branch=$(cd "$ROOT" && git branch --show-current 2>/dev/null || echo unknown)"
    echo "git_dirty=$(cd "$ROOT" && git diff --quiet 2>/dev/null && echo false || echo true)"
    echo "build_time=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "build_host=$(hostname)"
} > "$OUT/.build-info"

tar czf "$ROOT/deploy/rhwp-vm-package.tgz" -C "$ROOT/deploy" rhwp-vm-package

echo "패키지 생성:"
echo "  $OUT/"
echo "  $ROOT/deploy/rhwp-vm-package.tgz  ($(du -h "$ROOT/deploy/rhwp-vm-package.tgz" | cut -f1))"
echo "VM에서: tar xzf rhwp-vm-package.tgz && cd rhwp-vm-package && vi .env && ./run.sh"
