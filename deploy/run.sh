#!/usr/bin/env bash
# 배포 패키지 내에서 rhwp-server 실행. 같은 폴더의 .env 를 로드한다.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
cd "$HERE"

if [ -f "$HERE/.env" ]; then
  set -a
  # shellcheck disable=SC1091
  . "$HERE/.env"
  set +a
fi

# 패키지 내 studio 기본 경로 (env에서 미지정 시)
export RHWP_STUDIO_DIR="${RHWP_STUDIO_DIR:-$HERE/studio}"

echo "rhwp-server 시작 — ADDR=${RHWP_SERVER_ADDR:-0.0.0.0:7710} STUDIO_DIR=$RHWP_STUDIO_DIR"
echo "  모든 경로는 /hwp prefix — 헬스체크: curl http://localhost:7710/hwp/health"
exec "$HERE/rhwp-server"
