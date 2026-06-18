#!/usr/bin/env bash
# 배포 패키지 내에서 rhwp-server 실행.
#
# 사용:
#   ./run.sh           # .env 로드 (기본)
#   ./run.sh dev       # .env.dev 로드 (없으면 .env 폴백)
#   ./run.sh testing   # .env.testing (CI·QA)
#   ./run.sh staging   # .env.staging
#   ./run.sh prod      # .env.prod
#
# .env 파일은 gitignore — `.env.<name>.example` 을 복사·편집해 사용.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
cd "$HERE"

ENV_NAME="${1:-}"
ENV_FILE="$HERE/.env"
if [ -n "$ENV_NAME" ] && [ -f "$HERE/.env.$ENV_NAME" ]; then
    ENV_FILE="$HERE/.env.$ENV_NAME"
elif [ -n "$ENV_NAME" ]; then
    echo "warning: .env.$ENV_NAME 없음 — .env 로 폴백" >&2
fi

if [ -f "$ENV_FILE" ]; then
    echo "loading env: $ENV_FILE"
    set -a
    # shellcheck disable=SC1091
    . "$ENV_FILE"
    set +a
fi

# 패키지 내 studio 기본 경로 (env에서 미지정 시).
export RHWP_STUDIO_DIR="${RHWP_STUDIO_DIR:-$HERE/studio}"

echo "rhwp-server 시작 — ENV=${ENV_NAME:-default} ADDR=${RHWP_SERVER_ADDR:-0.0.0.0:7710} STUDIO=$RHWP_STUDIO_DIR"
echo "  모든 경로는 /hwp prefix — 헬스체크: curl http://localhost:7710/hwp/health"
exec "$HERE/rhwp-server"
