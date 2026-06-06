# 10 — UNIVA-rhwp SSR 통합 Sub-2 설계: 11+1 hwp-doc-patch 액션의 서버 SoT 완성

작성일 2026-06-06. 본 문서는 *11+1개 액션을 EditOperation variants 로 승격해 서버가 모든 편집의 SoT(진실 원천)가 되도록* 하는 Sub-2 의 설계서. Sub-1 의 설계는 [task_m200_zephy_bridge.md](task_m200_zephy_bridge.md), Sub-1 의 결과 보고는 [report/task_m200_zephy_bridge_report.md](../report/task_m200_zephy_bridge_report.md) 에 있다.

## 1. 배경

Sub-1 에서 *접합 인프라* + `insert_text` 1개 종단 시연을 통과시켰다. 그러나 SKILL.md ([hwp-doc-edit/SKILL.md](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/SKILL.md)) 의 *12 명령* 중 나머지 11+1 (조회·종료 포함) 은 *passthrough* 로 흘려 보낸다. 서버는 그 액션의 *의미*를 모르고 broadcast 만 던지며, *서버 IR* 은 안 바뀐다. 즉:

- 두 사용자가 같은 fileId 로 동시에 접속해 있으면 — *현재 연결된 탭은* 각 클라가 workbench broadcast 를 받아 wasm 으로 적용하므로 화면이 갱신된다.
- *서버 상태* 는 안 바뀌므로, 새 탭이 접속해 `/ir` 또는 `/export` 로 복원하면 — *passthrough 액션의 변경은 보이지 않는다*. *새로고침해도 마찬가지*.

이 상태는 [task_m200_zephy_bridge_report.md](../report/task_m200_zephy_bridge_report.md) 의 "Sub-2로 미루는 항목" 절에서 *Sub-2 의 본질* 로 명시했다. 본 spec 은 그 본질을 푼다.

## 2. 목표와 비목표

**목표 (Sub-2)**

1. SKILL.md 의 12 명령 (`replace-runs`·`set-paragraph-style`·`delete-range`·`insert-paragraph`·`delete-element`·`insert-table`·`set-cell-style`·`merge-cells`·`replace-cell-runs`·`insert-text-in-cell`·`delete-range-in-cell` + `complete`) 을 *서버가 진짜로 적용*하도록 한다. `insert-text` 와 `get-ir-slice` 는 Sub-1 또는 본 spec 의 별도 endpoint 로 처리.
2. 적용 결과는 *서버 sqlite 에 영속*되어 — 새 접속자가 `/ir` 또는 `/export` 로 복원했을 때 *완전한 상태*를 본다.
3. 모든 액션에 대해 *서버 undo* 를 보장한다 (`POST /sessions/:id/undo`). 단일 사용자 가정.
4. `complete` 액션은 *최종 snapshot 영속* + `ServerEvent::Complete` broadcast 로 다른 탭에 종료 알림.
5. `get-ir-slice` 는 새 endpoint `GET /sessions/:id/ir-slice` 로 — *현재 코어 IR 의 부분 slice* 만 반환.
6. 디버깅 가시성 보장 — broadcast 페이로드에 *정방향 EditOperation* 직렬화 본문 그대로 포함. 사후 `GET /sessions/:id/audit` 으로 액션 시퀀스 + 페이로드 조회. `GET /sessions/:id/diff` 으로 액션 전후 IR 차이 확인.

**비목표 (Sub-3 로 미룸)**

- 다중 사용자 동시 편집 시 *사용자별 undo stack 분리* — 본 spec 은 *세션당 전역 undo stack*.
- snapshot binary 누적 *delta 압축·LRU* — 본 spec 은 *마지막 N=100 개만 보관* 단순 정책.
- `apply_inverse_edit_op` 의 *EditOperation 기반 역연산* 보완 — 본 spec 은 *snapshot stash 만* 활용. 기존 4 variants 의 inverse 는 그대로 유지.
- sqlite write 실패 시 *코어 IR-sqlite 불일치* 자동 회복 — 본 spec 은 *알려진 한계 보고*.
- rhwp-studio 워크벤치 UI 패널 — 본 spec 은 *노트북 호출만* 검증.

## 3. 전체 로드맵 중 Sub-2 의 위치

| 번호 | 범위 | 산출물 |
|---|---|---|
| Sub-1 (완료) | 접합 인프라 + `insert_text` 1개 진짜 적용 | events 모듈, ws handler, workbench endpoint, session-client WS 갈아엎기, 새 노트북 |
| **Sub-2 (본 spec)** | 11+1 액션 진짜 적용 + sqlite snapshot stash + undo/audit/diff/ir-slice 신규 endpoint | EditOperation 12 신규 variants, native 신설 3개, 서버 workbench 12 arms, sqlite op_stash, ServerEvent 2 신규 variants, 클라 onServerEvent 12 분기 확장, 양방향 e2e 12+ |
| Sub-3 (다음) | 다중 사용자 undo 분리, delta 압축, write-ahead log, 워크벤치 UI 패널 | (별도 spec) |

## 4. 접근법 결정 — *정방향만 EditOperation, 역방향은 snapshot stash*

본 spec 의 *접근법* 결정은 brainstorming 단계에서 4개 후보 중 다음 1개로 확정했다.

| 후보 | broadcast 페이로드 | inverse 직렬화 부담 | rhwp 본체 헤더 의도 | 정방향 가시성 |
|---|---|---|---|---|
| (A) 12 액션 *완전* inverse 포함 | 큼 (KB~MB) | 11종 형식 0 설계 | *위배* (역연산 표현 불가는 snapshot 으로 처리하라는 [edit_op.rs](../../src/document_core/commands/edit_op.rs#L11-L12) 헤더) | 완전 |
| (B) workbench passthrough + 서버도 native 호출 + binary snapshot stash | 작음 | 0 | 정합 | action+payload 만 |
| (C) 2-tier — 정밀 군은 EditOp, 복잡 군은 snapshot push | 가변 | 6종 | 부분 정합 | 가변 |
| **(채택) 정방향 EditOp + 역 snapshot stash** | 작음 (정방향만) | 0 | 정합 | *완전* (EditOp 데이터 단위) |

**결정 이유**

- (A) 의 *진짜 부담* 은 `DeleteElement`·`MergeCells` 같은 *대량 inverse 데이터* (삭제된 element 전체, 합쳐진 cell 들의 원형) 의 직렬화 형식 0 설계. *외부 contract* 로 한 번 박으면 v2 마이그레이션 부담 영구. (8) 의 *정방향 가시성 ↓* 문제는 *EditOp 의 정방향 데이터 자체* 가 의미 단위라 풀린다.
- inverse 데이터는 *서버 sqlite 내부 binary* (export_hwpx_native 의 결과) 로만 보관. 외부 contract 없음. broadcast 페이로드에 포함 안 함.
- undo 시점에는 sqlite 에서 binary 꺼내 `import_hwpx_native` 통째 교체 + 다른 클라에 `ServerEvent::SnapshotRestored { snapshot_base64 }` broadcast.
- rhwp 본체 헤더 docstring 의 *"역연산 표현 불가는 snapshot 으로 처리"* 의도와 *정합*.

## 5. 컴포넌트와 책임

| # | 컴포넌트 | 위치 | 책임 |
|---|---|---|---|
| 1 | rhwp 본체 native 신설 | `src/document_core/commands/text_editing.rs` 등 | `replace_runs_native`, `replace_cell_runs_native` 2 신설 (*delete_range_native 가 `cell_ctx: Option<(usize,usize,usize)>` 으로 셀 다문단 범위 삭제를 이미 지원하므로 multipara 신설 불필요* — 코드 조사 결과 `text_editing.rs:602` 확인). `apply_para_format_native(sec, para, props_json: &str)` / `set_cell_properties_native(..., json)` / `apply_char_format_native(..., props_json)` *모두 partial JSON 직접 수용* — 코드 조사로 확인. 즉 서버 핸들러는 `PartialXxx` → JSON 직렬화 → native partial 인자 직접 전달. *서버 측 read+병합 단계 불필요*. |
| 2 | EditOperation 12 신규 variants | `src/document_core/commands/edit_op.rs` (수정) | 정방향만 — `ReplaceRuns`·`SetParagraphStyle`·`DeleteRange`·`InsertParagraph`·`DeleteElement`·`InsertTable`·`SetCellStyle`·`MergeCells`·`ReplaceCellRuns`·`InsertTextInCell`·`DeleteRangeInCell` + (참고: `InsertText` 기존 유지). `Partial*` 타입들 (`PartialParagraphStyle`, `PartialCellStyle`, `RunSpec`, `ElementType` 등). `apply_edit_op` match arm 11 신설 (옵션 b 패턴: 읽기 → 부분 병합 → 완전 객체로 native). `apply_inverse_edit_op` 의 신규 variants 분기는 *unreachable!* — sqlite snapshot stash 경로로 위임. |
| 3 | sqlite op_stash | `server/src/store.rs` (수정) | 신규 테이블 `op_stash(seq INTEGER PRIMARY KEY, file_id TEXT, op_json TEXT, before_snapshot_blob BLOB, created_at INTEGER)`. `append_op_stash` / `pop_op_stash` / `list_ops_by_range` / `get_before_blob_by_seq`. *마지막 100 entry 정책* (101번째 진입 시 가장 오래된 row 삭제). |
| 4 | ServerEvent 2 신규 + rename_all 정정 | `server/src/events.rs` (수정) | `Complete { seq }` / `SnapshotRestored { seq, snapshot_base64: String }`. 기존 variants 보존. *Sub-1 의 `#[serde(rename_all = "lowercase")]` 를 `snake_case` 로 변경* — `SnapshotRestored` 가 JSON 에서 `"kind":"snapshot_restored"` 로 직렬화되도록. 기존 `Ops`·`Workbench` 는 lowercase 와 snake_case 결과가 동일 (`"ops"`/`"workbench"`) — 호환성 깨지지 않음. |
| 5 | 서버 workbench 12 arms | `server/src/main.rs::workbench` (수정) | 액션 12개 match arm — payload 검증 (엄격: 낯선 키 400, 옵셔널 키 누락 200) + EditOperation 변환 + `core.save_snapshot_native` 직전 호출 (id 보관, 메모리) + `core.export_hwpx_native` (before_blob, sqlite 영속용) + `core.apply_edit_op(&op)` + `store.append_op_stash` + `broadcast ServerEvent::Ops`. `complete` 특수 arm — `export_hwpx_native` (final blob) + sqlite 영속 + `broadcast ServerEvent::Complete`. |
| 6 | 신규 endpoint 4개 | `server/src/main.rs` (수정) | `POST /sessions/:id/undo` (stash pop → `import_hwpx_native` → `broadcast SnapshotRestored`), `GET /sessions/:id/audit?seq_from=&seq_to=`, `GET /sessions/:id/diff?seq=N`, `GET /sessions/:id/ir-slice?sec=&para_start=&para_end=&mode=`. |
| 7 | 클라 onServerEvent 확장 | `rhwp-studio/src/main.ts` (수정) | `ev.kind === 'ops'` 분기에 신규 12 종 op 처리 추가 — 각 op 종류별 wasm 호출 (예: `op.op === 'replace_runs'` → `wasm.replaceRuns(...)`). `ev.kind === 'snapshot_restored'` 핸들러 신설 — `wasm.importHwpx(snapshot_base64)` 통째 교체. `ev.kind === 'complete'` 핸들러 — Sub-2 범위에서는 `console.log` 만. `eventBus.emit('document-changed')` 는 Sub-1 패턴 그대로. |
| 8 | 양방향 e2e | `rhwp-studio/e2e/` (수정) | 액션별 12 e2e + undo + audit + diff + ir-slice + complete. 각 액션마다 *정방향 broadcast + 서버 IR 영속 + 클라 wasm 호출 확인*. |
| 9 | 노트북 SSR 라우터 회귀 | `hwp_sub_agent_simulation_ssr.ipynb` (검증) | Sub-1 셀 3 의 `_normalize_action` / `_normalize_payload` 가 12 액션 모두 처리 가능 확인. 부분 업데이트 시연 (bold 만 보내고 다른 서식 유지) 시나리오 셀 추가. |

각 컴포넌트는 한 단위의 책임. *rhwp 본체 PR*, *server crate PR*, *클라 PR*, *노트북 변경* 4 PR 을 substage 단위로 합쳐 진행.

## 6. 호출자 책임 — 정규화는 *호출자 측*

서버 `POST /workbench` 는 *snake_case action* + *정규화된 payload 키* 만 수용한다. SKILL.md 의 원형 (`replace-runs`, `sec`, `char_offset`) 을 그대로 보내면 *400 INVALID_PAYLOAD*.

| 호출자 | 정규화 위치 |
|---|---|
| 노트북 SSR 라우터 | Sub-1 셀 3 의 `_normalize_action` / `_normalize_payload`. 12 액션 모두 지원하도록 키 매핑 표 확장 |
| rhwp-studio 워크벤치 패널 (Sub-3) | `core/workbench-client.ts` (Sub-3 에서 신설) |
| 직접 `curl` 디버깅 | 호출자가 직접 snake_case + 정규화 키로 |

정규화 매핑 표 (확정):

| SKILL.md 원형 | 서버 수용 |
|---|---|
| 액션 이름 kebab-case (`replace-runs`) | snake_case (`replace_runs`) |
| `sec` | `section` |
| `char_offset` | `offset` |
| `cell_para_start` / `cell_para_end` | 그대로 |
| 그 외 키 | 그대로 |

## 7. 부분 업데이트 — *호출 직전 read + 병합*

`set_paragraph_style` / `set_cell_style` / `replace_runs` / `insert_text` (style 필드) 등은 *부분 객체* 를 수용한다. 모델이 `bold` 만 바꾸고 싶은 호출은 `{style: {bold: true}}` 만 보내고 — 다른 서식은 *현재 값 유지*.

**처리 — rhwp native 가 *이미 partial JSON 직접 수용***. 코드 조사 결과 `apply_para_format_native(sec, para, props_json: &str)` 가 `ParaShapeMods` (Option<> 필드 11개) 로 파싱 — *지정되지 않은 필드는 현재 값 유지*. `apply_char_format_native` 도 `CharShapeMods` 동일 패턴. `set_cell_properties_native` 도 JSON 수동 파싱으로 *지정된 키만* 업데이트.

즉 서버 핸들러는 *`PartialXxx` Rust struct → serde_json 으로 JSON 직렬화 → native partial JSON 인자에 그대로 전달*. *서버 측 read + 병합 단계 불필요*.

예시 — `set_paragraph_style { section: 0, para: 3, style: {alignment: "right"} }` 처리:

```rust
// 1. EditOperation::SetParagraphStyle 로부터 PartialParagraphStyle 추출
let partial: &PartialParagraphStyle = &op.style;

// 2. JSON 직렬화 (옵셔널 None 필드는 skip_serializing_if 로 제외)
let props_json = serde_json::to_string(partial)
    .map_err(|e| AppError::Internal(format!("serialize: {e}")))?;
// → '{"alignment":"right"}'

// 3. native partial 호출
core.apply_para_format_native(0, 3, &props_json)?;
// → alignment 만 right 로 갱신, 나머지는 유지
```

**필수 vs 옵셔널 분류**

| 액션 | 필수 키 | 옵셔널 키 (부분 업데이트) |
|---|---|---|
| `replace_runs` | section, para, runs[] | (runs[i].style 은 부분) |
| `set_paragraph_style` | section, para | style.align, style.indent, style.line_height |
| `delete_range` | section, para_start, char_start, para_end, char_end | — |
| `insert_paragraph` | section, after_para (*위치* — Enter 와 동일, 해당 index 에 새 문단 삽입 + 기존 문단 뒤로 밀림) | count(default=1), style 부분 (각 신규 문단에 동일 적용) |
| `delete_element` | section, para, element_type | — |
| `insert_table` | section, insert_after_para, rows, cols | — |
| `set_cell_style` | section, table_para, row, col | style.bgcolor, style.border, ... |
| `merge_cells` | section, table_para, row_start, col_start, row_end, col_end | — |
| `replace_cell_runs` | section, table_para, row, col, cell_para, runs[] | — |
| `insert_text_in_cell` | section, table_para, row, col, cell_para, offset, text | style 부분 |
| `delete_range_in_cell` | section, table_para, row, col, cell_para_start, char_start, cell_para_end, char_end | — |

EditOperation 직렬화 형식:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartialParagraphStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<u32>,
}

pub enum EditOperation {
    // 기존 4 변종 (정+역) — 그대로 유지
    InsertText { section, para, offset, text },
    DeleteText { section, para, offset, count, deleted_text },
    SplitParagraph { section, para, offset },
    MergeParagraph { section, para, prev_len },

    // 신규 — 정방향 데이터만. 역방향은 snapshot stash.
    ReplaceRuns { section: usize, para: usize, runs: Vec<RunSpec> },
    SetParagraphStyle { section: usize, para: usize, style: PartialParagraphStyle },
    DeleteRange { section: usize, para_start: usize, char_start: usize, para_end: usize, char_end: usize },
    InsertParagraph { section: usize, after_para: usize, #[serde(default = "default_count")] count: usize, #[serde(default)] style: Option<PartialParagraphStyle> },
    DeleteElement { section: usize, para: usize, element_type: ElementType },
    InsertTable { section: usize, insert_after_para: usize, rows: usize, cols: usize },
    SetCellStyle { section: usize, table_para: usize, row: usize, col: usize, style: PartialCellStyle },
    MergeCells { section: usize, table_para: usize, row_start: usize, col_start: usize, row_end: usize, col_end: usize },
    ReplaceCellRuns { section: usize, table_para: usize, row: usize, col: usize, cell_para: usize, runs: Vec<RunSpec> },
    InsertTextInCell { section: usize, table_para: usize, row: usize, col: usize, cell_para: usize, offset: usize, text: String, #[serde(default)] style: Option<PartialRunStyle> },
    DeleteRangeInCell { section: usize, table_para: usize, row: usize, col: usize, cell_para_start: usize, char_start: usize, cell_para_end: usize, char_end: usize },
}
```

## 8. inverse 메커니즘 — *sqlite snapshot stash (옵션 B)*

신규 12 variants 는 `apply_inverse_edit_op` match arm 에서 `unreachable!("Sub-2 variants use snapshot stash for inverse")` 로 두고, *역연산은 sqlite snapshot stash 경로*에서만 처리. 기존 4 variants 의 inverse 는 *그대로 유지*. `apply_inverse_edit_op` 는 현 시점에 [edit_op.rs unit test 안에서만 호출](../../src/document_core/commands/edit_op.rs#L177) — 외부 public 경로 없음. 즉 *신규 12 의 inverse unit test 를 작성하지 않으면* unreachable! 가 트리거될 일 없음. 신규 12 variants 의 unit test 는 *정방향만*.

sqlite schema (신규 테이블):

```sql
CREATE TABLE op_stash (
    seq           INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id       TEXT NOT NULL,
    op_json       TEXT NOT NULL,           -- 정방향 EditOperation 직렬화
    before_blob   BLOB NOT NULL,           -- 호출 직전 export_hwpx_native 결과
    created_at    INTEGER NOT NULL,
    FOREIGN KEY (file_id) REFERENCES sessions(file_id)
);
CREATE INDEX idx_op_stash_file_seq ON op_stash(file_id, seq);
```

**snapshot binary 영속 정책 (옵션 B)**

- 매 액션마다 `core.export_hwpx_native()` → `before_blob` 을 sqlite 에 영속.
- *세션(file_id)당* 마지막 100 entry 만 보관. 101번째 진입 시 *해당 file_id 의 가장 오래된 row* 삭제 (`DELETE FROM op_stash WHERE file_id = ? AND seq IN (SELECT seq FROM op_stash WHERE file_id = ? ORDER BY seq ASC LIMIT 1)`).
- *옵션 B → 옵션 A (in-memory only) 다운그레이드 마이그레이션은 단방향 코드 삭제 — 위험도 낮음*. Sub-3 진입 시 디스크 부담 평가 후 결정 가능.

**Undo 흐름**

```
1. POST /sessions/:id/undo
2. SELECT * FROM op_stash WHERE file_id = ? ORDER BY seq DESC LIMIT 1
3. let new_core = DocumentCore::from_bytes(&row.before_blob)?;   // 신규 인스턴스 (hwp/hwpx auto-detect)
4. session.core = new_core;                                       // 기존 core 통째 교체
5. DELETE FROM op_stash WHERE seq = ?
6. broadcast ServerEvent::SnapshotRestored { seq, snapshot_base64: base64(&row.before_blob) }
7. 응답 {seq_reverted: <removed seq>, applied: "undo"}
```

*rhwp 본체에 `import_hwpx_native(&mut self, blob)` 같은 *기존 인스턴스 교체* 메서드는 없음 — `DocumentCore::from_bytes(data: &[u8])` 가 *신규 인스턴스 생성*만 지원. Sub-2 는 *Session.core 필드를 통째 덮어쓰기* 패턴 ([server/src/main.rs](../../server/src/main.rs#L54-L61) 의 `Session` struct).*

다른 클라:
```
ev.kind === 'snapshot_restored'
  → wasm.importHwpx(snapshot_base64)
  → eventBus.emit('document-changed')
```

## 9. 데이터 흐름 — 6 시나리오

### 9-A — LLM 의 정상 액션 (`replace_runs`)

```
1. LLM Bash("hwp-doc-patch replace-runs --payload '{"sec":0,"para":3,"runs":[...]}'")
2. [노트북 SSR 라우터] (Sub-1 셀 3 정규화 그대로, 12 액션 지원 매핑 확장)
   - kebab → snake: "replace-runs" → "replace_runs"
   - 키 매핑: sec → section
   - POST /workbench {action:"replace_runs", payload:{section, para, runs}}
3. [server::workbench]
   a. payload → EditOperation::ReplaceRuns 변환 + 검증
       - 낯선 키 (예: "sec" 잔존) → 400 INVALID_PAYLOAD
       - 필수 키 (section/para/runs) 누락 → 400 MISSING_REQUIRED_KEY
       - 옵셔널 키 (runs[i].style 부분) 누락 → 200 진행
   b. core.save_snapshot_native() → before_id (in-memory)
   c. core.export_hwpx_native() → before_blob
   d. core.apply_edit_op(&op)?
       - 내부에서 옵션 b: 현재 값 read → Partial 병합 → replace_runs_native 호출
   e. store.append_op_stash(seq, op_json, before_blob)
       - 정책: 마지막 100 entry 초과 시 가장 오래된 row 삭제
   f. broadcast ServerEvent::Ops { seq, ops: [op] }
   g. 응답 {seq, applied:"ops"}
4. [다른 클라 onServerEvent]
   - op.op === "replace_runs" → wasm.replaceRuns(section, para, runs)
   - eventBus.emit('document-changed') → CanvasView refreshPages
```

### 9-B — 클라 직접 편집 (Sub-1 기존 경로)

기존 `ClientMessage::Ops` 그대로. 클라가 보내는 op 종류가 *신규 12 종 포함 가능*. 서버 `ws.rs::handle_client_text` 가 `apply_edit_ops_json` 호출 → 자동으로 신규 variants 도 처리. 단 *클라가 ops 를 보내는 경우 sqlite op_stash 에도 영속해야 하는가* 결정 — **본 spec 은 클라 ops 경로도 op_stash 에 영속**. 사용자가 직접 친 텍스트도 undo 대상.

### 9-C — Undo 요청

위 8절 끝부분 참조.

### 9-D — Audit 조회

```
GET /sessions/:id/audit?seq_from=10&seq_to=20
→ [
    {seq: 10, op_json: "...", created_at: 1717..},
    {seq: 11, ...},
    ...
  ]
```

각 액션의 *정방향 EditOperation* 본문 + 타임스탬프. 디버깅 시 "어떤 액션이 일어났는가" 즉시 확인.

### 9-E — Diff 조회

```
GET /sessions/:id/diff?seq=15
→ {
    seq: 15,
    op: {...},
    before_paragraphs: [...],   // import before_blob 후 IR 의 본문 paragraph text list
    after_paragraphs: [...],    // 다음 seq 의 before_blob (=현 seq 의 after) IR
    chars_added: 42,
    chars_removed: 0,
  }
```

before_blob 두 개를 `import_hwpx_native` 로 *임시 코어* 두 개에 적용 후 IR 텍스트 비교. 응답 형식은 *짧은 요약* 만. 대용량 binary 는 응답에 포함 안 함.

### 9-F — `get-ir-slice` 조회

```
GET /sessions/:id/ir-slice?sec=0&para_start=3&para_end=7&mode=compact
→ IR JSON slice
```

`mode`:
- `raw` — 모든 필드 포함
- `compact` — 텍스트·좌표만, 스타일 인덱스만
- `auto` — 길이가 일정 임계값 이하면 raw, 초과면 compact

본 endpoint 는 *편집 아님*. EditOperation 변환 안 함. broadcast 없음.

### 9-G — `complete` 종료 시그널

```
POST /workbench {action:"complete"}
1. [server::workbench complete arm]
   a. core.export_hwpx_native() → final_blob
   b. store.save_final_snapshot(file_id, seq, final_blob)
      // 신규 테이블: final_snapshots(file_id PK, seq, blob, created_at)
      // file_id 가 PK — 한 세션당 *마지막 최종 snapshot 만* 유지 (UPSERT). 누적 안 됨.
   c. broadcast ServerEvent::Complete { seq }
2. [다른 클라]
   - Complete 받으면 Sub-2 범위 — console.log 만. UI 통합은 Sub-3.
```

## 10. 에러 처리

| 발생 지점 | 에러 종류 | 처리 |
|---|---|---|
| workbench payload 변환 | 낯선 키, 타입 불일치 | 400 `{error:"INVALID_PAYLOAD", action, offending_keys}`. broadcast 안 함. sqlite 안 씀. |
| workbench payload 변환 | 필수 키 누락 | 400 `{error:"MISSING_REQUIRED_KEY", action, missing}`. |
| save_snapshot_native 실패 | OOM | 500 `{error:"SNAPSHOT_FAILED"}`. broadcast 안 함. |
| export_hwpx_native 실패 | 직렬화 오류 | 500 `{error:"EXPORT_FAILED"}`. 이미 잡은 before_id 는 `discard_snapshot_native`. broadcast 안 함. |
| apply_edit_op 실패 | 좌표 invalid 등 HwpError | 500 `{error:"APPLY_FAILED", details}`. before_id `discard_snapshot_native`. before_blob 폐기. sqlite 안 씀. |
| broadcast 실패 | receiver 없음 / lagged | 로그만. 응답에 영향 없음. |
| sqlite op_stash write 실패 | 디스크 full | 500 `{error:"PERSIST_FAILED"}`. *적용된 변경은 코어 IR 에 남아있음* (알려진 불일치 — Sub-3 에서 write-ahead log 로 해결). |
| undo stash empty | logical | 409 `{error:"NO_UNDO_AVAILABLE"}`. |
| import_hwpx_native 실패 | binary 파손 | 500. *복원 시도 후 자동 삭제* — 파손된 row 누적 방지. |
| 알 수 없는 op 종류 (클라 구버전) | logical | `console.warn` + skip. broadcast 의 *모든 ops* 가 skip 되면 emit('document-changed') 안 함 (Sub-1 Critical fix #2 패턴 그대로). |

## 11. Testing

| 레이어 | 테스트 |
|---|---|
| rhwp 본체 unit | EditOperation 12 신규 variants apply 동작 (정방향). 신설 3 native (replace_runs / replace_cell_runs / delete_range_in_cell_multipara) round-trip. apply_*_format_native 의 부분 업데이트 동작 검증 + 읽기 메서드 정합. |
| 서버 integration | 액션별 12 e2e — POST /workbench → core IR 변경 + sqlite stash 영속 + broadcast 수신. 부분 업데이트 검증 (일부 키만 보내고 나머지 유지). |
| 서버 endpoint | POST /undo (단순·체이닝·empty), GET /audit (범위), GET /diff, GET /ir-slice (mode별), workbench complete arm. |
| 클라 unit | onServerEvent 12 신규 op 분기별 wasm 호출 mock 검증. SnapshotRestored 핸들러. Complete 핸들러. |
| 양방향 e2e | 액션별 — POST → broadcast → 클라 wasm 호출 → 서버 IR 와 클라 wasm IR 일치 확인 (양쪽 export 후 텍스트 비교). |
| 회귀 | Sub-1 기존 ops 경로 + 기존 e2e ws-bridge 통과 보장. `apply_edit_ops_json("[{op}]")` 기존 4 variants 정합. |

## 12. Substage 분해 (6 단계)

| # | 범위 | 검증 게이트 |
|---|---|---|
| **2a** | rhwp 본체 native 신설 + WASM export 신설 — `replace_runs_native` / `replace_cell_runs_native` 2 신설 (둘 다 `runs: Vec<RunSpec>` 입력. *delete_range_in_cell_multipara 신설 불필요 — `delete_range_native(cell_ctx)` 가 다문단 셀 범위 삭제 이미 지원*). `wasm_api.rs` 에 대응 `replace_runs`, `replace_cell_runs` export 신설. *partial 동작 (`apply_para_format_native` / `set_cell_properties_native` / `apply_char_format_native` 가 JSON partial 직접 수용)* 은 코드 조사로 이미 확인 — 별도 작업 없음. | rhwp `cargo test` 신설 2 native round-trip + WASM export 단위 테스트. |
| **2b** | EditOperation *본문 7* — `ReplaceRuns` / `SetParagraphStyle` / `DeleteRange` / `InsertParagraph` / `DeleteElement` / `InsertTable` (+ 기존 `InsertText`). `PartialParagraphStyle` / `RunSpec` / `ElementType` 등 부분 타입 정의. `apply_edit_op` match arm (옵션 b). | rhwp `cargo test` 신규 variant apply + JSON 직렬화 round-trip. |
| **2c** | EditOperation *셀 5* — `SetCellStyle` / `MergeCells` / `ReplaceCellRuns` / `InsertTextInCell` / `DeleteRangeInCell`. `PartialCellStyle` 정의. `apply_edit_op` match arm. | rhwp `cargo test` 신규 variant apply + JSON 직렬화 round-trip. |
| **2d** | 서버 workbench 12 arms + sqlite snapshot stash + ServerEvent 2 신규 — `op_stash` schema + workbench handler 액션 12 arm (옵션 b 패턴: 검증 → save_snapshot → export_hwpx → apply → stash 영속 → broadcast Ops) + `ServerEvent::Complete` + `ServerEvent::SnapshotRestored` variant. | server `cargo test` 12 workbench integration + sqlite stash 영속 확인 + 100 entry 정책 검증. |
| **2e** | 신규 endpoint 4개 + complete arm — `POST /sessions/:id/undo` (stash pop + import_hwpx + broadcast SnapshotRestored) / `GET /sessions/:id/audit` / `GET /sessions/:id/diff` / `GET /sessions/:id/ir-slice` (raw/compact/auto) / workbench `complete` arm (export + final snapshot + broadcast Complete). | endpoint별 server `cargo test` integration + undo 체이닝. |
| **2f** | 클라 wasm-bridge wrapper 신설 + onServerEvent ops 12 분기 + SnapshotRestored / Complete 핸들러 + 양방향 e2e + 보고서 — `rhwp-studio/src/core/wasm-bridge.ts` 에 누락 wrapper 6개 추가 (`replaceRuns`·`applyParaFormat`·`deleteRange`·`insertParagraph`·`deleteElement`·`replaceCellRuns`. 나머지 6개 `createTable`·`setCellProperties`·`mergeTableCells`·`insertTextInCell`·`deleteTextInCell`·`applyCharFormat` 은 *코드 조사로 존재 확인됨*). `main.ts` onServerEvent 확장. SnapshotRestored 핸들러는 `wasm.fromBytes(blob)` 또는 *전체 wasm 재 mount*. Complete 는 `console.log` 만. e2e (액션별 12개 + undo + audit + diff + ir-slice). 회귀 검증 + stage2/report 작성. | 양방향 e2e 모두 통과 + Sub-1 기존 e2e ws-bridge 회귀 0. |

*시각 변경을 동반하는 substage 종결 시* sub-agent 시각 비교 의무 — 본 spec 의 경우 substage **2f** (클라 onServerEvent 분기 확장 + 양방향 e2e) 가 직접 시각 변경에 영향. 사용자 메모리 `[feedback_substage_visual_verification_mandatory]` 와 정합 (rdocx 영역의 룰이지만 시각 회귀 동반 substage 에 응용).

## 13. Definition of Done

1. 12 액션 모두 *서버가 진짜로 적용* 후 sqlite 에 영속. 새 접속자가 `/ir` 또는 `/export` 로 복원했을 때 *완전한 상태* 확인.
2. 신설 endpoint 4개 (POST /undo, GET /audit, GET /diff, GET /ir-slice) + `complete` workbench arm 모두 spec 대로 동작.
3. 부분 업데이트 (옵셔널 키 누락) 시 *현재 값 유지* 확인. 부분 업데이트 시연 e2e 1개 이상.
4. 자동화 테스트 — rhwp 본체 cargo test + server cargo test 모두 통과. Sub-1 기존 7 cargo test + e2e ws-bridge 회귀 0.
5. 양방향 e2e — 액션별 12 + undo + audit + diff + ir-slice + complete 모두 통과.
6. 수동 시연 시나리오 통과 — 새 노트북에서 12 액션 호출 + 브라우저 시각 반영 + undo 통째 복원.
7. broadcast 페이로드에 *정방향 EditOperation* 본문 포함 (디버깅 가시성 확보).
8. 본 spec 의 모든 인터페이스가 코드에 구현 + 회귀 0.

## 14. 알려진 한계 (Sub-3 으로 미룸)

- 다중 사용자 동시 편집 *사용자별 undo stack* 분리.
- snapshot binary 누적 *delta 압축*, *LRU* 정교화.
- *write-ahead log* — sqlite write 실패 시 코어 IR-sqlite 자동 회복.
- 옵션 B → 옵션 A (in-memory only) 다운그레이드 결정 (디스크 부담 평가 후).
- rhwp-studio 워크벤치 *UI 패널* — Sub-2 는 노트북 호출만.
- `apply_inverse_edit_op` 의 *EditOperation 기반 역연산* — 기존 4 variants 외 신규 12 는 *snapshot stash 만* 활용.
