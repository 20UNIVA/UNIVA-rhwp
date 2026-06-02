# Task #999 Stage 8(후속) — minio 저장소 프록시 (upload/download 연동)

- 브랜치: `feature/ssr`
- 성격: 외부 파일 저장소(minio) 실연동. rhwp-server가 프록시 역할.

## 목표

rhwp-server가 외부 minio 저장소(`UPLOAD_URL`/`DOWNLOAD_URL`)를 대행 호출하여:
- fileId 없는 신규 문서 → **upload**로 file_id 발급 + 세션 생성
- fileId만 있는 진입 → 세션/ sqlite 없으면 **download**로 가져와 세션 복원

(studio는 rhwp-server하고만 통신 → CORS 불필요, 저장소 URL 비노출)

## 구현

### `server/src/storage.rs` (신규)
- `Storage::from_env()` — `UPLOAD_URL`/`DOWNLOAD_URL` 환경변수
- `upload(bytes, filename) -> file_id` — multipart `file` POST, 응답 `{file_id}` 파싱
- `download(file_id) -> bytes` — `{file_id}` placeholder 치환 GET
- `enabled()` — 두 URL 모두 설정 시 활성

### `server/src/main.rs`
- `AppState.storage: Arc<Storage>`
- **`POST /documents`** `{filename?, fileBase64}` → 파싱검증 → upload → 발급 file_id로 세션 생성 → `SessionInfo`(fileId 포함) 반환
- `get_or_restore` 를 async로: 메모리 → sqlite → **minio download 폴백** 순. (편집 진행분 보존 위해 sqlite 우선)
- 모든 호출부 `.await` 적용

### `server/Cargo.toml`
- `reqwest`(default-features=false, multipart/json/stream) 추가 — http 전용

## 검증 (실서버 + 실제 minio 테스트 서버)

```
외부 저장소 연동: 활성

[POST /documents] new.hwp 업로드 → fileId=36a01f54-… 발급 + 세션 생성 + GET /ir 정상   PASS
[download 폴백]   빈 서버(db 없음)에 위 fileId로 GET /ir
                  → sqlite 미스 → minio download → 세션 복원 → 문단 정상            PASS
[없는 fileId]     GET /ir → 404                                                    PASS
```

## 비고 / 후속

- 서버 실행 시 `UPLOAD_URL`, `DOWNLOAD_URL`(`{file_id}` placeholder) 환경변수 필요. 미설정 시 저장소 기능 비활성(기존 fileId+바이트 직접 전달 방식만 동작).
- 대용량 업로드(axum body limit 2MB) 후속 과제 여전 — studio가 큰 문서 `POST /documents` 시 상향 필요.
- 다음(Stage 9): studio 측 워크플로우 — fileId 없으면 빈 문서 upload, 열기 시 빈문서 dirty 여부로 세션 닫기/유지 분기.
