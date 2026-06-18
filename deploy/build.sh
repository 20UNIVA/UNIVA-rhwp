#!/usr/bin/env bash
# rhwp SSR 배포 빌드 — WASM → studio dist → server release 순서.
# 요구사항: 호스트 wasm-pack (cargo install wasm-pack), Node >= 20.19 (vite), Rust toolchain.
# rcode·rdocx 와 같은 패턴 — Docker 의존 제거. Docker 흐름이 필요하면
# scripts/build-wasm.sh 의 코멘트 참고.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> [1/3] WASM 빌드 → pkg/"
./scripts/build-wasm.sh

echo "==> [2/3] studio 정적 자산 빌드 (vite)"
node -v
cd rhwp-studio
[ -d node_modules ] || npm ci
npm run build
cd "$ROOT"

echo "==> [3/3] rhwp-server 릴리즈 빌드"
cd rhwp-server
cargo build --release
cd "$ROOT"

echo "완료:"
echo "  - rhwp-server/target/release/rhwp-server"
echo "  - rhwp-studio/dist/"
echo "다음: deploy/package.sh 로 배포 패키지 생성"
