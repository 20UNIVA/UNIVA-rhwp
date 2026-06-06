# Task #zephy-bridge Sub-2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** SKILL.md 12 액션 (`replace-runs`·`set-paragraph-style`·`delete-range`·`insert-paragraph`·`delete-element`·`insert-table`·`set-cell-style`·`merge-cells`·`replace-cell-runs`·`insert-text-in-cell`·`delete-range-in-cell` + `complete`) 을 *서버가 진짜로 적용* 후 sqlite 영속하도록 — *11+1 액션의 서버 SoT 완성*.

**Architecture:** 정방향만 `EditOperation` variants 신규 추가 (12개), 역방향은 sqlite `op_stash` 에 *호출 직전 `export_hwpx_native` 결과 binary blob* 영속. `POST /sessions/:id/undo` 시 `DocumentCore::from_bytes` 로 통째 교체 + `ServerEvent::SnapshotRestored` broadcast. broadcast 페이로드는 *정방향 EditOp 본문 그대로* — 디버깅 가시성 확보. 신규 endpoint 4개 (undo/audit/diff/ir-slice) + `complete` workbench arm 별도.

**Tech Stack:** Rust (rhwp 본체 + axum 서버), TypeScript (rhwp-studio), WebAssembly (rhwp 본체 → wasm export), sqlite (rusqlite bundled), WebSocket (axum::ws), base64 0.22.

**관련 문서:**
- 설계: [task_m200_zephy_bridge_sub2.md](task_m200_zephy_bridge_sub2.md)
- Sub-1 결과: [../report/task_m200_zephy_bridge_report.md](../report/task_m200_zephy_bridge_report.md)
- Sub-1 stage1: [../working/task_m200_zephy_bridge_stage1.md](../working/task_m200_zephy_bridge_stage1.md)

---

## Files Map — 변경 매트릭스

### Create

| 파일 | 역할 |
|---|---|
| `rhwp-studio/e2e/sub2-replace-runs.test.mjs` | replace_runs e2e |
| `rhwp-studio/e2e/sub2-set-paragraph-style.test.mjs` | set_paragraph_style e2e |
| `rhwp-studio/e2e/sub2-delete-range.test.mjs` | delete_range e2e |
| `rhwp-studio/e2e/sub2-insert-paragraph.test.mjs` | insert_paragraph e2e |
| `rhwp-studio/e2e/sub2-delete-element.test.mjs` | delete_element e2e |
| `rhwp-studio/e2e/sub2-insert-table.test.mjs` | insert_table e2e |
| `rhwp-studio/e2e/sub2-set-cell-style.test.mjs` | set_cell_style e2e |
| `rhwp-studio/e2e/sub2-merge-cells.test.mjs` | merge_cells e2e |
| `rhwp-studio/e2e/sub2-replace-cell-runs.test.mjs` | replace_cell_runs e2e |
| `rhwp-studio/e2e/sub2-insert-text-in-cell.test.mjs` | insert_text_in_cell e2e |
| `rhwp-studio/e2e/sub2-delete-range-in-cell.test.mjs` | delete_range_in_cell e2e |
| `rhwp-studio/e2e/sub2-undo.test.mjs` | undo e2e |
| `rhwp-studio/e2e/sub2-audit-diff-ir-slice.test.mjs` | audit·diff·ir-slice endpoint e2e |
| `mydocs/working/task_m200_zephy_bridge_sub2_stage1.md` | substage 결과 |
| `mydocs/report/task_m200_zephy_bridge_sub2_report.md` | Sub-2 최종 보고 |

### Modify

| 파일 | 변경 분량 추정 | 핵심 변경 |
|---|---|---|
| `src/document_core/commands/text_editing.rs` | +120 / -0 | `replace_runs_native`, `replace_cell_runs_native` 신설 |
| `src/wasm_api.rs` | +40 / -0 | WASM `replace_runs`, `replace_cell_runs` export |
| `src/document_core/commands/edit_op.rs` | +350 / -0 | 12 신규 variants + Partial 타입 + apply match arms |
| `server/src/events.rs` | +25 / -2 | rename_all snake_case + Complete/SnapshotRestored variant |
| `server/src/store.rs` | +120 / -0 | op_stash + final_snapshots 테이블 + 함수들 |
| `server/src/main.rs` | +400 / -10 | workbench 12 arms + 신규 endpoint 4개 + complete arm |
| `server/Cargo.toml` | +0 / -0 | (변경 없음 — base64·rusqlite 이미 있음) |
| `rhwp-studio/src/core/wasm-bridge.ts` | +90 / -0 | wrapper 메서드 6개 신설 |
| `rhwp-studio/src/main.ts` | +130 / -2 | onServerEvent ops 12 분기 + SnapshotRestored + Complete |
| `hwp_sub_agent_simulation_ssr.ipynb` (작업 공간 루트) | cell 3 정규화 매핑 확장 + 부분 업데이트 시연 셀 추가 | |

---

## Phase 2a: rhwp 본체 native + WASM export 신설

`replace_runs_native` / `replace_cell_runs_native` 2 신설 + 대응 WASM export 2개. `delete_range_native(cell_ctx)` 가 셀 다문단 범위 삭제를 *이미 지원* — 추가 native 불필요. `apply_para_format_native` / `set_cell_properties_native` / `apply_char_format_native` 가 *partial JSON 직접 수용* — 추가 작업 불필요.

### Task 2a.1: replace_runs_native 시그니처 + 빈 구현 + 실패 테스트

**Files:**
- Modify: `src/document_core/commands/text_editing.rs` (마지막 `impl DocumentCore` 블록 끝부분, 같은 파일의 `delete_range_native` 패턴 따름)

- [ ] **Step 1: 실패 테스트 작성** — `src/document_core/commands/text_editing.rs` 의 `#[cfg(test)] mod tests` 안 (이미 존재하는 테스트 모듈) 에 추가:

```rust
#[test]
fn test_replace_runs_native_basic() {
    let mut core = DocumentCore::new_empty();
    core.create_blank_document_native().unwrap();
    core.insert_text_native(0, 0, 0, "Hello World").unwrap();

    let runs_json = r#"[
        {"text": "Hi", "style": {"bold": true}},
        {"text": " there", "style": {}}
    ]"#;
    core.replace_runs_native(0, 0, runs_json).unwrap();

    assert_eq!(core.document.sections[0].paragraphs[0].text, "Hi there");
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test --lib test_replace_runs_native_basic -- --nocapture`
Expected: 컴파일 실패 — `replace_runs_native` 메서드 없음.

- [ ] **Step 3: 빈 구현 추가** — `src/document_core/commands/text_editing.rs` 의 `delete_range_native` 함수 *직후* (라인 약 720, 함수 끝 후 다음 빈 줄) 에 다음 추가:

```rust
    /// `(sec, para)` 본문 문단의 runs 를 *runs_json* 으로 통째 교체.
    ///
    /// runs_json 형식: `[{"text": "...", "style": {bold?, italic?, ...}}, ...]`.
    /// 기존 문단 내용은 모두 제거 후 새 runs 로 재구성. CharShape 는
    /// 각 run 의 style 을 [`apply_char_format_native`] 가 받아들이는
    /// CharShapeMods JSON 형식으로 해석.
    pub fn replace_runs_native(
        &mut self,
        section_idx: usize,
        para_idx: usize,
        runs_json: &str,
    ) -> Result<String, HwpError> {
        // 1. 기존 문단의 텍스트 길이 측정
        let para_len = self.document.sections[section_idx]
            .paragraphs[para_idx]
            .text
            .chars()
            .count();

        // 2. 기존 텍스트 통째 삭제
        if para_len > 0 {
            self.delete_text_native(section_idx, para_idx, 0, para_len)?;
        }

        // 3. runs_json 파싱
        let runs: Vec<serde_json::Value> = serde_json::from_str(runs_json)
            .map_err(|e| HwpError::InvalidFile(format!("runs_json 파싱 실패: {e}")))?;

        // 4. 각 run 순회 — 텍스트 삽입 후 char_format 적용
        let mut cursor = 0usize;
        for run in runs {
            let text = run.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if text.is_empty() {
                continue;
            }
            let len = text.chars().count();
            self.insert_text_native(section_idx, para_idx, cursor, text)?;

            // style 이 있고 빈 객체가 아닐 때만 char_format 적용
            if let Some(style) = run.get("style") {
                if style.is_object() && !style.as_object().unwrap().is_empty() {
                    let style_json = serde_json::to_string(style)
                        .map_err(|e| HwpError::InvalidFile(format!("style 직렬화: {e}")))?;
                    self.apply_char_format_native(
                        section_idx, para_idx, cursor, cursor + len, &style_json,
                    )?;
                }
            }

            cursor += len;
        }

        Ok(super::super::helpers::json_ok_with(&format!(
            "\"paraIdx\":{},\"runsCount\":{}",
            para_idx, cursor
        )))
    }
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test --lib test_replace_runs_native_basic -- --nocapture`
Expected: `test result: ok. 1 passed; 0 failed`

- [ ] **Step 5: 커밋**

```bash
git add src/document_core/commands/text_editing.rs
git commit -m "Task #zephy-bridge Sub-2 [2a.1]: replace_runs_native 신설

문단의 runs 를 통째 교체. 기존 텍스트 삭제 후 각 run 별 insert_text +
apply_char_format. style 빈 객체일 때는 char_format skip.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2a.2: WASM replace_runs export 신설

**Files:**
- Modify: `src/wasm_api.rs` (apply_para_format 부근 라인 4862 패턴 답습)

- [ ] **Step 1: 실패 테스트 작성** — `src/document_core/commands/text_editing.rs` 의 tests 모듈에 추가 (WASM export 는 JS-only 라 Rust 단위로는 underlying native 동일 검증):

```rust
#[test]
fn test_replace_runs_with_styled_runs() {
    let mut core = DocumentCore::new_empty();
    core.create_blank_document_native().unwrap();
    core.insert_text_native(0, 0, 0, "원본").unwrap();

    let runs_json = r#"[
        {"text": "굵게", "style": {"bold": true}},
        {"text": " 보통", "style": {}}
    ]"#;
    core.replace_runs_native(0, 0, runs_json).unwrap();

    let para = &core.document.sections[0].paragraphs[0];
    assert_eq!(para.text, "굵게 보통");
    // char_shapes 가 *2개 이상* 의 charShapeId 를 가져야 — 첫 번째 굵게 + 두 번째 보통
    assert!(para.char_shapes.len() >= 2, "최소 2개의 char_shape 구간이 있어야 함");
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test --lib test_replace_runs_with_styled_runs -- --nocapture`
Expected: PASS — 이미 2a.1 의 구현이 char_format 까지 적용하므로 통과 가능. 만약 FAIL 이면 *style 미적용 버그* — 디버깅.

- [ ] **Step 3: WASM export 추가** — `src/wasm_api.rs` 의 `apply_para_format` (라인 약 4862) *직후* 다음 추가:

```rust
    /// JS: replaceRuns(secIdx, paraIdx, runsJson) → ok JSON
    ///
    /// runs_json 형식: `[{"text": "...", "style": {bold?, italic?, ...}}, ...]`.
    #[wasm_bindgen(js_name = replaceRuns)]
    pub fn replace_runs(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        runs_json: &str,
    ) -> Result<String, JsValue> {
        self.replace_runs_native(section_idx as usize, para_idx as usize, runs_json)
            .map_err(|e| e.into())
    }
```

- [ ] **Step 4: WASM 빌드 확인**

Run: `cargo build --target wasm32-unknown-unknown --release --lib 2>&1 | tail -20`
Expected: 컴파일 성공. 새 export `replace_runs` 가 wasm 모듈에 노출.

- [ ] **Step 5: 커밋**

```bash
git add src/wasm_api.rs src/document_core/commands/text_editing.rs
git commit -m "Task #zephy-bridge Sub-2 [2a.2]: WASM replace_runs export 신설

JS 측에서 wasm.replaceRuns(sec, para, runsJson) 으로 호출 가능. 내부적으로
replace_runs_native 위임. 스타일 적용 검증 unit test 추가.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2a.3: replace_cell_runs_native 신설

**Files:**
- Modify: `src/document_core/commands/text_editing.rs` (replace_runs_native 직후)

- [ ] **Step 1: 실패 테스트 작성**

```rust
#[test]
fn test_replace_cell_runs_native_basic() {
    let mut core = DocumentCore::new_empty();
    core.create_blank_document_native().unwrap();
    // 1행 2열 표 생성
    core.create_table_native(0, 0, 0, 1, 2).unwrap();
    // table_para = 0, cell (0,0) 의 cell_para_idx = 0 에 텍스트 삽입
    let ctrl_idx = 0usize;
    let cell_idx = 0usize;
    core.insert_text_in_cell_native(0, 0, ctrl_idx, cell_idx, 0, 0, "원본").unwrap();

    let runs_json = r#"[
        {"text": "변경", "style": {"bold": true}}
    ]"#;
    core.replace_cell_runs_native(0, 0, ctrl_idx, cell_idx, 0, runs_json).unwrap();

    let cell = core.get_cell(0, 0, ctrl_idx, cell_idx).unwrap();
    assert_eq!(cell.paragraphs[0].text, "변경");
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test --lib test_replace_cell_runs_native_basic -- --nocapture`
Expected: 컴파일 실패 — `replace_cell_runs_native` / `get_cell` 메서드 없음 (`get_cell` 은 helper 인지 확인 — 없으면 `core.document.sections[0].paragraphs[0].controls[0]` 직접 접근으로 변경).

- [ ] **Step 3: 구현 추가** — `replace_runs_native` 함수 *직후*:

```rust
    /// 셀 내 문단의 runs 를 통째 교체. (sec, table_para, ctrl, cell, cell_para) 좌표.
    pub fn replace_cell_runs_native(
        &mut self,
        section_idx: usize,
        table_para_idx: usize,
        control_idx: usize,
        cell_idx: usize,
        cell_para_idx: usize,
        runs_json: &str,
    ) -> Result<String, HwpError> {
        // 1. 기존 셀 문단 텍스트 길이
        let para_len = {
            let cell_para = self.get_cell_paragraph(
                section_idx, table_para_idx, control_idx, cell_idx, cell_para_idx
            )?;
            cell_para.text.chars().count()
        };

        // 2. 기존 텍스트 통째 삭제
        if para_len > 0 {
            self.delete_text_in_cell_native(
                section_idx, table_para_idx, control_idx, cell_idx, cell_para_idx, 0, para_len,
            )?;
        }

        // 3. runs_json 파싱
        let runs: Vec<serde_json::Value> = serde_json::from_str(runs_json)
            .map_err(|e| HwpError::InvalidFile(format!("runs_json 파싱: {e}")))?;

        // 4. 각 run 순회
        let mut cursor = 0usize;
        for run in runs {
            let text = run.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if text.is_empty() {
                continue;
            }
            let len = text.chars().count();
            self.insert_text_in_cell_native(
                section_idx, table_para_idx, control_idx, cell_idx, cell_para_idx,
                cursor, text,
            )?;

            if let Some(style) = run.get("style") {
                if style.is_object() && !style.as_object().unwrap().is_empty() {
                    let style_json = serde_json::to_string(style)
                        .map_err(|e| HwpError::InvalidFile(format!("style 직렬화: {e}")))?;
                    self.apply_char_format_in_cell_native(
                        section_idx, table_para_idx, control_idx, cell_idx, cell_para_idx,
                        cursor, cursor + len, &style_json,
                    )?;
                }
            }

            cursor += len;
        }

        Ok(super::super::helpers::json_ok_with(&format!(
            "\"cellParaIdx\":{},\"runsCount\":{}",
            cell_para_idx, cursor
        )))
    }
```

*참고: `get_cell_paragraph` / `apply_char_format_in_cell_native` 가 이미 존재하는지 확인 — text_editing.rs / formatting.rs 의 `pub fn get_cell_paragraph` / `apply_char_format_in_cell_native` grep 으로 검증. 없으면 *해당 helper 도 신설 task* 로 분리.*

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test --lib test_replace_cell_runs_native_basic -- --nocapture`
Expected: PASS

- [ ] **Step 5: 커밋**

```bash
git add src/document_core/commands/text_editing.rs
git commit -m "Task #zephy-bridge Sub-2 [2a.3]: replace_cell_runs_native 신설

셀 내 문단의 runs 통째 교체. replace_runs_native 의 셀 변형.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2a.4: WASM replace_cell_runs export 신설

**Files:**
- Modify: `src/wasm_api.rs` (replace_runs export 직후 또는 insert_text_in_cell 부근 라인 646)

- [ ] **Step 1: WASM export 추가**

```rust
    /// JS: replaceCellRuns(secIdx, tableParaIdx, ctrlIdx, cellIdx, cellParaIdx, runsJson)
    #[wasm_bindgen(js_name = replaceCellRuns)]
    pub fn replace_cell_runs(
        &mut self,
        section_idx: u32,
        table_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        runs_json: &str,
    ) -> Result<String, JsValue> {
        self.replace_cell_runs_native(
            section_idx as usize,
            table_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
            runs_json,
        )
        .map_err(|e| e.into())
    }
```

- [ ] **Step 2: WASM 빌드 확인**

Run: `cargo build --target wasm32-unknown-unknown --release --lib 2>&1 | tail -10`
Expected: 컴파일 성공.

- [ ] **Step 3: 커밋**

```bash
git add src/wasm_api.rs
git commit -m "Task #zephy-bridge Sub-2 [2a.4]: WASM replace_cell_runs export 신설

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2a.5: Phase 2a 회귀 검증

- [ ] **Step 1: 전체 cargo test 실행**

Run: `cargo test --lib 2>&1 | tail -30`
Expected: 모든 기존 + 신규 unit test 통과. 신규 test 들 (`test_replace_runs_native_basic`, `test_replace_runs_with_styled_runs`, `test_replace_cell_runs_native_basic`) 통과 명시.

- [ ] **Step 2: cargo clippy 통과 확인**

Run: `cargo clippy --lib -- -D warnings 2>&1 | tail -20`
Expected: warning 0.

- [ ] **Step 3: WASM 빌드 통과 확인**

Run: `cargo build --target wasm32-unknown-unknown --release --lib 2>&1 | tail -5`
Expected: 컴파일 성공.

- [ ] **Step 4: 회귀 0 명시 commit (변경 없으면 commit 안 함, 변경 있으면 fixup 추가)**

```bash
git status   # 깨끗한지 확인
```

---

## Phase 2b: EditOperation 본문 7 + Partial 타입

`EditOperation` enum 에 본문 7 신규 variants 추가 — `ReplaceRuns`·`SetParagraphStyle`·`DeleteRange`·`InsertParagraph`·`DeleteElement`·`InsertTable` (+ 기존 `InsertText`). `PartialParagraphStyle`·`RunSpec`·`ElementType` 타입 정의. `apply_edit_op` match arm 추가.

### Task 2b.1: Partial 타입 + RunSpec + ElementType 정의

**Files:**
- Modify: `src/document_core/commands/edit_op.rs` (라인 19 위 `use` block 직후, enum 선언 전)

- [ ] **Step 1: 실패 테스트 작성** — edit_op.rs 의 `mod tests` 내:

```rust
#[test]
fn test_partial_paragraph_style_serialize_skip_none() {
    let partial = PartialParagraphStyle {
        alignment: Some("right".to_string()),
        line_spacing: None,
        margin_left: None,
        margin_right: None,
        indent: None,
        spacing_before: None,
        spacing_after: None,
    };
    let json = serde_json::to_string(&partial).unwrap();
    assert_eq!(json, r#"{"alignment":"right"}"#);
}

#[test]
fn test_run_spec_deserialize() {
    let json = r#"{"text":"안녕","style":{"bold":true}}"#;
    let run: RunSpec = serde_json::from_str(json).unwrap();
    assert_eq!(run.text, "안녕");
    assert!(run.style.is_some());
}

#[test]
fn test_element_type_serialize() {
    assert_eq!(serde_json::to_string(&ElementType::Paragraph).unwrap(), r#""paragraph""#);
    assert_eq!(serde_json::to_string(&ElementType::Table).unwrap(), r#""table""#);
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test --lib edit_op::tests::test_partial_paragraph_style_serialize_skip_none -- --nocapture`
Expected: 컴파일 실패 — `PartialParagraphStyle` 없음.

- [ ] **Step 3: Partial 타입들 정의** — `src/document_core/commands/edit_op.rs` 의 `use serde::{...};` 직후 (라인 약 16) 에 추가:

```rust
// ─── Sub-2: Partial 타입 (옵셔널 필드만 직렬화) ─────────────────

/// 본문 문단의 부분 스타일. None 인 필드는 *현재 값 유지* 의미.
/// JSON 직렬화 시 None 은 제외 (`skip_serializing_if`).
/// 직접 `apply_para_format_native(props_json)` 의 입력으로 사용 가능.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PartialParagraphStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alignment: Option<String>,   // "left"|"right"|"center"|"justify"|...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_before: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_after: Option<i16>,
}

/// 셀의 부분 스타일. None 인 필드는 *현재 값 유지*.
/// `set_cell_properties_native(json)` 의 입력으로 사용 가능.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PartialCellStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,   // "top"|"middle"|"bottom"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_fill_id: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_header: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_protect: Option<bool>,
    // padding/text_direction 은 Sub-3 에서 추가
}

/// run 의 부분 char 스타일. None 인 필드 유지.
/// `apply_char_format_native(props_json)` 입력으로 사용 가능.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PartialRunStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_size: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
}

/// run = 텍스트 한 조각 + (선택) 부분 스타일.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunSpec {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<PartialRunStyle>,
}

/// delete-element 의 element_type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElementType {
    Paragraph,
    Table,
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test --lib edit_op::tests -- --nocapture`  *(prefix filter — cargo test 는 *단일 positional argument* 만 허용. `edit_op::tests` 가 신규 3 함수 모두 매치.)*
Expected: 3 PASS.

- [ ] **Step 5: 커밋**

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2b.1]: Partial 타입 + RunSpec + ElementType 정의

PartialParagraphStyle / PartialCellStyle / PartialRunStyle — None 필드
skip_serializing. RunSpec / ElementType. native partial JSON 호환.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2b.2: EditOperation::ReplaceRuns + apply

**Files:**
- Modify: `src/document_core/commands/edit_op.rs` (enum 본문 끝 `MergeParagraph` 다음 + match arm)

- [ ] **Step 1: 실패 테스트**

```rust
#[test]
fn test_replace_runs_op_apply() {
    let mut core = core_with_text("원본");
    let op = EditOperation::ReplaceRuns {
        section: 0,
        para: 0,
        runs: vec![
            RunSpec { text: "변경".to_string(), style: Some(PartialRunStyle { bold: Some(true), ..Default::default() }) },
            RunSpec { text: " 보통".to_string(), style: None },
        ],
    };
    core.apply_edit_op(&op).unwrap();
    assert_eq!(para_text(&core, 0, 0), "변경 보통");
}

#[test]
fn test_replace_runs_op_json_roundtrip() {
    let json = r#"{"op":"replace_runs","section":0,"para":3,"runs":[{"text":"hi","style":{"bold":true}}]}"#;
    let op: EditOperation = serde_json::from_str(json).unwrap();
    assert!(matches!(op, EditOperation::ReplaceRuns { section: 0, para: 3, .. }));
    let back = serde_json::to_string(&op).unwrap();
    let op2: EditOperation = serde_json::from_str(&back).unwrap();
    assert_eq!(op, op2);
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib edit_op::tests::test_replace_runs_op_apply -- --nocapture`
Expected: 컴파일 실패.

- [ ] **Step 3: enum variant 추가** — `EditOperation` enum 의 `MergeParagraph` 다음 (라인 약 55) 에 추가:

```rust
    // ─── Sub-2: 신규 12 variants (정방향만, inverse 는 sqlite snapshot stash) ───

    /// 문단 내 runs 를 통째 교체.
    ReplaceRuns {
        section: usize,
        para: usize,
        runs: Vec<RunSpec>,
    },
```

- [ ] **Step 4: apply_edit_op match arm 추가** — `apply_edit_op` 의 `MergeParagraph { ... }` arm 다음:

```rust
            EditOperation::ReplaceRuns { section, para, runs } => {
                let runs_json = serde_json::to_string(runs)
                    .map_err(|e| HwpError::RenderError(format!("runs 직렬화: {e}")))?;
                self.replace_runs_native(*section, *para, &runs_json)?;
            }
```

- [ ] **Step 5: apply_inverse_edit_op match arm 추가**

```rust
            EditOperation::ReplaceRuns { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
```

- [ ] **Step 6: 테스트 통과**

Run: `cargo test --lib edit_op::tests::test_replace_runs_op -- --nocapture`  *(prefix `test_replace_runs_op` 가 _apply 와 _json_roundtrip 두 함수 모두 매치)*
Expected: 2 PASS.

- [ ] **Step 7: 커밋**

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2b.2]: EditOperation::ReplaceRuns + apply

inverse 는 snapshot stash 위임. JSON tag 'replace_runs'.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2b.3: EditOperation::SetParagraphStyle + apply

- [ ] **Step 1: 실패 테스트**

```rust
#[test]
fn test_set_paragraph_style_op_apply_partial() {
    let mut core = core_with_text("hello");
    let op = EditOperation::SetParagraphStyle {
        section: 0,
        para: 0,
        style: PartialParagraphStyle {
            alignment: Some("right".to_string()),
            ..Default::default()
        },
    };
    core.apply_edit_op(&op).unwrap();
    // alignment 만 변경 — 다른 필드 (line_spacing 등) 는 유지
    // 확인은 get_para_properties_at_native 결과 JSON 으로
    let result = core.get_para_properties_at_native(0, 0).unwrap();
    assert!(result.contains(r#""alignment":"right""#));
}

#[test]
fn test_set_paragraph_style_op_json() {
    let json = r#"{"op":"set_paragraph_style","section":0,"para":0,"style":{"alignment":"center"}}"#;
    let op: EditOperation = serde_json::from_str(json).unwrap();
    assert!(matches!(op, EditOperation::SetParagraphStyle { section: 0, para: 0, .. }));
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib edit_op::tests::test_set_paragraph_style_op_apply_partial -- --nocapture`
Expected: 컴파일 실패.

- [ ] **Step 3: variant + arm 추가**

variant (enum 안):
```rust
    SetParagraphStyle {
        section: usize,
        para: usize,
        style: PartialParagraphStyle,
    },
```

apply arm:
```rust
            EditOperation::SetParagraphStyle { section, para, style } => {
                let props_json = serde_json::to_string(style)
                    .map_err(|e| HwpError::RenderError(format!("style 직렬화: {e}")))?;
                self.apply_para_format_native(*section, *para, &props_json)?;
            }
```

inverse arm:
```rust
            EditOperation::SetParagraphStyle { .. } => {
                unreachable!("Sub-2 variants use snapshot stash for inverse");
            }
```

- [ ] **Step 4: 테스트 통과**

Run: `cargo test --lib edit_op::tests::test_set_paragraph_style_op -- --nocapture`  *(prefix 매치 두 함수)*
Expected: 2 PASS.

- [ ] **Step 5: 커밋**

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2b.3]: EditOperation::SetParagraphStyle + apply

PartialParagraphStyle 직렬화 → apply_para_format_native partial JSON 직접.
부분 업데이트 동작 검증 unit test.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2b.4: EditOperation::DeleteRange + apply

- [ ] **Step 1: 실패 테스트**

```rust
#[test]
fn test_delete_range_op_apply_same_para() {
    let mut core = core_with_text("ABCDE");
    let op = EditOperation::DeleteRange {
        section: 0, para_start: 0, char_start: 1, para_end: 0, char_end: 3,
    };
    core.apply_edit_op(&op).unwrap();
    assert_eq!(para_text(&core, 0, 0), "ADE");
}

#[test]
fn test_delete_range_op_apply_multi_para() {
    let mut core = core_with_text("AAA");
    // 두 번째 문단 추가
    core.apply_edit_op(&EditOperation::SplitParagraph { section: 0, para: 0, offset: 3 }).unwrap();
    core.insert_text_native(0, 1, 0, "BBB").unwrap();
    // 첫 문단 끝부터 두번째 문단 중간까지 삭제
    let op = EditOperation::DeleteRange {
        section: 0, para_start: 0, char_start: 2, para_end: 1, char_end: 2,
    };
    core.apply_edit_op(&op).unwrap();
    // 결과: "AA" + "B" = "AAB" 한 문단
    assert_eq!(core.document.sections[0].paragraphs.len(), 1);
    assert_eq!(para_text(&core, 0, 0), "AAB");
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib edit_op::tests::test_delete_range_op_apply -- --nocapture`
Expected: 컴파일 실패.

- [ ] **Step 3: variant + arm 추가**

variant:
```rust
    DeleteRange {
        section: usize,
        para_start: usize,
        char_start: usize,
        para_end: usize,
        char_end: usize,
    },
```

apply arm:
```rust
            EditOperation::DeleteRange { section, para_start, char_start, para_end, char_end } => {
                self.delete_range_native(*section, *para_start, *char_start, *para_end, *char_end, None)?;
            }
```

inverse arm: `unreachable!` 동일 패턴.

- [ ] **Step 4: 테스트 통과**

Run: `cargo test --lib edit_op::tests::test_delete_range_op_apply -- --nocapture`  *(prefix 매치)*
Expected: 2 PASS.

- [ ] **Step 5: 커밋**

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2b.4]: EditOperation::DeleteRange + apply

delete_range_native(cell_ctx=None) 위임. 동문단·다문단 모두 통과.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2b.5: EditOperation::InsertParagraph + apply

- [ ] **Step 1: 실패 테스트**

```rust
#[test]
fn test_insert_paragraph_op_apply() {
    let mut core = core_with_text("first");
    let op = EditOperation::InsertParagraph {
        section: 0,
        after_para: 0,
        count: 1,
        style: None,
    };
    core.apply_edit_op(&op).unwrap();
    assert_eq!(core.document.sections[0].paragraphs.len(), 2);
}

#[test]
fn test_insert_paragraph_op_default_count() {
    let json = r#"{"op":"insert_paragraph","section":0,"after_para":0}"#;
    let op: EditOperation = serde_json::from_str(json).unwrap();
    if let EditOperation::InsertParagraph { count, .. } = op {
        assert_eq!(count, 1);
    } else {
        panic!("Wrong variant");
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib edit_op::tests::test_insert_paragraph_op_apply -- --nocapture`
Expected: 컴파일 실패.

- [ ] **Step 3: variant + arm**

variant:
```rust
    InsertParagraph {
        section: usize,
        after_para: usize,
        #[serde(default = "one_count")]
        count: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        style: Option<PartialParagraphStyle>,
    },
```

`one_count` helper 함수 (파일 끝, tests 앞):
```rust
fn one_count() -> usize { 1 }
```

apply arm:
```rust
            EditOperation::InsertParagraph { section, after_para, count, style } => {
                // count 회 반복 insert_paragraph_native — insert_paragraph_native(sec, after_para) 가 *after_para 다음에* 빈 문단 1개 삽입
                for i in 0..*count {
                    self.insert_paragraph_native(*section, *after_para + i)?;
                    // 각 신규 문단에 style 적용 (있을 때)
                    if let Some(s) = style {
                        let props_json = serde_json::to_string(s)
                            .map_err(|e| HwpError::RenderError(format!("style 직렬화: {e}")))?;
                        self.apply_para_format_native(*section, *after_para + i + 1, &props_json)?;
                    }
                }
            }
```

inverse arm: `unreachable!`.

- [ ] **Step 4: 테스트 통과**

Run: `cargo test --lib edit_op::tests::test_insert_paragraph_op -- --nocapture`  *(prefix 매치)*
Expected: 2 PASS.

- [ ] **Step 5: 커밋**

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2b.5]: EditOperation::InsertParagraph + apply

count default 1. 옵셔널 style 은 각 신규 문단에 apply_para_format.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2b.6: EditOperation::DeleteElement + apply

- [ ] **Step 1: 실패 테스트**

```rust
#[test]
fn test_delete_element_op_apply_paragraph() {
    let mut core = core_with_text("first");
    core.apply_edit_op(&EditOperation::SplitParagraph { section: 0, para: 0, offset: 5 }).unwrap();
    core.insert_text_native(0, 1, 0, "second").unwrap();
    // 두 문단 — 첫번째 삭제
    let op = EditOperation::DeleteElement {
        section: 0, para: 0, element_type: ElementType::Paragraph,
    };
    core.apply_edit_op(&op).unwrap();
    assert_eq!(core.document.sections[0].paragraphs.len(), 1);
    assert_eq!(para_text(&core, 0, 0), "second");
}

#[test]
fn test_delete_element_op_apply_table() {
    let mut core = core_with_text("");
    core.create_table_native(0, 0, 0, 2, 3).unwrap();
    // table_para = 0 (table 이 들어간 문단)
    let op = EditOperation::DeleteElement {
        section: 0, para: 0, element_type: ElementType::Table,
    };
    core.apply_edit_op(&op).unwrap();
    // table 컨트롤이 사라져야 함 — 검증 방식은 paragraph.controls 가 비었는지
    let para = &core.document.sections[0].paragraphs[0];
    assert!(para.controls.is_empty() || !para.controls.iter().any(|c| matches!(c, crate::model::Control::Table(_))));
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib edit_op::tests::test_delete_element_op_apply_paragraph -- --nocapture`
Expected: 컴파일 실패.

- [ ] **Step 3: variant + arm**

variant:
```rust
    DeleteElement {
        section: usize,
        para: usize,
        element_type: ElementType,
    },
```

apply arm:
```rust
            EditOperation::DeleteElement { section, para, element_type } => {
                match element_type {
                    ElementType::Paragraph => {
                        self.delete_paragraph_native(*section, *para)?;
                    }
                    ElementType::Table => {
                        // delete_table_control_native 는 table_ops.rs:1596
                        // 시그니처: (sec, para_idx) — 해당 문단의 첫 table control 삭제 (확인 필요)
                        self.delete_table_control_native(*section, *para)?;
                    }
                }
            }
```

*주의: `delete_table_control_native` 의 정확한 시그니처는 `table_ops.rs:1596` 코드 직접 read 로 확인 — 인자가 (sec, para) 만일 수도, 추가 ctrl_idx 필요할 수도. 다를 경우 spec 보정 (element_type 에 control_idx 추가) 또는 첫 table control 자동 검색 로직 추가.*

inverse arm: `unreachable!`.

- [ ] **Step 4: 테스트 통과**

Run: `cargo test --lib edit_op::tests::test_delete_element_op_apply -- --nocapture`  *(prefix 매치)*
Expected: 2 PASS.

- [ ] **Step 5: 커밋**

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2b.6]: EditOperation::DeleteElement + apply

element_type 분기 (Paragraph → delete_paragraph_native, Table → delete_table_control_native).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2b.7: EditOperation::InsertTable + apply

- [ ] **Step 1: 실패 테스트**

```rust
#[test]
fn test_insert_table_op_apply() {
    let mut core = core_with_text("hello");
    let op = EditOperation::InsertTable {
        section: 0,
        insert_after_para: 0,
        rows: 2,
        cols: 3,
    };
    core.apply_edit_op(&op).unwrap();
    // 신규 table 이 어딘가에 — controls 에 Table 1개 이상
    let has_table = core.document.sections[0].paragraphs.iter().any(|p| {
        p.controls.iter().any(|c| matches!(c, crate::model::Control::Table(_)))
    });
    assert!(has_table, "Table 컨트롤이 삽입되어야 함");
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib edit_op::tests::test_insert_table_op_apply -- --nocapture`
Expected: 컴파일 실패.

- [ ] **Step 3: variant + arm**

variant:
```rust
    InsertTable {
        section: usize,
        insert_after_para: usize,
        rows: u16,
        cols: u16,
    },
```

apply arm:
```rust
            EditOperation::InsertTable { section, insert_after_para, rows, cols } => {
                // create_table_native 시그니처: (sec, para_idx, char_offset, rows, cols)
                // insert_after_para 의 의미 — *해당 문단 끝에 표 삽입*. char_offset = 문단 길이.
                let para_len = self.document.sections[*section]
                    .paragraphs[*insert_after_para]
                    .text
                    .chars()
                    .count();
                self.create_table_native(*section, *insert_after_para, para_len, *rows, *cols)?;
            }
```

inverse arm: `unreachable!`.

- [ ] **Step 4: 테스트 통과**

Run: `cargo test --lib edit_op::tests::test_insert_table_op_apply -- --nocapture`
Expected: PASS.

- [ ] **Step 5: 커밋**

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2b.7]: EditOperation::InsertTable + apply

insert_after_para 의 끝(char_offset = para_len)에 create_table_native 호출.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2b.8: Phase 2b 회귀

- [ ] **Step 1: 전체 unit test 통과**

Run: `cargo test --lib 2>&1 | tail -30`
Expected: 모든 통과 — Phase 2b 신규 unit test 7개 (`test_replace_runs_op_apply`, `test_set_paragraph_style_op_apply_partial`, `test_delete_range_op_apply_same_para`, `test_delete_range_op_apply_multi_para`, `test_insert_paragraph_op_apply`, `test_delete_element_op_apply_paragraph`, `test_delete_element_op_apply_table`, `test_insert_table_op_apply`) + 기존 통과.

- [ ] **Step 2: clippy**

Run: `cargo clippy --lib -- -D warnings 2>&1 | tail -10`
Expected: warning 0.

---

## Phase 2c: EditOperation 셀 5

`SetCellStyle`·`MergeCells`·`ReplaceCellRuns`·`InsertTextInCell`·`DeleteRangeInCell` 5 신규 variants. Phase 2b 패턴 동일.

### Task 2c.1: EditOperation::SetCellStyle + apply

- [ ] **Step 1: 실패 테스트**

```rust
#[test]
fn test_set_cell_style_op_apply() {
    let mut core = core_with_text("");
    core.create_table_native(0, 0, 0, 2, 2).unwrap();
    // table_para = 0, control_idx = 0, cell_idx = 0
    let op = EditOperation::SetCellStyle {
        section: 0,
        table_para: 0,
        row: 0,
        col: 0,
        style: PartialCellStyle {
            vertical_align: Some("middle".to_string()),
            ..Default::default()
        },
    };
    core.apply_edit_op(&op).unwrap();
    // 검증 — 첫 셀의 vertical_align 이 middle 로
    // 검증 방식은 코드 조사 후 결정 (Table::cells[0].vertical_align 또는 helper)
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib edit_op::tests::test_set_cell_style_op_apply -- --nocapture`
Expected: 컴파일 실패.

- [ ] **Step 3: variant + arm**

variant:
```rust
    SetCellStyle {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        style: PartialCellStyle,
    },
```

apply arm:
```rust
            EditOperation::SetCellStyle { section, table_para, row, col, style } => {
                // 1. (row, col) 을 cell_idx 로 변환 — table.cells[i].row == row && .col == col 인 i 검색
                let ctrl_idx = 0usize;  // 첫 table control 가정 (Sub-3 에서 정교화)
                let cell_idx = self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?;
                let json = serde_json::to_string(style)
                    .map_err(|e| HwpError::RenderError(format!("style 직렬화: {e}")))?;
                self.set_cell_properties_native(*section, *table_para, ctrl_idx, cell_idx, &json)?;
            }
```

*`find_cell_idx` helper 가 없다면 신설 task 분리. 또는 직접 `table.cells` 순회.*

inverse arm: `unreachable!`.

- [ ] **Step 4: 테스트 통과**

Run: `cargo test --lib edit_op::tests::test_set_cell_style_op_apply -- --nocapture`
Expected: PASS.

- [ ] **Step 5: 커밋**

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2c.1]: EditOperation::SetCellStyle + apply

PartialCellStyle 직렬화 → set_cell_properties_native partial JSON.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2c.2: EditOperation::MergeCells + apply

- [ ] **Step 1: 실패 테스트**

```rust
#[test]
fn test_merge_cells_op_apply() {
    let mut core = core_with_text("");
    core.create_table_native(0, 0, 0, 3, 3).unwrap();
    let op = EditOperation::MergeCells {
        section: 0,
        table_para: 0,
        row_start: 0,
        col_start: 0,
        row_end: 0,
        col_end: 1,
    };
    core.apply_edit_op(&op).unwrap();
    // 검증 — table.cells 가 9개에서 8개로 (한 셀 병합으로 1개 줄어듦)
}
```

- [ ] **Step 2: 실패 확인 + variant + arm**

variant:
```rust
    MergeCells {
        section: usize,
        table_para: usize,
        row_start: usize,
        col_start: usize,
        row_end: usize,
        col_end: usize,
    },
```

apply arm:
```rust
            EditOperation::MergeCells { section, table_para, row_start, col_start, row_end, col_end } => {
                let ctrl_idx = 0usize;
                self.merge_table_cells_native(
                    *section, *table_para, ctrl_idx,
                    *row_start as u16, *col_start as u16,
                    *row_end as u16, *col_end as u16,
                )?;
            }
```

- [ ] **Step 3: 테스트 통과 + 커밋**

Run: `cargo test --lib edit_op::tests::test_merge_cells_op_apply`
Expected: PASS.

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2c.2]: EditOperation::MergeCells + apply

merge_table_cells_native(ctrl_idx=0) 위임.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2c.3: EditOperation::ReplaceCellRuns + apply

variant:
```rust
    ReplaceCellRuns {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        cell_para: usize,
        runs: Vec<RunSpec>,
    },
```

apply arm:
```rust
            EditOperation::ReplaceCellRuns { section, table_para, row, col, cell_para, runs } => {
                let ctrl_idx = 0usize;
                let cell_idx = self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?;
                let runs_json = serde_json::to_string(runs)
                    .map_err(|e| HwpError::RenderError(format!("runs 직렬화: {e}")))?;
                self.replace_cell_runs_native(*section, *table_para, ctrl_idx, cell_idx, *cell_para, &runs_json)?;
            }
```

- [ ] **Step 1-3: 테스트 + 구현 + 커밋** (위 패턴 동일)

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2c.3]: EditOperation::ReplaceCellRuns + apply

replace_cell_runs_native 위임. (row,col) → cell_idx 변환.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2c.4: EditOperation::InsertTextInCell + apply

variant:
```rust
    InsertTextInCell {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        cell_para: usize,
        offset: usize,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        style: Option<PartialRunStyle>,
    },
```

apply arm:
```rust
            EditOperation::InsertTextInCell { section, table_para, row, col, cell_para, offset, text, style } => {
                let ctrl_idx = 0usize;
                let cell_idx = self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?;
                let text_len = text.chars().count();
                self.insert_text_in_cell_native(
                    *section, *table_para, ctrl_idx, cell_idx, *cell_para, *offset, text,
                )?;
                if let Some(s) = style {
                    let json = serde_json::to_string(s)
                        .map_err(|e| HwpError::RenderError(format!("style 직렬화: {e}")))?;
                    self.apply_char_format_in_cell_native(
                        *section, *table_para, ctrl_idx, cell_idx, *cell_para,
                        *offset, *offset + text_len, &json,
                    )?;
                }
            }
```

- [ ] 테스트 + 커밋:

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2c.4]: EditOperation::InsertTextInCell + apply

옵셔널 style 시 apply_char_format_in_cell_native 추가.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2c.5: EditOperation::DeleteRangeInCell + apply

variant:
```rust
    DeleteRangeInCell {
        section: usize,
        table_para: usize,
        row: usize,
        col: usize,
        cell_para_start: usize,
        char_start: usize,
        cell_para_end: usize,
        char_end: usize,
    },
```

apply arm:
```rust
            EditOperation::DeleteRangeInCell { section, table_para, row, col, cell_para_start, char_start, cell_para_end, char_end } => {
                let ctrl_idx = 0usize;
                let cell_idx = self.find_cell_idx(*section, *table_para, ctrl_idx, *row as u16, *col as u16)?;
                // delete_range_native 의 cell_ctx 활용 — 다문단 셀 범위 삭제 지원
                self.delete_range_native(
                    *section, *cell_para_start, *char_start, *cell_para_end, *char_end,
                    Some((*table_para, ctrl_idx, cell_idx)),
                )?;
            }
```

- [ ] 테스트 + 커밋:

```bash
git add src/document_core/commands/edit_op.rs
git commit -m "Task #zephy-bridge Sub-2 [2c.5]: EditOperation::DeleteRangeInCell + apply

delete_range_native(cell_ctx=Some((tp, ctrl, cell))) 활용 — 다문단 셀 범위
삭제는 native 가 이미 지원.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2c.6: Phase 2c 회귀

- [ ] cargo test --lib + clippy 통과 확인. 위 Phase 2a/2b 와 동일 패턴.

---

## Phase 2d: 서버 workbench 12 arms + sqlite op_stash + ServerEvent 신규

### Task 2d.1: events.rs rename_all snake_case 변경 + ServerEvent::Complete / SnapshotRestored 추가

**Files:**
- Modify: `server/src/events.rs` (라인 16, 33)

- [ ] **Step 1: 실패 테스트** — `server/src/events.rs` 의 `mod tests` 내:

```rust
    #[test]
    fn server_event_complete_serializes_with_snake_case() {
        let ev = ServerEvent::Complete { seq: 42 };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains(r#""kind":"complete""#));
        assert!(json.contains(r#""seq":42"#));
    }

    #[test]
    fn server_event_snapshot_restored_serializes_with_snake_case() {
        let ev = ServerEvent::SnapshotRestored {
            seq: 7,
            snapshot_base64: "AAAA".to_string(),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains(r#""kind":"snapshot_restored""#));
    }

    #[test]
    fn server_event_ops_still_lowercase_compat() {
        // Sub-1 의 기존 ops JSON 호환성
        let ev = ServerEvent::Ops { seq: 1, ops: vec![] };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains(r#""kind":"ops""#));
    }
```

- [ ] **Step 2: 실패 확인**

Run: `cd server && cargo test events::tests::server_event_snapshot_restored_serializes_with_snake_case`
Expected: 컴파일 실패 — variant 없음.

- [ ] **Step 3: events.rs 수정** — 라인 16: `lowercase` → `snake_case`. enum 본문에 variants 추가:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerEvent {
    Ops { seq: i64, ops: Vec<serde_json::Value> },
    Workbench { seq: i64, action: String, payload: serde_json::Value },
    /// Sub-2: 워크벤치 종료. 다른 탭에 알림.
    Complete { seq: i64 },
    /// Sub-2: undo 등으로 서버가 전체 스냅샷 복원. 클라는 wasm 통째 교체.
    SnapshotRestored { seq: i64, snapshot_base64: String },
}
```

ClientMessage 도 동일하게 `snake_case` 로 변경 (라인 33). 기존 `Ops`/`Snapshot`/`Ping` 은 단일 단어라 호환.

- [ ] **Step 4: 테스트 통과**

Run: `cd server && cargo test events::tests`
Expected: 4 PASS (기존 4 + 신규 3 = 7).

- [ ] **Step 5: 커밋**

```bash
git add server/src/events.rs
git commit -m "Task #zephy-bridge Sub-2 [2d.1]: ServerEvent::Complete / SnapshotRestored + rename_all snake_case

rename_all 을 lowercase → snake_case 로 변경. 기존 Ops·Workbench JSON 은 동일
(단일 단어). SnapshotRestored 가 'snapshot_restored' 로 직렬화되도록.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2d.2: sqlite op_stash 테이블 + 기본 함수 신설

**Files:**
- Modify: `server/src/store.rs` (라인 34-52 CREATE TABLE block 뒤에 추가)

- [ ] **Step 1: 실패 테스트** — `server/src/store.rs` 끝 `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn test_op_stash_append_and_pop() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("x.db").to_str().unwrap()).unwrap();
        store.create_session("file-1", "hwpx", &[]).unwrap();

        store.append_op_stash("file-1", 1, r#"{"op":"insert_text"}"#, b"BLOBA").unwrap();
        store.append_op_stash("file-1", 2, r#"{"op":"insert_text"}"#, b"BLOBB").unwrap();

        let popped = store.pop_op_stash("file-1").unwrap().unwrap();
        assert_eq!(popped.seq, 2);
        assert_eq!(popped.before_blob, b"BLOBB");

        let popped2 = store.pop_op_stash("file-1").unwrap().unwrap();
        assert_eq!(popped2.seq, 1);

        let empty = store.pop_op_stash("file-1").unwrap();
        assert!(empty.is_none());
    }

    #[test]
    fn test_op_stash_100_entry_limit_per_session() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("x.db").to_str().unwrap()).unwrap();
        store.create_session("file-A", "hwpx", &[]).unwrap();
        for i in 1..=105 {
            store.append_op_stash("file-A", i, r#"{}"#, &[]).unwrap();
        }
        let count = store.count_op_stash("file-A").unwrap();
        assert_eq!(count, 100, "세션당 마지막 100개만 보관");
    }

    #[test]
    fn test_op_stash_list_range() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("x.db").to_str().unwrap()).unwrap();
        store.create_session("file-2", "hwpx", &[]).unwrap();
        for i in 1..=10 {
            let op_json = format!(r#"{{"op":"test","seq":{}}}"#, i);
            store.append_op_stash("file-2", i, &op_json, &[]).unwrap();
        }
        let rows = store.list_op_stash_range("file-2", 3, 7).unwrap();
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].seq, 3);
        assert_eq!(rows[4].seq, 7);
    }
```

- [ ] **Step 2: 실패 확인**

Run: `cd server && cargo test store::tests::test_op_stash_append_and_pop`
Expected: 컴파일 실패.

- [ ] **Step 3: store.rs 수정** — CREATE TABLE 블록 (라인 52 닫기 `)?;` 직전) 에 추가:

```rust
     CREATE TABLE IF NOT EXISTS op_stash (
        seq         INTEGER NOT NULL,
        file_id     TEXT NOT NULL,
        op_json     TEXT NOT NULL,
        before_blob BLOB NOT NULL,
        created_at  INTEGER NOT NULL,
        PRIMARY KEY (file_id, seq)
     );
     CREATE INDEX IF NOT EXISTS idx_op_stash_file_seq ON op_stash(file_id, seq);
     CREATE TABLE IF NOT EXISTS final_snapshots (
        file_id    TEXT PRIMARY KEY,
        seq        INTEGER NOT NULL,
        blob       BLOB NOT NULL,
        created_at INTEGER NOT NULL
     );
```

함수 시그니처 — `impl Store` 블록 끝에 추가 (`append_snapshot` 함수 직후, 라인 약 99):

```rust
    pub fn append_op_stash(
        &self,
        file_id: &str,
        seq: i64,
        op_json: &str,
        before_blob: &[u8],
    ) -> rusqlite::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO op_stash(seq, file_id, op_json, before_blob, created_at) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![seq, file_id, op_json, before_blob, now],
        )?;
        // 세션당 100 entry 정책 — 초과 row 제거
        conn.execute(
            "DELETE FROM op_stash WHERE file_id = ?1 AND seq IN (
                SELECT seq FROM op_stash WHERE file_id = ?1 ORDER BY seq DESC LIMIT -1 OFFSET 100
             )",
            rusqlite::params![file_id],
        )?;
        Ok(())
    }

    pub fn pop_op_stash(&self, file_id: &str) -> rusqlite::Result<Option<OpStashRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT seq, op_json, before_blob FROM op_stash WHERE file_id = ?1 ORDER BY seq DESC LIMIT 1",
        )?;
        let row_opt = stmt.query_row(rusqlite::params![file_id], |row| {
            Ok(OpStashRow {
                seq: row.get(0)?,
                op_json: row.get(1)?,
                before_blob: row.get(2)?,
            })
        }).optional()?;

        if let Some(row) = &row_opt {
            conn.execute(
                "DELETE FROM op_stash WHERE file_id = ?1 AND seq = ?2",
                rusqlite::params![file_id, row.seq],
            )?;
        }
        Ok(row_opt)
    }

    pub fn count_op_stash(&self, file_id: &str) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM op_stash WHERE file_id = ?1",
            rusqlite::params![file_id],
            |row| row.get(0),
        )
    }

    pub fn list_op_stash_range(
        &self,
        file_id: &str,
        seq_from: i64,
        seq_to: i64,
    ) -> rusqlite::Result<Vec<OpStashRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT seq, op_json, before_blob FROM op_stash WHERE file_id = ?1 AND seq BETWEEN ?2 AND ?3 ORDER BY seq ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![file_id, seq_from, seq_to], |row| {
            Ok(OpStashRow {
                seq: row.get(0)?,
                op_json: row.get(1)?,
                before_blob: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_op_stash_by_seq(
        &self,
        file_id: &str,
        seq: i64,
    ) -> rusqlite::Result<Option<OpStashRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT seq, op_json, before_blob FROM op_stash WHERE file_id = ?1 AND seq = ?2",
        )?;
        stmt.query_row(rusqlite::params![file_id, seq], |row| {
            Ok(OpStashRow {
                seq: row.get(0)?,
                op_json: row.get(1)?,
                before_blob: row.get(2)?,
            })
        }).optional()
    }

    pub fn save_final_snapshot(
        &self,
        file_id: &str,
        seq: i64,
        blob: &[u8],
    ) -> rusqlite::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO final_snapshots(file_id, seq, blob, created_at) VALUES (?, ?, ?, ?)",
            rusqlite::params![file_id, seq, blob, now],
        )?;
        Ok(())
    }
```

그리고 파일 상단의 struct 정의 (Store 정의 부근) 직후에 추가:

```rust
#[derive(Debug, Clone)]
pub struct OpStashRow {
    pub seq: i64,
    pub op_json: String,
    pub before_blob: Vec<u8>,
}
```

`use rusqlite::OptionalExtension;` import 도 확인 (없으면 `use rusqlite::{params, OptionalExtension};` 추가).

- [ ] **Step 4: 테스트 통과**

Run: `cd server && cargo test store::tests::test_op_stash`
Expected: 3 PASS.

- [ ] **Step 5: 커밋**

```bash
git add server/src/store.rs
git commit -m "Task #zephy-bridge Sub-2 [2d.2]: op_stash + final_snapshots 테이블 + 함수들

append/pop/count/list_range/get_by_seq + save_final_snapshot. 세션당 100
entry 정책. OpStashRow struct.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2d.3: workbench handler 공통 helper — `apply_op_with_stash`

**Files:**
- Modify: `server/src/main.rs` (workbench handler 부근 라인 469-552)

- [ ] **Step 1: 헬퍼 함수 신설** — `workbench` 함수 *직전* (라인 약 468) 에 추가:

```rust
/// 단일 EditOperation 을 적용하면서 sqlite op_stash 영속 + broadcast 한 묶음으로 처리.
///
/// 1. core.export_hwpx_native() → before_blob
/// 2. core.apply_edit_op(&op)
/// 3. store.append_op_stash(file_id, seq, op_json, before_blob)
/// 4. events.publish(ServerEvent::Ops { seq, ops: [op] })
async fn apply_op_with_stash(
    state: &AppState,
    file_id: &str,
    session: Arc<Mutex<Session>>,
    op: EditOperation,
) -> Result<i64, AppError> {
    let before_blob = {
        let s = session.lock().unwrap();
        s.core.export_hwpx_native()
            .map_err(|e| AppError::Internal(format!("export_hwpx_native: {e}")))?
    };

    let op_json = serde_json::to_value(&op)
        .map_err(|e| AppError::Internal(format!("op 직렬화: {e}")))?;
    let op_json_str = op_json.to_string();

    let seq = {
        let mut s = session.lock().unwrap();
        s.core
            .apply_edit_op(&op)
            .map_err(|e| AppError::Internal(format!("apply_edit_op: {e}")))?;
        s.next_seq += 1;
        s.next_seq
    };

    state.store
        .append_op_stash(file_id, seq, &op_json_str, &before_blob)
        .map_err(|e| AppError::Internal(format!("op_stash 영속: {e}")))?;

    state.events.publish(
        file_id,
        ServerEvent::Ops { seq, ops: vec![op_json] },
    );

    Ok(seq)
}
```

`use` block (파일 상단) 에 `use crate::events::ServerEvent;`, `use rhwp_core::document_core::commands::edit_op::EditOperation;` 확인 (또는 정확한 경로 — DocumentCore crate 의 EditOperation re-export 경로).

- [ ] **Step 2: 컴파일 확인** — 호출자 없으니 미사용 경고만 (`#[allow(dead_code)]` 추가).

Run: `cd server && cargo build 2>&1 | tail -10`
Expected: 컴파일 성공.

- [ ] **Step 3: 커밋**

```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-2 [2d.3]: apply_op_with_stash helper

export_hwpx → apply → stash 영속 → broadcast 의 표준 시퀀스 1 함수로.
workbench arm 들이 호출.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2d.4: workbench handler — replace_runs arm

- [ ] **Step 1: 실패 통합 테스트** — `server/tests/workbench_actions.rs` (신설) 또는 main.rs tests 모듈:

```rust
#[tokio::test]
async fn test_workbench_replace_runs_persists_and_broadcasts() {
    let (state, app) = setup_test_server().await;
    let file_id = create_test_session_with_content(&state, "원본").await;

    let req = serde_json::json!({
        "action": "replace_runs",
        "payload": {
            "section": 0,
            "para": 0,
            "runs": [
                {"text": "변경됨", "style": {"bold": true}}
            ]
        }
    });
    let resp = post_workbench(&app, &file_id, &req).await;
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body["applied"], "ops");

    // 서버 IR 변경 확인
    let ir = get_ir(&app, &file_id, 0).await;
    let para0_text = ir["paragraphs"][0]["text"].as_str().unwrap();
    assert_eq!(para0_text, "변경됨");

    // sqlite op_stash 영속 확인
    let count = state.store.count_op_stash(&file_id).unwrap();
    assert_eq!(count, 1);
}
```

setup_test_server / create_test_session_with_content / post_workbench / get_ir 같은 helper 가 *현재 server crate 의 통합 테스트* 에 있는지 확인. 없으면 기존 [server/tests](../../server/tests/) 패턴 따라 신설.

- [ ] **Step 2: 실패 확인**

Run: `cd server && cargo test --test workbench_actions test_workbench_replace_runs_persists_and_broadcasts`
Expected: FAIL — handler 가 "replace_runs" 액션 인식 안 함 (현재 passthrough).

- [ ] **Step 3: workbench handler 의 `match req.action.as_str()` 에 새 arm 추가** — 현 코드 (라인 476) `"insert_text" => { ... }` 직후:

```rust
            "replace_runs" => {
                #[derive(serde::Deserialize)]
                struct Payload {
                    section: usize,
                    para: usize,
                    runs: Vec<rhwp_core::document_core::commands::edit_op::RunSpec>,
                }
                let payload: Payload = serde_json::from_value(req.payload.clone())
                    .map_err(|e| AppError::BadRequest(format!("INVALID_PAYLOAD: {e}")))?;
                let op = EditOperation::ReplaceRuns {
                    section: payload.section,
                    para: payload.para,
                    runs: payload.runs,
                };
                let seq = apply_op_with_stash(&state, &file_id, session.clone(), op).await?;
                return Ok(Json(WorkbenchResponse { seq, applied: "ops", info: None }));
            }
```

`RunSpec` 가 외부 export 되었는지 확인 (edit_op.rs 의 `pub struct RunSpec` 가 crate root 또는 commands 모듈에서 re-export 필요).

- [ ] **Step 4: 테스트 통과**

Run: `cd server && cargo test --test workbench_actions test_workbench_replace_runs_persists_and_broadcasts`
Expected: PASS.

- [ ] **Step 5: 커밋**

```bash
git add server/src/main.rs server/tests/workbench_actions.rs
git commit -m "Task #zephy-bridge Sub-2 [2d.4]: workbench replace_runs arm

payload → EditOperation::ReplaceRuns → apply_op_with_stash. 통합 테스트
서버 IR + op_stash 영속 검증.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2d.5: workbench handler — set_paragraph_style arm

같은 패턴. payload:
```rust
                #[derive(serde::Deserialize)]
                struct Payload {
                    section: usize,
                    para: usize,
                    #[serde(default)]
                    style: PartialParagraphStyle,
                }
```
op: `SetParagraphStyle { section, para, style }`.

부분 업데이트 검증 테스트 — `{style: {alignment: "right"}}` 만 보내고 다른 필드 *현재 값 유지* 확인.

```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-2 [2d.5]: workbench set_paragraph_style arm

부분 업데이트 (alignment 만 변경, 나머지 유지) 통합 테스트 포함.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2d.6: workbench handler — delete_range arm

payload:
```rust
                #[derive(serde::Deserialize)]
                struct Payload {
                    section: usize,
                    para_start: usize,
                    char_start: usize,
                    para_end: usize,
                    char_end: usize,
                }
```
op: `DeleteRange { section, para_start, char_start, para_end, char_end }`.

```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-2 [2d.6]: workbench delete_range arm

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2d.7: workbench handler — insert_paragraph arm

payload (옵셔널 count default 1):
```rust
                #[derive(serde::Deserialize)]
                struct Payload {
                    section: usize,
                    after_para: usize,
                    #[serde(default = "one")]
                    count: usize,
                    #[serde(default)]
                    style: Option<PartialParagraphStyle>,
                }
                fn one() -> usize { 1 }
```

```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-2 [2d.7]: workbench insert_paragraph arm

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2d.8: workbench handler — delete_element arm

payload:
```rust
                #[derive(serde::Deserialize)]
                struct Payload {
                    section: usize,
                    para: usize,
                    element_type: ElementType,
                }
```

```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-2 [2d.8]: workbench delete_element arm

element_type 분기 (Paragraph/Table). 테스트 양쪽.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2d.9: workbench handler — insert_table arm

payload:
```rust
                #[derive(serde::Deserialize)]
                struct Payload {
                    section: usize,
                    insert_after_para: usize,
                    rows: u16,
                    cols: u16,
                }
```

```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-2 [2d.9]: workbench insert_table arm

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2d.10: workbench handler — set_cell_style / merge_cells / replace_cell_runs / insert_text_in_cell / delete_range_in_cell arms

5 액션 한 task — 각각 payload 정의 + op 생성 + apply_op_with_stash 호출. 위 패턴 동일.

각 액션 payload 키 (spec §7 부합):

- `set_cell_style`: section, table_para, row, col, style (PartialCellStyle)
- `merge_cells`: section, table_para, row_start, col_start, row_end, col_end
- `replace_cell_runs`: section, table_para, row, col, cell_para, runs (Vec<RunSpec>)
- `insert_text_in_cell`: section, table_para, row, col, cell_para, offset, text, style (옵셔널)
- `delete_range_in_cell`: section, table_para, row, col, cell_para_start, char_start, cell_para_end, char_end

- [ ] **Step 1-5 × 5 액션**: 각각 통합 테스트 + arm + 커밋

각 액션마다 5개 통합 테스트 → 5 commit. 또는 묶어서 1 commit:

```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-2 [2d.10]: workbench 셀 액션 5개 arm 추가

set_cell_style·merge_cells·replace_cell_runs·insert_text_in_cell·
delete_range_in_cell. 각 액션 통합 테스트 — 서버 IR 변경 + op_stash 영속.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2d.11: Phase 2d 회귀

- [ ] **Step 1: server cargo test**

Run: `cd server && cargo test 2>&1 | tail -30`
Expected: 모든 통과 — events tests (7) + store tests (3 + 3 신규) + workbench actions (12 신규).

- [ ] **Step 2: clippy 통과 확인**

---

## Phase 2e: 신규 endpoint 4개 + complete arm

### Task 2e.1: POST /sessions/:id/undo

**Files:**
- Modify: `server/src/main.rs` (handler + 라우트 등록)

- [ ] **Step 1: 실패 테스트**

```rust
#[tokio::test]
async fn test_undo_pops_stash_and_restores_blob() {
    let (state, app) = setup_test_server().await;
    let file_id = create_test_session_with_content(&state, "원본").await;

    // replace_runs 로 변경 1
    post_workbench(&app, &file_id, &serde_json::json!({
        "action": "replace_runs",
        "payload": {"section":0, "para":0, "runs":[{"text":"변경1"}]}
    })).await;
    // replace_runs 로 변경 2
    post_workbench(&app, &file_id, &serde_json::json!({
        "action": "replace_runs",
        "payload": {"section":0, "para":0, "runs":[{"text":"변경2"}]}
    })).await;

    assert_eq!(state.store.count_op_stash(&file_id).unwrap(), 2);

    // undo 1회 → 변경1 상태로
    let resp = post(&app, &format!("/sessions/{}/undo", file_id), &serde_json::json!({})).await;
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body["applied"], "undo");

    let ir = get_ir(&app, &file_id, 0).await;
    assert_eq!(ir["paragraphs"][0]["text"], "변경1");
    assert_eq!(state.store.count_op_stash(&file_id).unwrap(), 1);

    // undo 2회 → 원본 상태로
    post(&app, &format!("/sessions/{}/undo", file_id), &serde_json::json!({})).await;
    let ir = get_ir(&app, &file_id, 0).await;
    assert_eq!(ir["paragraphs"][0]["text"], "원본");

    // 빈 stash → 409
    let resp = post(&app, &format!("/sessions/{}/undo", file_id), &serde_json::json!({})).await;
    assert_eq!(resp.status, 409);
}
```

- [ ] **Step 2: 실패 확인**

Run: `cd server && cargo test --test workbench_actions test_undo`
Expected: FAIL — endpoint 404.

- [ ] **Step 3: undo handler + 라우트**

handler 함수 — workbench handler 부근에 신설:

```rust
async fn undo_handler(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<Json<UndoResponse>, AppError> {
    let session = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(&file_id)
            .ok_or(AppError::NotFound)?
            .clone()
    };

    let row = state.store
        .pop_op_stash(&file_id)
        .map_err(|e| AppError::Internal(format!("pop_op_stash: {e}")))?
        .ok_or_else(|| AppError::Conflict("NO_UNDO_AVAILABLE".to_string()))?;

    let new_core = rhwp_core::document_core::DocumentCore::from_bytes(&row.before_blob)
        .map_err(|e| AppError::Internal(format!("from_bytes: {e}")))?;

    let seq = {
        let mut s = session.lock().unwrap();
        s.core = new_core;
        s.next_seq += 1;
        s.next_seq
    };

    let snapshot_base64 = base64::engine::general_purpose::STANDARD.encode(&row.before_blob);
    state.events.publish(
        &file_id,
        ServerEvent::SnapshotRestored { seq, snapshot_base64 },
    );

    Ok(Json(UndoResponse {
        seq_reverted: row.seq,
        applied: "undo",
    }))
}

#[derive(serde::Serialize)]
struct UndoResponse {
    seq_reverted: i64,
    applied: &'static str,
}
```

`AppError::Conflict` variant 가 없으면 추가. `base64::engine` import 도 확인.

라우트 등록 — `router()` 함수의 `/sessions/:id/workbench` 라우트 직후:
```rust
        .route("/sessions/:id/undo", post(undo_handler))
```

- [ ] **Step 4: 테스트 통과**

Run: `cd server && cargo test --test workbench_actions test_undo_pops_stash_and_restores_blob`
Expected: PASS.

- [ ] **Step 5: 커밋**

```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-2 [2e.1]: POST /sessions/:id/undo

pop_op_stash → DocumentCore::from_bytes → session.core 교체 → broadcast
SnapshotRestored. 빈 stash 시 409 NO_UNDO_AVAILABLE.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2e.2: GET /sessions/:id/audit?seq_from=&seq_to=

handler:
```rust
#[derive(serde::Deserialize)]
struct AuditQuery {
    seq_from: i64,
    seq_to: i64,
}

#[derive(serde::Serialize)]
struct AuditRow {
    seq: i64,
    op: serde_json::Value,
    created_at: i64,
}

async fn audit_handler(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditRow>>, AppError> {
    let rows = state.store
        .list_op_stash_range(&file_id, q.seq_from, q.seq_to)
        .map_err(|e| AppError::Internal(format!("list_op_stash_range: {e}")))?;

    let result: Vec<_> = rows.into_iter().map(|r| {
        // op_json 을 Value 로 파싱 (실패 시 raw 문자열)
        let op_value: serde_json::Value = serde_json::from_str(&r.op_json)
            .unwrap_or(serde_json::Value::String(r.op_json));
        AuditRow {
            seq: r.seq,
            op: op_value,
            created_at: 0,  // store::list_op_stash_range 가 created_at 반환하도록 확장 필요
        }
    }).collect();

    Ok(Json(result))
}
```

라우트:
```rust
        .route("/sessions/:id/audit", get(audit_handler))
```

테스트 + 커밋:
```bash
git add server/src/main.rs server/src/store.rs
git commit -m "Task #zephy-bridge Sub-2 [2e.2]: GET /audit endpoint

seq_from/seq_to 범위. list_op_stash_range 결과를 op_json 파싱해 반환.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2e.3: GET /sessions/:id/diff?seq=N

handler — seq 의 before_blob 과 seq+1 의 before_blob (= 현 seq 의 after) 두 개를 `DocumentCore::from_bytes` 로 임시 코어 만들어 IR 비교:

```rust
#[derive(serde::Deserialize)]
struct DiffQuery { seq: i64 }

#[derive(serde::Serialize)]
struct DiffResponse {
    seq: i64,
    op: serde_json::Value,
    before_paragraphs: Vec<String>,
    after_paragraphs: Vec<String>,
    chars_added: i64,
    chars_removed: i64,
}

async fn diff_handler(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(q): Query<DiffQuery>,
) -> Result<Json<DiffResponse>, AppError> {
    let target = state.store
        .get_op_stash_by_seq(&file_id, q.seq)
        .map_err(|e| AppError::Internal(format!("get_op_stash_by_seq: {e}")))?
        .ok_or(AppError::NotFound)?;

    let before_core = rhwp_core::document_core::DocumentCore::from_bytes(&target.before_blob)
        .map_err(|e| AppError::Internal(format!("before from_bytes: {e}")))?;

    // after = seq+1 의 before_blob. 없으면 현재 session.core 의 export.
    let after_blob = match state.store.get_op_stash_by_seq(&file_id, q.seq + 1)
        .map_err(|e| AppError::Internal(format!("get next: {e}")))? {
        Some(next) => next.before_blob,
        None => {
            let sessions = state.sessions.lock().unwrap();
            let session = sessions.get(&file_id).ok_or(AppError::NotFound)?.clone();
            let s = session.lock().unwrap();
            s.core.export_hwpx_native()
                .map_err(|e| AppError::Internal(format!("export after: {e}")))?
        }
    };
    let after_core = rhwp_core::document_core::DocumentCore::from_bytes(&after_blob)
        .map_err(|e| AppError::Internal(format!("after from_bytes: {e}")))?;

    let before_paragraphs: Vec<String> = before_core.document().sections[0]
        .paragraphs.iter().map(|p| p.text.clone()).collect();
    let after_paragraphs: Vec<String> = after_core.document().sections[0]
        .paragraphs.iter().map(|p| p.text.clone()).collect();

    let before_total: usize = before_paragraphs.iter().map(|s| s.chars().count()).sum();
    let after_total: usize = after_paragraphs.iter().map(|s| s.chars().count()).sum();
    let chars_added = (after_total as i64 - before_total as i64).max(0);
    let chars_removed = (before_total as i64 - after_total as i64).max(0);

    let op_value: serde_json::Value = serde_json::from_str(&target.op_json)
        .unwrap_or(serde_json::Value::Null);

    Ok(Json(DiffResponse {
        seq: q.seq,
        op: op_value,
        before_paragraphs,
        after_paragraphs,
        chars_added,
        chars_removed,
    }))
}
```

라우트:
```rust
        .route("/sessions/:id/diff", get(diff_handler))
```

테스트 + 커밋:
```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-2 [2e.3]: GET /diff endpoint

seq 의 before/after blob 두 개를 임시 코어 로 비교. 본문 paragraph
텍스트 + 글자 수 차이.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2e.4: GET /sessions/:id/ir-slice

handler:
```rust
#[derive(serde::Deserialize)]
struct IrSliceQuery {
    #[serde(default)]
    sec: Option<usize>,
    #[serde(default)]
    para_start: Option<usize>,
    #[serde(default)]
    para_end: Option<usize>,
    #[serde(default = "default_mode")]
    mode: String,
}
fn default_mode() -> String { "auto".to_string() }

async fn ir_slice_handler(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(q): Query<IrSliceQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions.get(&file_id).ok_or(AppError::NotFound)?.clone();
    drop(sessions);
    let s = session.lock().unwrap();

    let sec = q.sec.unwrap_or(0);
    let total = s.core.document().sections[sec].paragraphs.len();
    let para_start = q.para_start.unwrap_or(0);
    let para_end = q.para_end.unwrap_or(total).min(total);

    let paragraphs: Vec<serde_json::Value> = (para_start..para_end).map(|p| {
        let para = &s.core.document().sections[sec].paragraphs[p];
        match q.mode.as_str() {
            "raw" => serde_json::to_value(para).unwrap_or(serde_json::Value::Null),
            "compact" => serde_json::json!({
                "para": p,
                "text": para.text,
                "para_shape_id": para.para_shape_id,
            }),
            _ => {
                // auto — 텍스트 길이 합이 임계 미만이면 raw, 초과면 compact
                let total_chars: usize = (para_start..para_end)
                    .map(|i| s.core.document().sections[sec].paragraphs[i].text.chars().count())
                    .sum();
                if total_chars < 5000 {
                    serde_json::to_value(para).unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::json!({
                        "para": p,
                        "text": para.text,
                        "para_shape_id": para.para_shape_id,
                    })
                }
            }
        }
    }).collect();

    Ok(Json(serde_json::json!({
        "section": sec,
        "para_start": para_start,
        "para_end": para_end,
        "mode": q.mode,
        "paragraphs": paragraphs,
    })))
}
```

라우트:
```rust
        .route("/sessions/:id/ir-slice", get(ir_slice_handler))
```

테스트 + 커밋:
```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-2 [2e.4]: GET /ir-slice endpoint

sec/para_start/para_end/mode (raw|compact|auto). auto 는 5000자 임계.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2e.5: workbench `complete` arm + ServerEvent::Complete + save_final_snapshot

workbench handler 의 match 끝부분 (또는 새 arm):
```rust
            "complete" => {
                let blob = {
                    let s = session.lock().unwrap();
                    s.core.export_hwpx_native()
                        .map_err(|e| AppError::Internal(format!("export_hwpx: {e}")))?
                };
                let seq = {
                    let mut s = session.lock().unwrap();
                    s.next_seq += 1;
                    s.next_seq
                };
                state.store.save_final_snapshot(&file_id, seq, &blob)
                    .map_err(|e| AppError::Internal(format!("save_final_snapshot: {e}")))?;
                state.events.publish(&file_id, ServerEvent::Complete { seq });
                return Ok(Json(WorkbenchResponse { seq, applied: "complete", info: None }));
            }
```

테스트:
```rust
#[tokio::test]
async fn test_complete_action_persists_final_and_broadcasts() {
    let (state, app) = setup_test_server().await;
    let file_id = create_test_session_with_content(&state, "끝").await;
    let resp = post_workbench(&app, &file_id, &serde_json::json!({
        "action": "complete",
        "payload": {}
    })).await;
    assert_eq!(resp.body["applied"], "complete");
    // final_snapshots 영속 확인 — 직접 sqlite 조회 helper
    // (또는 final_snapshot exists 추가 함수)
}
```

커밋:
```bash
git add server/src/main.rs
git commit -m "Task #zephy-bridge Sub-2 [2e.5]: workbench complete arm + ServerEvent::Complete

export_hwpx_native → save_final_snapshot → broadcast Complete.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2e.6: Phase 2e 회귀

- [ ] cd server && cargo test + clippy 통과.

---

## Phase 2f: 클라 wasm-bridge wrapper + onServerEvent + e2e + 보고서

### Task 2f.1: wasm-bridge wrapper 6개 신설

**Files:**
- Modify: `rhwp-studio/src/core/wasm-bridge.ts`

- [ ] **Step 1: wrapper 추가** — 기존 `replaceCellRuns` / `applyParaFormat` 등 메서드 부근에 6개 신설:

```typescript
  replaceRuns(secIdx: number, paraIdx: number, runsJson: string): { ok: boolean } {
    const r = this.module.replaceRuns(secIdx, paraIdx, runsJson);
    return JSON.parse(r);
  }

  applyParaFormat(secIdx: number, paraIdx: number, propsJson: string): { ok: boolean } {
    const r = this.module.applyParaFormat(secIdx, paraIdx, propsJson);
    return JSON.parse(r);
  }

  deleteRange(
    secIdx: number, startPara: number, startOffset: number,
    endPara: number, endOffset: number,
  ): { ok: boolean } {
    // WASM apply 측 시그니처 확인 후 정확히 — 예: deleteRange(sec, sp, so, ep, eo, null)
    const r = this.module.deleteRange(secIdx, startPara, startOffset, endPara, endOffset);
    return JSON.parse(r);
  }

  insertParagraph(secIdx: number, paraIdx: number): { ok: boolean } {
    const r = this.module.insertParagraph(secIdx, paraIdx);
    return JSON.parse(r);
  }

  deleteParagraph(secIdx: number, paraIdx: number): { ok: boolean } {
    const r = this.module.deleteParagraph(secIdx, paraIdx);
    return JSON.parse(r);
  }

  deleteTableControl(secIdx: number, paraIdx: number): { ok: boolean } {
    const r = this.module.deleteTableControl(secIdx, paraIdx);
    return JSON.parse(r);
  }

  replaceCellRuns(
    secIdx: number, tableParaIdx: number, ctrlIdx: number,
    cellIdx: number, cellParaIdx: number, runsJson: string,
  ): { ok: boolean } {
    const r = this.module.replaceCellRuns(secIdx, tableParaIdx, ctrlIdx, cellIdx, cellParaIdx, runsJson);
    return JSON.parse(r);
  }
```

각 wrapper 의 정확한 WASM 메서드 이름은 wasm_api.rs 의 `#[wasm_bindgen(js_name = ...)]` 값에 맞춤.

- [ ] **Step 2: 타입 정의 보강** — `WasmBridge` interface 정의 (있다면) 에 메서드 시그니처 추가. 없으면 클래스 정의 그대로.

- [ ] **Step 3: tsc + vite build 확인**

Run: `cd rhwp-studio && npm run build 2>&1 | tail -20`
Expected: 빌드 성공.

- [ ] **Step 4: 커밋**

```bash
git add rhwp-studio/src/core/wasm-bridge.ts
git commit -m "Task #zephy-bridge Sub-2 [2f.1]: wasm-bridge wrapper 6개 신설

replaceRuns·applyParaFormat·deleteRange·insertParagraph·deleteParagraph·
deleteTableControl·replaceCellRuns. WASM export 호출 + JSON 파싱.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2f.2: main.ts onServerEvent ops 분기 12 종 확장

**Files:**
- Modify: `rhwp-studio/src/main.ts` (라인 117-156 의 ops 분기 switch 안)

- [ ] **Step 1: 12 case 추가** — 기존 `insert_text` / `split_paragraph` 다음에 추가:

```typescript
              case 'replace_runs':
                if (typeof op.section !== 'number' ||
                    typeof op.para !== 'number' ||
                    !Array.isArray(op.runs)) {
                  console.warn('[main] replace_runs payload 미일치 — 무시'); break;
                }
                wasm.replaceRuns(op.section, op.para, JSON.stringify(op.runs));
                appliedCount += 1;
                break;
              case 'set_paragraph_style':
                if (typeof op.section !== 'number' || typeof op.para !== 'number' || typeof op.style !== 'object') {
                  console.warn('[main] set_paragraph_style payload 미일치'); break;
                }
                wasm.applyParaFormat(op.section, op.para, JSON.stringify(op.style));
                appliedCount += 1;
                break;
              case 'delete_range':
                if (typeof op.section !== 'number' ||
                    typeof op.para_start !== 'number' ||
                    typeof op.char_start !== 'number' ||
                    typeof op.para_end !== 'number' ||
                    typeof op.char_end !== 'number') {
                  console.warn('[main] delete_range payload 미일치'); break;
                }
                wasm.deleteRange(op.section, op.para_start, op.char_start, op.para_end, op.char_end);
                appliedCount += 1;
                break;
              case 'insert_paragraph':
                if (typeof op.section !== 'number' || typeof op.after_para !== 'number') {
                  console.warn('[main] insert_paragraph payload 미일치'); break;
                }
                {
                  const cnt = typeof op.count === 'number' ? op.count : 1;
                  for (let i = 0; i < cnt; i++) {
                    wasm.insertParagraph(op.section, op.after_para + i);
                    if (op.style && typeof op.style === 'object') {
                      wasm.applyParaFormat(op.section, op.after_para + i + 1, JSON.stringify(op.style));
                    }
                  }
                  appliedCount += 1;
                }
                break;
              case 'delete_element':
                if (typeof op.section !== 'number' || typeof op.para !== 'number' ||
                    typeof op.element_type !== 'string') {
                  console.warn('[main] delete_element payload 미일치'); break;
                }
                if (op.element_type === 'paragraph') {
                  wasm.deleteParagraph(op.section, op.para);
                } else if (op.element_type === 'table') {
                  wasm.deleteTableControl(op.section, op.para);
                } else {
                  console.warn(`[main] 알 수 없는 element_type: ${op.element_type}`);
                  break;
                }
                appliedCount += 1;
                break;
              case 'insert_table':
                if (typeof op.section !== 'number' ||
                    typeof op.insert_after_para !== 'number' ||
                    typeof op.rows !== 'number' ||
                    typeof op.cols !== 'number') {
                  console.warn('[main] insert_table payload 미일치'); break;
                }
                {
                  // insert_after_para 의 끝(char_offset = para 길이)에 삽입.
                  // WASM createTable(sec, para, charOffset, rows, cols) — charOffset 은 서버와 동일하게 *para 길이* 로.
                  const paraLen = wasm.getParagraphLength(op.section, op.insert_after_para);
                  wasm.createTable(op.section, op.insert_after_para, paraLen, op.rows, op.cols);
                  appliedCount += 1;
                }
                break;
              case 'set_cell_style':
                if (typeof op.section !== 'number' ||
                    typeof op.table_para !== 'number' ||
                    typeof op.row !== 'number' ||
                    typeof op.col !== 'number' ||
                    typeof op.style !== 'object') {
                  console.warn('[main] set_cell_style payload 미일치'); break;
                }
                {
                  // (row, col) → cell_idx 변환은 wasm.findCellIdx 또는 자체 cell 좌표 계산
                  const ctrlIdx = 0;
                  const cellIdx = wasm.findCellIdx
                    ? wasm.findCellIdx(op.section, op.table_para, ctrlIdx, op.row, op.col)
                    : op.row * 100 + op.col;  // fallback — find_cell_idx wasm export 가 없으면 신설 필요
                  wasm.setCellProperties(op.section, op.table_para, ctrlIdx, cellIdx, JSON.stringify(op.style));
                  appliedCount += 1;
                }
                break;
              case 'merge_cells':
                if (typeof op.section !== 'number' || typeof op.table_para !== 'number' ||
                    typeof op.row_start !== 'number' || typeof op.col_start !== 'number' ||
                    typeof op.row_end !== 'number' || typeof op.col_end !== 'number') {
                  console.warn('[main] merge_cells payload 미일치'); break;
                }
                wasm.mergeTableCells(op.section, op.table_para, 0, op.row_start, op.col_start, op.row_end, op.col_end);
                appliedCount += 1;
                break;
              case 'replace_cell_runs':
                if (typeof op.section !== 'number' || typeof op.table_para !== 'number' ||
                    typeof op.row !== 'number' || typeof op.col !== 'number' ||
                    typeof op.cell_para !== 'number' || !Array.isArray(op.runs)) {
                  console.warn('[main] replace_cell_runs payload 미일치'); break;
                }
                {
                  const ctrlIdx = 0;
                  const cellIdx = wasm.findCellIdx
                    ? wasm.findCellIdx(op.section, op.table_para, ctrlIdx, op.row, op.col)
                    : op.row * 100 + op.col;
                  wasm.replaceCellRuns(op.section, op.table_para, ctrlIdx, cellIdx, op.cell_para, JSON.stringify(op.runs));
                  appliedCount += 1;
                }
                break;
              case 'insert_text_in_cell':
                if (typeof op.section !== 'number' || typeof op.table_para !== 'number' ||
                    typeof op.row !== 'number' || typeof op.col !== 'number' ||
                    typeof op.cell_para !== 'number' || typeof op.offset !== 'number' ||
                    typeof op.text !== 'string') {
                  console.warn('[main] insert_text_in_cell payload 미일치'); break;
                }
                {
                  const ctrlIdx = 0;
                  const cellIdx = wasm.findCellIdx
                    ? wasm.findCellIdx(op.section, op.table_para, ctrlIdx, op.row, op.col)
                    : op.row * 100 + op.col;
                  wasm.insertTextInCell(op.section, op.table_para, ctrlIdx, cellIdx, op.cell_para, op.offset, op.text);
                  if (op.style && typeof op.style === 'object') {
                    wasm.applyCharFormatInCell(op.section, op.table_para, ctrlIdx, cellIdx, op.cell_para, op.offset, op.offset + op.text.length, JSON.stringify(op.style));
                  }
                  appliedCount += 1;
                }
                break;
              case 'delete_range_in_cell':
                if (typeof op.section !== 'number' || typeof op.table_para !== 'number' ||
                    typeof op.row !== 'number' || typeof op.col !== 'number' ||
                    typeof op.cell_para_start !== 'number' || typeof op.char_start !== 'number' ||
                    typeof op.cell_para_end !== 'number' || typeof op.char_end !== 'number') {
                  console.warn('[main] delete_range_in_cell payload 미일치'); break;
                }
                {
                  const ctrlIdx = 0;
                  const cellIdx = wasm.findCellIdx
                    ? wasm.findCellIdx(op.section, op.table_para, ctrlIdx, op.row, op.col)
                    : op.row * 100 + op.col;
                  wasm.deleteRangeInCell(op.section, op.table_para, ctrlIdx, cellIdx, op.cell_para_start, op.char_start, op.cell_para_end, op.char_end);
                  appliedCount += 1;
                }
                break;
```

*주의: `wasm.findCellIdx` 가 wasm export 에 *없을 가능성*. *(row,col) → cell_idx 변환* 이 클라 측에서 어떻게 일어나는지 코드 조사 필요. 만약 fallback (`op.row * 100 + op.col`) 이 부정확하면 *wasm 측 신설 export 추가* — Task 2a 에서 다뤘어야. 본 step 진입 전 wasm-bridge 확인.*

- [ ] **Step 2: tsc + vite build**

Run: `cd rhwp-studio && npm run build 2>&1 | tail -10`
Expected: 빌드 성공.

- [ ] **Step 3: 커밋**

```bash
git add rhwp-studio/src/main.ts
git commit -m "Task #zephy-bridge Sub-2 [2f.2]: onServerEvent ops 분기 12 종 확장

replace_runs / set_paragraph_style / delete_range / insert_paragraph /
delete_element / insert_table / set_cell_style / merge_cells /
replace_cell_runs / insert_text_in_cell / delete_range_in_cell.
각 op shape 가드 + wasm 호출 + appliedCount++.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2f.3: onServerEvent SnapshotRestored + Complete 핸들러

**Files:**
- Modify: `rhwp-studio/src/main.ts`

- [ ] **Step 1: 추가** — ops/workbench 분기 뒤에 신설:

```typescript
      } else if (ev.kind === 'snapshot_restored') {
        if (typeof ev.snapshot_base64 !== 'string') {
          console.warn('[main] snapshot_restored snapshot_base64 누락'); return;
        }
        try {
          // base64 → Uint8Array
          const bin = Uint8Array.from(atob(ev.snapshot_base64), c => c.charCodeAt(0));
          // WASM 통째 교체 — wasm.fromBytes(bin) 가 *기존 인스턴스 reset* 또는 신규 인스턴스 mount
          // 정확한 API 는 wasm-bridge.ts 의 fromBytes / loadDocument 등. 없으면 신설.
          wasm.loadDocument(bin);
          eventBus.emit('document-changed');
        } catch (e) {
          console.error('[main] snapshot_restored 적용 실패:', e);
        }
      } else if (ev.kind === 'complete') {
        console.log(`[main] 워크벤치 종료 시그널 — seq ${ev.seq}`);
        // Sub-3 에서 UI 표시 통합.
      }
```

`wasm.loadDocument` 가 존재하지 않으면 — wasm-bridge 에 wrapper 추가. WASM 측 `from_bytes` export 가 *기존 인스턴스 reset* 지원하는지 확인. 안 되면 *WasmBridge 인스턴스 재생성* 패턴 (SessionClient 가 reset 호출).

- [ ] **Step 2: 빌드 확인**

Run: `cd rhwp-studio && npm run build 2>&1 | tail -10`
Expected: 빌드 성공.

- [ ] **Step 3: 커밋**

```bash
git add rhwp-studio/src/main.ts rhwp-studio/src/core/wasm-bridge.ts
git commit -m "Task #zephy-bridge Sub-2 [2f.3]: onServerEvent SnapshotRestored / Complete

SnapshotRestored 받으면 base64 디코드 → wasm 통째 교체 → document-changed.
Complete 는 console.log. UI 통합은 Sub-3.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2f.4: 노트북 SSR 라우터 정규화 확장

**Files:**
- Modify: `hwp_sub_agent_simulation_ssr.ipynb` cell 3 (정규화 매핑 표)

- [ ] **Step 1: 매핑 표 확인 + 부분 업데이트 시연 셀 추가**

Cell 3 의 `SKILL_TO_SERVER_KEY` 가 *현재 매핑* 을 모두 포함하는지 확인. spec §6 표 기준:
- `sec` → `section`
- `char_offset` → `offset`
- 그 외 키는 그대로

추가로 필요한 매핑이 있는지 — 예: SKILL.md 가 `para_start`/`char_start` 등 키를 어떤 형태로 보내는지. SKILL.md 의 payload 표 (spec §7 표) 가 *snake_case + char_start 그대로* 이므로 추가 매핑 불필요.

- [ ] **Step 2: 부분 업데이트 시연 셀 추가** — cell 5 (sub_agent_run 정의 직후 또는 별도 cell):

```python
# Sub-2 시연 — 부분 업데이트 (bold 만, 다른 서식 유지)
result = sub_agent_run(
    file_id=file_id,
    user_message='첫 문단을 굵게 만들어줘',
    tool_allowlist={'Bash'},
)
print(result)
```

- [ ] **Step 3: 커밋**

```bash
git add hwp_sub_agent_simulation_ssr.ipynb
git commit -m "Task #zephy-bridge Sub-2 [2f.4]: 노트북 부분 업데이트 시연 cell 추가

bold 만 변경 시 다른 서식 유지 시연 — set_paragraph_style + replace_runs.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2f.5-2f.15: e2e 11개 액션별

각 액션마다 1 e2e — `sub2-<action>.test.mjs`. 패턴:

```javascript
import WebSocket from 'ws';

const FILE_ID = `sub2-${Date.now()}`;
const BASE = 'http://127.0.0.1:7710';

// 1. 세션 생성
await fetch(`${BASE}/sessions`, {
  method: 'POST',
  headers: {'Content-Type': 'application/json'},
  body: JSON.stringify({fileId: FILE_ID, format: 'hwpx', base64: <빈 hwpx>})
});

// 2. WS 구독 시작
const ws = new WebSocket(`ws://127.0.0.1:7710/sessions/${FILE_ID}/ws`);
const received = [];
ws.on('message', (m) => received.push(JSON.parse(m.toString())));
await new Promise(r => ws.once('open', r));

// 3. POST /workbench replace_runs (예시)
const resp = await fetch(`${BASE}/sessions/${FILE_ID}/workbench`, {
  method: 'POST',
  headers: {'Content-Type': 'application/json'},
  body: JSON.stringify({
    action: 'replace_runs',
    payload: {section: 0, para: 0, runs: [{text: 'E2E-RUN', style: {bold: true}}]}
  })
});
console.assert(resp.status === 200);
const body = await resp.json();
console.assert(body.applied === 'ops');

// 4. 브로드캐스트 수신 확인
await new Promise(r => setTimeout(r, 500));
const opsEv = received.find(ev => ev.kind === 'ops');
console.assert(opsEv && opsEv.ops[0].op === 'replace_runs');

// 5. 서버 IR 확인
const ir = await fetch(`${BASE}/sessions/${FILE_ID}/ir?page=0`).then(r => r.json());
console.assert(ir.paragraphs[0].text === 'E2E-RUN');

console.log('=== Sub-2 replace_runs e2e 통과 ===');
ws.close();
```

11 액션 각각 동일 패턴 — 단 payload 와 검증만 변형. 분량 절약을 위해 *e2e 1개 task 당 1 action* 또는 *11 액션 묶어 1 commit*. 권장:

- [ ] e2e 11개 각각 신설 + 통합 1 commit:

```bash
git add rhwp-studio/e2e/sub2-*.test.mjs
git commit -m "Task #zephy-bridge Sub-2 [2f.5-2f.15]: e2e 11 액션 + undo + audit/diff/ir-slice

각 액션별 — POST /workbench → broadcast 수신 + 서버 IR 영속 검증.
undo e2e — replace_runs 2회 후 undo 2회 원복 확인. audit/diff/ir-slice
endpoint e2e 별도.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2f.16: 부분 업데이트 e2e

별도 e2e — `sub2-partial-update.test.mjs`:
```javascript
// 1. 세션 생성 + 본문 1개 문단
// 2. set_paragraph_style {alignment: "right"} 만 보냄 — line_spacing 등 유지
// 3. set_paragraph_style {line_spacing: 200.0} 만 보냄 — alignment 가 right 유지 확인
// 4. /ir 또는 /ir-slice 로 검증
```

```bash
git add rhwp-studio/e2e/sub2-partial-update.test.mjs
git commit -m "Task #zephy-bridge Sub-2 [2f.16]: 부분 업데이트 e2e

alignment 만 변경 → line_spacing 만 변경 → alignment 유지 확인.
spec §7 부분 업데이트 의도 검증.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2f.17: 회귀 검증 — Sub-1 e2e + 신규 e2e 모두 통과

- [ ] **Step 1: e2e 전체 실행**

Run: `cd rhwp-studio && node e2e/ws-bridge.test.mjs && node e2e/sub2-replace-runs.test.mjs && node e2e/sub2-set-paragraph-style.test.mjs && node e2e/sub2-delete-range.test.mjs && node e2e/sub2-insert-paragraph.test.mjs && node e2e/sub2-delete-element.test.mjs && node e2e/sub2-insert-table.test.mjs && node e2e/sub2-set-cell-style.test.mjs && node e2e/sub2-merge-cells.test.mjs && node e2e/sub2-replace-cell-runs.test.mjs && node e2e/sub2-insert-text-in-cell.test.mjs && node e2e/sub2-delete-range-in-cell.test.mjs && node e2e/sub2-undo.test.mjs && node e2e/sub2-audit-diff-ir-slice.test.mjs && node e2e/sub2-partial-update.test.mjs`

Expected: 모든 e2e 통과. Sub-1 ws-bridge 회귀 0.

- [ ] **Step 2: cargo test 전체 (rhwp 본체 + server)**

Run: `cargo test --lib --workspace 2>&1 | tail -10`
Expected: 모든 통과.

- [ ] **Step 3: 시각 회귀 검증 — sub-agent 시연**

수동 시연 안내 — 새 노트북 (혹은 Incognito 탭) 에서:
1. cell 1-6 실행
2. LLM 에 "첫 문단을 굵게, 두번째 문단 추가, 그 안에 표 삽입" 같은 복합 요청
3. 브라우저 화면이 *각 액션마다 실시간 반영* 되는지 확인
4. undo 호출 (별도 curl 또는 UI) → 원본 복원 시각 확인

이건 사용자 영역 — sub-agent 가 자동화 가능하면 *Puppeteer 또는 Playwright* 활용 (Sub-1 처럼 Node WS 직접 검증으로 충분할 수도).

---

### Task 2f.18: stage2 + report 작성

**Files:**
- Create: `mydocs/working/task_m200_zephy_bridge_sub2_stage1.md`
- Create: `mydocs/report/task_m200_zephy_bridge_sub2_report.md`

- [ ] **Step 1: stage2 작성** — Sub-1 의 [task_m200_zephy_bridge_stage1.md](../working/task_m200_zephy_bridge_stage1.md) 톤 따름. 각 task 결과·주요 commit·자동 검증 결과·알려진 한계.

- [ ] **Step 2: report 작성** — Sub-1 의 [task_m200_zephy_bridge_report.md](../report/task_m200_zephy_bridge_report.md) 톤 따름. DoD 통과 여부 표 + 신규 인터페이스 정리 + 신규/수정 파일 + 커밋 이력 + Sub-3 으로 미루는 항목.

- [ ] **Step 3: 커밋**

```bash
git add mydocs/working/task_m200_zephy_bridge_sub2_stage1.md mydocs/report/task_m200_zephy_bridge_sub2_report.md
git commit -m "Task #zephy-bridge Sub-2: stage2 + report 작성

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## 자체 점검 게이트 — Definition of Done

- [ ] 12 액션 모두 *서버가 적용 + sqlite 영속* — `cd server && cargo test` 통과
- [ ] 신규 endpoint 4개 — `/undo`, `/audit`, `/diff`, `/ir-slice` integration test 통과
- [ ] `complete` workbench arm — `ServerEvent::Complete` 브로드캐스트 + sqlite `final_snapshots` 영속
- [ ] 부분 업데이트 — `set_paragraph_style {alignment 만}` → 다른 필드 *현재 값 유지* e2e 통과
- [ ] Sub-1 기존 e2e (`ws-bridge.test.mjs`) 회귀 0
- [ ] 양방향 e2e 모든 액션 통과 (12 + undo + audit/diff/ir-slice + partial-update)
- [ ] broadcast 페이로드에 *정방향 EditOperation* 본문 포함 (audit·diff 로 확인)
- [ ] 수동 시연 통과 — 새 노트북 실행 → LLM 12 액션 호출 → 브라우저 시각 반영 + undo 통째 복원
- [ ] stage2 + report 작성·commit
