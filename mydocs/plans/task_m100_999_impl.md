# Task #999 구현 계획서 — SSR 배포 전환

- 수행계획서: [task_m100_999.md](task_m100_999.md)
- 브랜치: `feature/ssr`
- 전제 확정: ①iframe 직접 HTTP(가능하면 same-origin) ②디바운스 배치 patch ③sqlite 영속 ④단일 편집자

## 설계 요지

- 서버는 **별도 crate** `server/` (`rhwp`를 path 의존)로 둔다. axum/tokio/rusqlite가 WASM 빌드(메인 crate)에 섞이지 않게 한다.
- 편집 결정성: 서버의 native 적용기는 클라이언트 WASM과 **동일한 `document_core` 경로**를 호출한다. 같은 op → 같은 결과.
- patch 양방향성: 각 `EditOperation`은 forward 데이터 + **inverse 데이터**(삭제 텍스트/이전 서식 id/분할 위치 등)를 함께 담는다.

---

## Stage 1 — Document IR → JSON 직렬화 경로

**목표**: 모델 조회(`GET /ir`)의 기반인 IR→JSON 직렬화를 메인 crate에 추가.

**작업**
- `src/model/document.rs` 및 하위 모델에 모델 조회용 **뷰 DTO**(`DocumentIrView`) 정의. 전체 IR 원형 직렬화 대신 모델이 읽기 좋은 구조(섹션→문단→텍스트/charShape/문단속성/컨트롤 요약)로 투영.
- `Document::to_ir_json(&self) -> String` (serde_json). native 전용(`cfg(not(wasm32))` 불필요 — serde는 양쪽 가능).
- 스키마 버전 필드(`schema_version`) 포함.

**산출물**: `src/model/ir_view.rs`(신규) + `to_ir_json`.
**검증**: 대표 샘플 3종(hwp/hwpx/표 포함) 파싱 → `to_ir_json` → 스냅샷 테스트(`cargo test`). 텍스트/문단수/표구조가 IR과 일치.

## Stage 2 — EditOperation 양방향 프로토콜 + native 적용기

**목표**: WASM 편집 API를 직렬화 가능한 양방향 patch로 승격하고, native에서 동일 적용.

**작업 (Rust)**
- `src/document_core/edit_op.rs`(신규): `EditOperation` enum 정의. `command.ts` 매핑:
  - `InsertText { pos, text }` / inverse: 같은 pos에서 `text.len` 삭제
  - `DeleteText { pos, count, deleted_text }` / inverse: `deleted_text` 삽입
  - `SplitParagraph { pos }` ↔ `MergeParagraph { pos, split_offset }`
  - `ApplyCharFormat { range, char_shape_id, prev_char_shape_ids }`
  - `Move*/Resize*` (table/picture/shape): before/after 위치·크기
  - 그 외(붙여넣기/객체삽입/표행열) → 연산 불가 표시 → **스냅샷형**으로 분류
- `apply(&mut self, op)` / `apply_inverse(&mut self, op)` — 기존 `document_core/commands/*` 재사용.
- WASM 노출: `applyOps(json)` / `applyInverseOps(json)` (`src/wasm_api.rs`).

**작업 (TS)**
- `rhwp-studio/src/engine/command.ts`: 각 `EditCommand`에 `serialize(): EditOperation | null` 추가(연산형만, 스냅샷형은 null→스냅샷 경로).

**검증**: 라운드트립 테스트 — 샘플에 `apply(op)` 후 `apply_inverse(op)` → 원본 IR과 동일. WASM 편집 결과 == native `applyOps` 결과.

## Stage 3 — Native Rust 서버(axum) + 세션 매니저 + sqlite 영속

**목표**: fileId 세션을 보유·영속하는 서버.

**작업**
- 신규 crate `server/` (`Cargo.toml`: rhwp path dep, axum, tokio, rusqlite, serde_json).
- 세션 매니저: `fileId -> SessionState { document: Document, base_seq, op_log, lock }` (in-memory, sqlite 백업).
- sqlite 스키마: `sessions(file_id PK, created, base_blob, format)`, `snapshots(file_id, seq, blob)`, `ops(file_id, seq, op_json, ts)`.
- 엔드포인트:
  - `POST /sessions` `{ fileId, format, fileBytes }` → 파싱 → 세션 생성/복원. **(minio는 외부 모듈이 fileBytes 공급)**
  - `POST /sessions/{id}/ops` `[EditOperation]` → 순차 apply + ops 테이블 append
  - `PUT /sessions/{id}/snapshot` `{ fileBytes }` → 스냅샷형 동기화(문서 replace + snapshots append)
  - `GET /sessions/{id}/ir` → `to_ir_json`
  - `DELETE /sessions/{id}` → close/flush
- 재시작 복원: sqlite의 base_blob + 이후 ops/snapshots 재적용.

**검증**: 세션 생성 → ops 적용 → 서버 재시작 → `GET /ir` 동일. 동시 요청 시 fileId lock 직렬화.

## Stage 4 — rhwp-studio 클라이언트 미러링 + iframe fileId 연결

**목표**: 편집을 서버로 미러링하고, iframe 진입을 세션과 연결.

**작업**
- 세션 부트스트랩: iframe 진입 시 `fileId`(+ 파일) 수신 → `POST /sessions` → 서버 IR과 동기화. fileId 수신 경로는 URL query 또는 기존 postMessage `loadFile` 확장.
- 편집 미러링: `input-handler`의 `executeOperation`에서 connational 발생 op를 `serialize()` → **디바운스 배치 큐** → `POST /ops`.
- 스냅샷형: `SnapshotCommand` 실행 시 `exportHwpx()` → `PUT /snapshot`.
- 통신 모듈 `rhwp-studio/src/core/session-client.ts`(신규): 서버 base URL, 디바운스, 재시도/오프라인 큐.

**검증**: 편집 → iframe 닫기 → 재진입 → 서버 상태로 복원되는지 E2E. 디바운스 중 닫아도 flush(beforeunload) 보장.

## Stage 5 — export API + 외부(minio) 모듈 인터페이스 경계

**목표**: 확정 저장용 바이너리 제공 + 외부 모듈 계약 명문화.

**작업**
- `GET /sessions/{id}/export?fmt=hwp|hwpx` → `serialize_document`.
- "확정 저장" 트리거 정의: 명시적 save 또는 `DELETE`(close) 시 export 산출.
- 외부 모듈 계약 문서(`mydocs/tech/`): input(fileId+bytes), output(export bytes), minio 연동은 외부 책임.

**검증**: export 바이트 재파싱 → 편집 반영 확인. hwp/hwpx 양 포맷 라운드트립.

## Stage 6 — 통합 / E2E 검증 + 최종 보고

**목표**: 전체 시나리오 검증.

**작업·검증**
- 시나리오: `POST /sessions` → 연산+스냅샷 혼합 편집 → iframe close → `GET /ir`(모델 조회) → `export` → 재파싱.
- WASM↔native 결정성 회귀(Stage 2 테스트 확장).
- `cargo test`(메인+server) / `cargo clippy` / studio e2e 통과.
- 최종 보고서 `task_m100_999_report.md`.

---

## 단계별 산출물·커밋 매핑

| Stage | 주요 산출물 | 커밋 단위 |
|-------|------------|----------|
| 1 | `ir_view.rs`, `to_ir_json` | IR JSON 직렬화 |
| 2 | `edit_op.rs`, `applyOps`, TS `serialize()` | EditOperation 프로토콜 |
| 3 | `server/` crate, sqlite | 서버+세션+영속 |
| 4 | `session-client.ts`, iframe 부트스트랩 | 클라이언트 미러링 |
| 5 | export API, 인터페이스 계약 문서 | export+경계 |
| 6 | 통합 테스트, `_report.md` | 통합 검증 |

각 Stage 완료 후 `_stage{N}.md` 작성 → 승인 → 다음 단계.

## 위험·완화 (구현 관점)

| 위험 | 완화 |
|------|------|
| WASM↔native 편집 불일치 | 동일 `document_core` 호출, Stage 2 라운드트립 회귀 고정 |
| 디바운스 중 종료로 op 유실 | `beforeunload` flush + 서버측 seq 기반 멱등 |
| sqlite blob 비대(대용량 문서) | 주기적 base 스냅샷 압축, ops 로그 컴팩션 |
| IR JSON 스키마 변경 호환 | `schema_version` + 뷰 DTO 분리 |
