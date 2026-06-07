#!/bin/bash
# Sub-2 e2e 서버 가동/정리 스크립트.
# 사용: ./sub2-server.sh start | stop | restart | status

set -e
PIDFILE="/tmp/rhwp-server-sub2.pid"
LOGFILE="/tmp/rhwp-server-sub2.log"
SERVER_DIR="$(cd "$(dirname "$0")/../.." && pwd)/server"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"

case "${1:-}" in
  start)
    if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
      echo "이미 가동 중 (pid=$(cat $PIDFILE))"
      exit 0
    fi
    if [ ! -d "$SERVER_DIR" ]; then
      echo "server 디렉터리 없음: $SERVER_DIR"
      exit 1
    fi
    if [ ! -x "$CARGO" ]; then
      echo "cargo 실행 파일 없음: $CARGO (환경변수 CARGO 로 재지정)"
      exit 1
    fi
    # Canvas 시각 검증 e2e 가 같은 포트 (7710) 에서 studio dist 를 받아야 한다.
    # dist 가 있으면 자동으로 RHWP_STUDIO_DIR 지정 — Puppeteer 가 7710 으로 갈 수 있음.
    SCRIPT_DIR_ABS="$(cd "$(dirname "$0")" && pwd)"
    STUDIO_DIST_DIR="$(cd "$SCRIPT_DIR_ABS/.." && pwd)/dist"
    if [ -d "$STUDIO_DIST_DIR" ] && [ -z "${RHWP_STUDIO_DIR:-}" ]; then
      export RHWP_STUDIO_DIR="$STUDIO_DIST_DIR"
      echo "studio 정적 서빙 활성 — RHWP_STUDIO_DIR=$RHWP_STUDIO_DIR"
    fi
    cd "$SERVER_DIR"
    RUST_LOG="${RUST_LOG:-info}" nohup "$CARGO" run > "$LOGFILE" 2>&1 &
    echo $! > "$PIDFILE"
    echo "서버 시작 pid=$(cat $PIDFILE), log=$LOGFILE"
    # 가동 대기 — /health 200 까지 최대 30초.
    for i in $(seq 1 30); do
      if curl -sf http://127.0.0.1:7710/hwp/health > /dev/null 2>&1; then
        echo "서버 가동 확인 (${i}초 소요)"
        exit 0
      fi
      sleep 1
    done
    echo "서버 가동 timeout — log: $LOGFILE"
    exit 1
    ;;
  stop)
    if [ -f "$PIDFILE" ]; then
      pid="$(cat "$PIDFILE")"
      if kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null || true
        echo "서버 종료 (pid=$pid)"
      else
        echo "PID 파일 있지만 프로세스 없음 (pid=$pid)"
      fi
      rm -f "$PIDFILE"
    else
      echo "가동 중 서버 없음 (PIDFILE 없음)"
    fi
    ;;
  restart)
    "$0" stop || true
    sleep 1
    "$0" start
    ;;
  status)
    if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
      echo "가동 중 (pid=$(cat $PIDFILE))"
      curl -sf http://127.0.0.1:7710/hwp/health > /dev/null 2>&1 \
        && echo "/health OK" \
        || echo "/health 응답 없음"
    else
      echo "정지 상태"
    fi
    ;;
  *)
    echo "사용: $0 start | stop | restart | status"
    exit 1
    ;;
esac
