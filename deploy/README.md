# rhwp SSR — VM 배포 가이드 (최소 패키지)

VM에 **rhwp-server 바이너리 1개 + studio 정적 자산(dist)** 만 올리면 동작합니다.
rhwp-server가 API와 studio 정적 파일을 **같은 포트(single-origin)** 로 서빙하므로
별도 웹서버(nginx)나 CORS 설정이 필요 없습니다.

```
[Agent 서비스] ──iframe──▶ https://VM-HOST/?fileId=...   (studio)
                            └─ 같은 호스트의 /sessions, /documents … (API)
[모델/MCP]    ──HTTP──▶ https://VM-HOST/sessions/{id}/ir, /ops
rhwp-server  ──HTTP──▶ minio (UPLOAD_URL / DOWNLOAD_URL)
```

---

## 1. 패키지 구성 (VM에 올라가는 것)

```
rhwp-vm-package/
├── rhwp-server        # Rust 릴리즈 바이너리 (단일 실행파일)
├── studio/            # studio 정적 자산 (vite dist: index.html, assets, fonts, sw.js …)
│   └── ... (WASM 포함)
├── .env               # 환경변수 (.env.example 복사·수정)
└── run.sh             # 실행 스크립트
```

> sqlite는 런타임에 `RHWP_SERVER_DB` 경로에 자동 생성됩니다(별도 설치 불필요, 바이너리에 내장).

---

## 2. 빌드 (개발 머신에서 1회)

빌드 머신 요구사항: **Docker**(WASM 빌드용), **Node ≥ 20.19**(vite), **Rust toolchain**.

```bash
deploy/build.sh        # WASM → studio dist → server release 순서로 빌드
deploy/package.sh      # 산출물을 deploy/rhwp-vm-package/ 로 모으고 .tgz 생성
```

산출물:
- `server/target/release/rhwp-server`
- `rhwp-studio/dist/`
- → `deploy/rhwp-vm-package.tgz` (VM으로 전송할 패키지)

---

## 3. VM에서 실행

```bash
tar xzf rhwp-vm-package.tgz && cd rhwp-vm-package
cp .env .env.local 2>/dev/null || true   # 필요 시 편집
vi .env                                   # UPLOAD_URL/DOWNLOAD_URL/ADDR 등 확인
./run.sh
```

기본 포트 `0.0.0.0:7710`. 헬스체크: `curl http://localhost:7710/health` → `ok`.

### 환경변수 (.env)

| 변수 | 예시 | 설명 |
|------|------|------|
| `RHWP_SERVER_ADDR` | `0.0.0.0:7710` | bind 주소 |
| `RHWP_SERVER_DB` | `/var/lib/rhwp/sessions.db` | sqlite 경로(작업 중 세션 영속) |
| `RHWP_STUDIO_DIR` | `./studio` | studio 정적 자산 경로(설정 시 same-origin 서빙) |
| `UPLOAD_URL` | `http://minio-host:25029/upload` | minio 업로드(파일→file_id, file_id 포함 시 덮어쓰기) |
| `DOWNLOAD_URL` | `http://minio-host:25029/download/{file_id}` | minio 다운로드(`{file_id}` placeholder 필수) |
| `RUST_LOG` | `rhwp_server=info` | 로그 레벨 |

> `RHWP_STUDIO_DIR` 를 비우면 **API 전용**으로 뜹니다(정적 서빙 끔). 이 경우 studio는 별도 웹서버로 서빙하고 iframe에 `?ssrBase=`로 서버 주소를 넘기면 됩니다(CorsLayer permissive).

---

## 4. Agent 서비스에서 iframe 연결

same-origin 배포(권장)에서는 `ssrBase` 없이 동작합니다.

```html
<!-- 새 빈 문서로 시작 (서버가 빈문서 업로드 후 fileId 발급) -->
<iframe src="https://VM-HOST/?ssr=1"></iframe>

<!-- 기존 문서 열기 (minio file_id) -->
<iframe src="https://VM-HOST/?fileId=<minio-file-id>"></iframe>
```

- 사용자가 편집하면 자동으로 서버 세션에 미러링(디바운스 배치).
- "저장" 누르면 같은 file_id로 minio에 덮어쓰기.
- 주소창 `?fileId=`가 자동 갱신되어 새로고침/공유 시 복원됩니다.

별도 origin(studio를 다른 호스트에서 서빙)일 때:
```html
<iframe src="https://STUDIO-HOST/?fileId=X&ssrBase=https://VM-HOST"></iframe>
```

---

## 5. 모델/MCP 연동

MCP는 VM의 rhwp-server에 직접 HTTP로 붙습니다(사람과 동일 인터페이스).

| 용도 | 호출 |
|------|------|
| 문서 구조 조회 | `GET /sessions/{id}/ir` (또는 `?page=N` 페이지별) |
| 편집 적용 | `POST /sessions/{id}/ops` `[{op, section, para, offset, text}, …]` |
| 현재본 내보내기 | `GET /sessions/{id}/export?fmt=hwp|hwpx` |
| 저장(덮어쓰기) | `POST /sessions/{id}/save` |

`/ir` 의 문단 `index`(section/para)는 절대 좌표이므로, 페이지 필터로 봐도 그 좌표로 `/ops` 를 그대로 보내면 됩니다.

---

## 6. (선택) systemd 서비스

`deploy/systemd/rhwp-server.service` 참고. 패키지를 `/opt/rhwp/` 에 두고:

```bash
sudo cp -r rhwp-vm-package /opt/rhwp
sudo cp deploy/systemd/rhwp-server.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now rhwp-server
```

---

## 현재 한계 (배포 전 인지)

- **업로드 크기**: axum 기본 body limit(약 2MB) — 그보다 큰 hwp 업로드는 413. 대용량 문서를 다룰 경우 `DefaultBodyLimit` 상향 필요(후속).
- **단일 편집자 가정**: 한 fileId를 사람·모델이 동시 편집하면 op 좌표가 어긋날 수 있음. 모델은 편집 직전 `GET /ir` 재확인 권장.
- **모델 편집 → 사람 화면 실시간 반영 없음**: 단방향 미러링. 사람 화면 갱신은 새로고침(복원). 양방향 푸시(SSE/WebSocket)는 후속.
- **TLS**: rhwp-server는 평문 HTTP. 외부 노출 시 리버스 프록시(nginx/caddy)로 TLS 종단 권장.
