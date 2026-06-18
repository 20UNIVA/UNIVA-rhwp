#!/usr/bin/env bash
# rhwp WASM 빌드 — 호스트 wasm-pack 으로 pkg/ 에 산출.
# rcode/rdocx 와 같은 패턴 — Docker daemon 의존 부재.
#
# Docker 흐름 fallback 이 필요하면 (호스트 wasm-pack 부재 환경):
#   docker compose --env-file .env.docker run --rm wasm
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "wasm-pack 미설치. 다음 중 하나로 설치:" >&2
    echo "  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh" >&2
    echo "  cargo install wasm-pack" >&2
    exit 1
fi

cd "$REPO_ROOT"
echo "[wasm] host wasm-pack ($(wasm-pack --version))"
wasm-pack build --target web

echo "WASM 산출: $REPO_ROOT/pkg/"
ls -1 "$REPO_ROOT/pkg/" 2>/dev/null | head -10
