# Task #999 Stage 11(후속) — VM 배포 최소 패키지

- 브랜치: `feature/ssr`
- 목적: 현 기능 그대로 VM에 띄우기 위한 최소 패키지 + 실행 방법 정리

## 핵심 결정 — single-origin 단일 프로세스

rhwp-server가 **API + studio 정적 자산을 같은 포트에서 서빙**하도록 옵션 추가.
→ 별도 웹서버(nginx)·CORS 불필요. **바이너리 1개 + dist 폴더**가 최소 패키지.

## 구현

### `server/src/main.rs`
- `RHWP_STUDIO_DIR` 환경변수 지정 시 `tower_http::services::ServeDir` 를 `fallback_service` 로 장착(SPA index fallback). 미지정 시 API 전용(기존 동작 유지). (tower-http `fs` feature는 기존부터 활성)

### `deploy/` (신규)
| 파일 | 역할 |
|------|------|
| `README.md` | 빌드·패키징·실행·iframe 연결·MCP 연동·한계 가이드 |
| `.env.example` | `RHWP_SERVER_ADDR`/`DB`/`STUDIO_DIR`/`UPLOAD_URL`/`DOWNLOAD_URL`/`RUST_LOG` |
| `build.sh` | WASM(Docker) → studio dist(vite) → server release 일괄 빌드 |
| `package.sh` | 산출물을 `rhwp-vm-package/`(+ `.tgz`)로 수집 |
| `run.sh` | 패키지 내 실행(.env 로드 + STUDIO_DIR 기본 `./studio`) |
| `systemd/rhwp-server.service` | (선택) systemd 유닛 |

패키지 구성: `rhwp-server`(릴리즈 바이너리) + `studio/`(dist) + `.env` + `run.sh`. sqlite는 런타임 자동 생성(바이너리 내장).

## 검증

```
single-origin (debug):
  / → <title>rhwp-studio</title>, /health → ok, /favicon.ico → 200   PASS

release 패키지(deploy/package.sh, 37M .tgz):
  생성된 rhwp-vm-package/run.sh 기동
   → listening + "studio 정적 서빙: ./studio"
   → / (studio) / /health (ok) / POST /documents (minio 업로드+fileId 발급)  PASS
```

## VM 배포 흐름(요약)

1. (개발머신) `deploy/build.sh` → `deploy/package.sh` → `rhwp-vm-package.tgz`
2. (VM) `tar xzf` → `.env` 편집(UPLOAD/DOWNLOAD/ADDR) → `./run.sh`
3. (Agent) iframe `src="https://VM/?fileId=X"` 또는 `?ssr=1`(빈문서)
4. (MCP) `GET /sessions/{id}/ir` · `POST /ops` · `POST /save`

## 한계(README 기재)

- 업로드 body limit(~2MB), 단일 편집자, 모델→사람 실시간 미반영, 평문 HTTP(외부 노출 시 TLS 프록시) — 모두 후속.
