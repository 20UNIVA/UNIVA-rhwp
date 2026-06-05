# Task #999 Stage 3 완료보고서 — Native Rust 서버(axum) + 세션 매니저 + sqlite 영속

- 브랜치: `feature/ssr`
- 구현계획서: [task_m100_999_impl.md](../plans/task_m100_999_impl.md) Stage 3

## 목표

`fileId`(=minio fileId) 단위로 `DocumentCore` 를 서버 메모리에 보유하고 sqlite 에 영속하는 서버를 구축한다. 클라이언트(iframe)가 닫혀도 상태가 유지되고, AI 모델이 IR JSON 을 조회할 수 있어야 한다.

## 구현 내용

### 신규 crate `server/` (독립 — WASM 빌드와 분리)

- `server/Cargo.toml`: `rhwp`(path 의존) + axum 0.7 + tokio + tower-http + rusqlite(bundled) + base64 + tracing
- `server/src/store.rs`: sqlite 영속 (`sessions`/`ops`/`snapshots` 3테이블)
- `server/src/main.rs`: 세션 매니저 + axum 라우터 + 핸들러 + 구동
- `server/.gitignore`: `/target`, `*.db`

### rhwp 엔진 진입점 재사용 (신규 로직 없음)

- 파싱: `rhwp::parse_document(&bytes)` (HWP/HWPX/HWP3 자동 판별)
- 로드: `DocumentCore::new_empty()` + `set_document(doc)` (styles/compose/paginate)
- 편집: `core.apply_edit_ops_json(json)` (Stage 2)
- IR JSON: `core.document().to_ir_json()` (Stage 1)

### API

| 메서드 | 경로 | 설명 |
|--------|------|------|
| POST | `/sessions` | 세션 생성/재생성 `{fileId, format?, fileBase64}` |
| POST | `/sessions/{id}/ops` | 연산형 patch 적용 `[EditOperation, ...]` |
| PUT | `/sessions/{id}/snapshot` | 스냅샷형 동기화 `{fileBase64}` |
| GET | `/sessions/{id}/ir` | 현재 상태 IR JSON (모델 조회) |
| DELETE | `/sessions/{id}` | 메모리 세션 해제 (영속 유지) |
| GET | `/health` | 헬스 체크 |

### 영속/복원 설계

- 세션 생성 시 원본 바이트를 `sessions.base_blob` 에 저장, ops/snapshots 초기화
- op 적용마다 `ops(seq, op_json)` append (seq 단조 증가)
- 스냅샷 동기화 시 `snapshots(seq, blob)` append
- **복원**: 메모리에 없으면 sqlite 에서 "가장 최근 snapshot(없으면 base) + 그 이후 ops 재적용"으로 `DocumentCore` 재구성 후 메모리 등록 (`get_or_restore`)

## 검증

### 단위 테스트 (sqlite 영속)
```
cargo test (server)
  store::tests::test_create_and_load ... ok
  store::tests::test_snapshot_supersedes_base ... ok
  store::tests::test_load_missing ... ok
  test result: ok. 3 passed
```

### End-to-End 스모크 (실서버 기동)
샘플 `samples/re-align-center-hancom.hwp` 로 검증:
```
CREATE   200  {sectionCount:1, paragraphCount:1}
GET /ir       첫 문단 "가나다라마바사아…" (schema_version=1)
POST /ops     [{insert_text, offset:0, text:"XYZ"}] → 200
GET /ir       "XYZ가나다라마바사아…"  → PASS
```

### 영속/복원 검증 (핵심 — "닫아도 유지")
```
서버 종료 → 같은 DB로 재시작(메모리 비어있음) → GET /sessions/doc1/ir
  → "XYZ가나다라마바사아…"  → PASS
  (sqlite base_blob 파싱 + op 재적용으로 복원)
```

→ 클라이언트 연결과 무관하게 서버단에 문서·패치가 유지되고, 모델이 IR JSON 으로 현재 상태를 조회할 수 있음을 실증.

## 스코프/한계

- **export API**(`GET /export`)는 Stage 5 에서 추가 (확정 저장용 바이너리 + 외부 minio 모듈 경계).
- 세션 lock 은 `std::sync::Mutex` (단일 편집자 1차 가정). 작업이 짧아 async 블로킹 영향 미미. 대용량 동시성은 후속 `spawn_blocking` 검토.
- 세션 TTL/정리 정책 미구현 (DELETE 는 메모리만 해제). 후속.

## 다음 단계

Stage 4 — rhwp-studio 클라이언트 미러링(디바운스 배치) + iframe `fileId` 세션 연결.
