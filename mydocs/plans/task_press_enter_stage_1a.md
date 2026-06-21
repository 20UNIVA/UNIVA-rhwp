# 단계 1a (rhwp) — `PressEnter` EditOperation 신설

작성일: 2026-06-21
대상 저장소: UNIVA-rhwp
연결 spec: [`../../../docs/25-press-enter-redesign.md`](../../../docs/25-press-enter-redesign.md)

이슈 등록 자세 없이 진행 (사용자 결정). 워크플로우 — 수행계획서 → 단계별 보고 → 최종 보고 자세는 유지.

## 1. 작업 개요

`EditOperation::PressEnter` 신설 — `InsertParagraph` 의 *이름 ↔ 동작 어긋남* 사고 해소 + 셀 내부 paragraph 추가 신규 가치. **단일 variant `PressEnter`**. payload 키 (`table_para` 부재 / 박힘) 로 본문 / 셀 분기. 옛 `InsertParagraph` 는 단계 1a 자체에서는 그대로 유지 (단계 4a 에서 제거).

근거·디자인 권위는 [25-press-enter-redesign.md](../../../docs/25-press-enter-redesign.md).

## 2. 결정 사항

- *단일 variant `PressEnter`* — payload key 자체로 분기 (본문 / 셀)
- *셀 모드 + `page_break:true`* → `INVALID_PAYLOAD` 에러 반환 (silent 무시 금지)
- *`char_offset > len(text)`* → clamp to len (우측 엔터)
- *`count > 1` + `page_break:true`* → 첫 번째 새 paragraph 만 페이지 분리, 나머지는 일반 Enter

## 3. payload 시맨틱

```jsonc
// 본문 모드
{
  "section": 0,
  "para": 3,
  "char_offset": -1,    // 기본 -1 = 본문 끝. 음수 → len, 양수 → min(value, len)
  "count": 1,           // 기본 1
  "style": {...},       // 옵션
  "page_break": false   // 옵션, 기본 false
}

// 셀 모드 (table_para 키 박힘으로 분기)
{
  "section": 0,
  "table_para": 2,
  "row": 1,
  "col": 0,
  "cell_para": 0,
  "char_offset": -1,
  "count": 1,
  "style": {...},
  "ctrl_idx": 0,        // 옵션 (서버 측에서 find_table_ctrl_idx 자동 호출)
  "cell_idx": 7         // 옵션 (서버 측에서 find_cell_idx 자동 호출)
  // page_break 키 — 박혀 있으면 INVALID_PAYLOAD 에러
}
```

## 4. 변경 자리

| 자리 | 변경 |
|---|---|
| [src/document_core/commands/edit_op.rs:230-315](../../src/document_core/commands/edit_op.rs) | `PressEnter` enum variant 신설. 본문 키 + 셀 키 모두 Option 으로 박고 apply 자리에서 분기 |
| [src/document_core/commands/edit_op.rs:1007 형제 자리](../../src/document_core/commands/edit_op.rs) | apply 분기 신설. 본문 모드는 `split_paragraph_native`, 셀 모드는 `split_paragraph_in_cell_native` 활용 |
| [src/document_core/commands/edit_op.rs:765 형제](../../src/document_core/commands/edit_op.rs) | `affected_range_for` 자리에 `PressEnter` 추가 |
| [rhwp-server/src/main.rs:905 형제](../../rhwp-server/src/main.rs) | `press_enter` REST workbench 핸들러 신설 |
| [src/wasm_api.rs:1367 형제](../../src/wasm_api.rs) | `pressEnter` WASM API 신설 (본문 모드 + 셀 모드 분기 인자) |

변경 *안 하는* 자리:
- `EditOperation::InsertParagraph` — 단계 4a 에서 제거
- `insert_paragraph_native` 헬퍼 — 단계 4a 에서 제거
- 옛 REST 핸들러 / WASM API — 단계 4a 에서 제거
- rhwp-studio UI — 단계 2a 에서 변경

## 5. 단계 1.1 — `PressEnter` variant + apply

```rust
// edit_op.rs

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EditOperation {
    // ... 기존 variants ...

    /// 한컴 Enter / Ctrl+Enter 와 동등. 커서 위치 (char_offset) 에서 Enter.
    ///
    /// payload 키 분기:
    /// - `table_para` 부재 → 본문 모드. 필수: section, para
    /// - `table_para` 박힘 → 셀 모드. 필수: section, table_para, row, col, cell_para
    ///
    /// char_offset 시맨틱 (i64):
    /// - -1 또는 음수 → 본문 끝 (= len(text))
    /// - 0 → 본문 시작
    /// - len 보다 큰 값 → clamp to len
    /// - 중간값 → 본문 분할
    ///
    /// count: 같은 자리에서 Enter N 회 (= N 개 빈 paragraph 누적). 기본 1.
    /// page_break:true → 첫 번째 새 paragraph 만 페이지 분리. 셀 모드 + true 면 INVALID_PAYLOAD.
    PressEnter {
        section: usize,
        // 본문 모드 키
        #[serde(default, skip_serializing_if = "Option::is_none")]
        para: Option<usize>,
        // 셀 모드 키
        #[serde(default, skip_serializing_if = "Option::is_none")]
        table_para: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        row: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        col: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cell_para: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ctrl_idx: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cell_idx: Option<usize>,
        // 공통 키
        #[serde(default = "default_char_offset")]
        char_offset: i64,
        #[serde(default = "one_count")]
        count: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        style: Option<PartialParagraphStyle>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page_break: Option<bool>,
    },
}

fn default_char_offset() -> i64 { -1 }
```

apply 분기:

```rust
EditOperation::PressEnter {
    section, para, table_para, row, col, cell_para, ctrl_idx, cell_idx,
    char_offset, count, style, page_break,
} => {
    let cell_mode = table_para.is_some();

    if cell_mode {
        // 셀 모드 — page_break 박혀 있으면 거부
        if page_break.unwrap_or(false) {
            return Err(HwpError::InvalidPayload(
                "셀 안 page_break 미지원. 셀 모드에서 page_break:true 박지 마세요.".to_string()
            ));
        }
        let table_para = table_para.unwrap();
        let row = row.ok_or_else(|| HwpError::InvalidPayload("셀 모드 row 누락".into()))?;
        let col = col.ok_or_else(|| HwpError::InvalidPayload("셀 모드 col 누락".into()))?;
        let cell_para = cell_para.ok_or_else(|| HwpError::InvalidPayload("셀 모드 cell_para 누락".into()))?;
        let ctrl_idx = match ctrl_idx {
            Some(idx) => *idx,
            None => self.find_table_ctrl_idx(*section, table_para)?,
        };
        let cell_idx = match cell_idx {
            Some(idx) => *idx,
            None => self.find_cell_idx(*section, table_para, ctrl_idx, row as u16, col as u16)?,
        };

        // cell paragraph 의 본문 길이 추출
        let cell_text_len = self.cell_paragraph_text_len(*section, table_para, ctrl_idx, cell_idx, cell_para)?;
        let resolved_offset = if *char_offset < 0 {
            cell_text_len
        } else {
            (*char_offset as usize).min(cell_text_len)
        };

        for i in 0..*count {
            let target_cell_para = cell_para + i;
            let target_offset = if i == 0 { resolved_offset } else { 0 };
            self.split_paragraph_in_cell_native(
                *section, table_para, ctrl_idx, cell_idx, target_cell_para, target_offset,
            )?;
            if let Some(s) = style {
                let props_json = partial_paragraph_style_to_native_json(s);
                self.apply_cell_para_format_native(
                    *section, table_para, ctrl_idx, cell_idx, target_cell_para + 1, &props_json,
                )?;
            }
        }
    } else {
        // 본문 모드
        let para = para.ok_or_else(|| HwpError::InvalidPayload("본문 모드 para 누락".into()))?;
        let para_text_len = self.document.sections[*section].paragraphs[para].text.chars().count();
        let resolved_offset = if *char_offset < 0 {
            para_text_len
        } else {
            (*char_offset as usize).min(para_text_len)
        };

        for i in 0..*count {
            let target_para = para + i;
            let target_offset = if i == 0 { resolved_offset } else { 0 };
            self.split_paragraph_native(*section, target_para, target_offset)?;
            if let Some(s) = style {
                let props_json = partial_paragraph_style_to_native_json(s);
                self.apply_para_format_native(*section, target_para + 1, &props_json)?;
            }
            // page_break — 첫 번째 새 paragraph 만
            if i == 0 && page_break.unwrap_or(false) {
                self.insert_page_break_native(*section, target_para + 1, 0)?;
            }
        }
    }
}
```

`affected_range_for` 자리 — 본문 모드는 `para` 부터 `para + count`, 셀 모드는 cell focus.

## 6. 단계 1.2 — REST 핸들러 + WASM API

`rhwp-server/src/main.rs` workbench 의 `press_enter` action 분기:

```rust
"press_enter" => {
    #[derive(serde::Deserialize)]
    struct Payload {
        section: usize,
        #[serde(default)] para: Option<usize>,
        #[serde(default)] table_para: Option<usize>,
        #[serde(default)] row: Option<usize>,
        #[serde(default)] col: Option<usize>,
        #[serde(default)] cell_para: Option<usize>,
        #[serde(default)] ctrl_idx: Option<usize>,
        #[serde(default)] cell_idx: Option<usize>,
        #[serde(default = "default_char_offset_i64")] char_offset: i64,
        #[serde(default = "one_count")] count: usize,
        #[serde(default)] style: Option<rhwp::document_core::PartialParagraphStyle>,
        #[serde(default)] page_break: Option<bool>,
    }
    let payload: Payload = serde_json::from_value(req.payload.clone())
        .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
    let op = rhwp::document_core::EditOperation::PressEnter {
        section: payload.section,
        para: payload.para,
        table_para: payload.table_para,
        row: payload.row,
        col: payload.col,
        cell_para: payload.cell_para,
        ctrl_idx: payload.ctrl_idx,
        cell_idx: payload.cell_idx,
        char_offset: payload.char_offset,
        count: payload.count,
        style: payload.style,
        page_break: payload.page_break,
    };
    let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
    Ok(Json(WorkbenchResp { seq, applied: "ops".to_string(), info: None, diff }))
}
```

`wasm_api.rs` WASM API:

```rust
#[wasm_bindgen(js_name = pressEnter)]
pub fn press_enter_wasm(
    &mut self,
    sec: u32,
    para: Option<u32>,
    table_para: Option<u32>,
    row: Option<u32>,
    col: Option<u32>,
    cell_para: Option<u32>,
    char_offset: i64,
    count: u32,
) -> Result<JsValue, JsValue> { ... }
```

## 7. 단계 1.3 — 테스트

### 단위 테스트 — `edit_op.rs` 안 `#[cfg(test)] mod tests`

본문 모드 — 7 케이스:
1. `char_offset = -1, count = 1` — 본문 끝 Enter. 원 본문 그대로, 새 빈 paragraph 가 +1 자리
2. `char_offset = 0, count = 1` — 본문 시작 Enter. 빈 paragraph 가 앞, 원 본문이 +1 자리
3. `char_offset = len, count = 1` — `-1` 과 동등
4. `char_offset = len + 100, count = 1` — clamp 시맨틱, `-1` 과 동등
5. `char_offset = 5, count = 1` (본문 = "ABCDEFGH") — "ABCDE" / "FGH" 분할
6. `char_offset = -1, count = 3` — 원 본문 + 3 개 빈 paragraph 누적
7. `style 옵션` + `page_break = true` — 새 paragraph 의 align/페이지 break

셀 모드 — 7 케이스 (위 1~7 동등 패턴, 셀 자리)

에러 케이스 — 5 건:
8. 본문 모드 + `para` 누락 → INVALID_PAYLOAD
9. 셀 모드 + `row` 누락 → INVALID_PAYLOAD
10. 셀 모드 + `page_break:true` → INVALID_PAYLOAD (셀 안 page_break 미지원)
11. 본문 모드 + `section` 범위 초과 → 적절한 에러
12. 본문 모드 + `para` 범위 초과 → 적절한 에러

총 19 케이스.

### REST e2e — `rhwp-server/tests/ir_slice_basic.rs` 형제

POST `/sessions/:id/workbench` `{"action":"press_enter","payload":{...}}` 19 케이스 동등. 응답의 `diff.after` 자리 IR 검사 + `info.paragraph_count` 검사.

### studio 시각 검증

`rhwp-studio/e2e/text-flow.test.mjs` 형제 자리:
- 빈 새 문서에 "Hello World" 박은 paragraph 자리 + `press_enter(char_offset=-1)` 호출 → "Hello World" 그대로 + 새 빈 paragraph 자리 시각 확인
- 같은 자리 `press_enter(char_offset=0)` 호출 → 빈 paragraph 가 앞, "Hello World" 가 +1 자리

## 8. 발견 사실 보고

테스트 자리에서 예상과 다른 동작 / 엣지 케이스 발견 시 [`UNIVA-rhwp/mydocs/feedback/task_press_enter_stage_1a_findings.md`](../feedback/) 자리 박음. 단계 1b (rdocx) 진입 전 반영.

## 9. 완료 기준

- `PressEnter` variant 신설 + apply 분기 + REST + WASM
- 단위 19 + REST 19 통과
- studio 시각 검증 통과
- `cargo build` + `cargo test` 0 실패
- 옛 `InsertParagraph` 동작 영향 0 (단계 4a 에서 제거)
- 발견 사실 보고서

## 10. 단계 보고서 자리

- `mydocs/working/task_press_enter_stage_1a_step1.md` — variant + apply
- `mydocs/working/task_press_enter_stage_1a_step2.md` — REST + WASM
- `mydocs/working/task_press_enter_stage_1a_step3.md` — 테스트 + studio 시각 검증
- `mydocs/report/task_press_enter_stage_1a_report.md` — 최종 보고 + 발견 사실
