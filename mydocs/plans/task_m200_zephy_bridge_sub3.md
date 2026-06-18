# Task #zephy-bridge Sub-3 — IR Compact 응답 서버 포팅

작성일 2026-06-07.

## 목표 한 줄

서버가 `get-ir-slice` 응답을 *모델이 읽기 좋은 평탄 형식* (init.md 가 약속한 형식 — type/runs/cell_locator/defaults) 으로 내보내도록, *옛 rhwp 원본의 ir-builder.ts 알고리즘* 을 *서버 측 Rust 모듈* 로 옮긴다.

## 1. 배경 — 왜 필요한가

### 1.1 진단

Sub-2 까지 *workbench 12 액션* 의 *쓰기 path* (모델이 명령을 발행하면 서버가 적용) 는 완성됐다. 그러나 *읽기 path* — 모델이 *현재 문서 좌표를 받아오는* `get-ir-slice` 응답 — 가 *세 군데에서 갈라져* 모델 입장에서 문서를 해석할 수 없다.

| layer | 현재 동작 | 모델 입장에서 보이는 결과 |
|---|---|---|
| `Paragraph` Serialize derive ([src/model/paragraph.rs](../../src/model/paragraph.rs)) | char_shapes·line_segs·raw_header_extra 등 *내부 raw 필드* 24개 노출. `controls`/`ctrl_data_records` 는 `#[serde(skip)]` (Control enum 이 Serialize 미구현) | *표가 들어있는 문단* 이 `text:""` + `controls_len:1` 만 보이고 *어떤 표인지 행렬 크기·셀 내용* 어느 것도 알 수 없음 |
| `/sessions/:id/ir-slice` compact 분기 ([server/src/main.rs:1040-1046](../../server/src/main.rs#L1040-L1046)) | `{para, text, para_shape_id}` *세 필드만* 반환 | run-level 글자 서식 (bold/color/font-size) 평탄화 없음. 모델이 "문단이 있다" 정도만 알 수 있음 |
| 노트북 cell 3 `_handle_get_ir_slice` ([hwp_sub_agent_simulation_ssr.ipynb](../../../hwp_sub_agent_simulation_ssr.ipynb)) | `mode` 키만 query 로 변환. *LLM 이 보내는 `compact: true/false` 키는 무시* | 모델 요청과 무관하게 항상 default `mode=auto` → 25자 미만 문서는 raw 로 떨어짐 |

`init.md` ([26ZEPHY-skills/.../references/init.md](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/init.md)) 가 약속한 compact 응답 형식은 *어느 layer 에도 구현되어 있지 않다*. 약속 vs 현재 비교:

| init.md 약속 | 현재 응답 |
|---|---|
| `paragraphs[].type: "text" \| "table"` | 없음 |
| `runs[]` (`char_offset`, `text`, 평탄 `style` 객체) | 없음 — `text` 한 줄과 `char_shapes` 배열만 |
| 표 문단의 `rows`/`cols`/`cells[]` | `controls_len:1` 만 |
| 셀 안 문단의 `cell_locator` (`table_para`/`row`/`col`/`cell_para`) | 없음 |
| `defaults` 박스 (run/paragraph 기본값) | 없음 |
| 단일 run + 스타일 없음 → `runs` 생략, `text` 직속 | 없음 |

### 1.2 같이 발견된 보조 이슈

표 정보가 *raw 모드* 에서도 빠지는 까닭은 `Paragraph.controls` 가 `#[serde(skip)]` 인데 `Control` enum 자체는 *Serialize 미구현*. 본 작업의 *주제 외* 라 별도 substage 로 분리 (§7 참조).

## 2. 결정된 구조 — 서버 측 Rust 포팅

### 2.1 큰 그림

```
                          ┌─ 노트북 (LLM 측) ────────────┐
                          │ GET /sessions/:id/ir-slice  │ ← (1) 형식이 모델 입장에서 문서를 읽을 수 없음
                          │ POST /workbench (편집 명령)  │   (Sub-2 에서 완성)
                          └─────────────┬───────────────┘
                                        │ HTTP
                                        ▼
┌───────────────────────────────────────────────────────────────────────┐
│ 서버 (Rust, 7710)                                                      │
│   ─ DocumentCore 1부 (서버판 문서 본체)                                  │
│   ─ sqlite 변경 일지                                                    │
│   ─ broadcast 채널                                                     │
│                                                                       │
│   [Sub-3 신설]                                                         │
│   server/src/ir_compact.rs                                            │
│      ─ DocumentCore 의 내부 Paragraph·CharShape·ParaShape·Cell 구조를    │
│        읽어, 모델 친화적 평탄 JSON 으로 변환                              │
│      ─ 알고리즘 청사진: rhwp/rhwp-studio/src/llm-replay/ir-builder.ts    │
│      ─ 값 변환 청사진: 같은 디렉토리 style-map.ts                         │
│      ─ 응답 형식 청사진: 같은 디렉토리 types.ts                           │
└────────────────────┬──────────────────────────────────────────────────┘
                     │ WS 양방향 (broadcast)
                     ▼
┌───────────────────────────────────────────────────────────────────────┐
│ rhwp-studio (브라우저)                                                  │
│   변경 없음 — 본 작업은 *읽기 path* 만 다룸                              │
└───────────────────────────────────────────────────────────────────────┘
```

### 2.2 왜 서버 측 포팅인가

세 가지 후보 (A: rhwp 본체 `ir_view.rs` 확장 / B: 서버 측 신규 모듈 / C: 노트북 측 변환) 중 **B 안 (서버 측)** 채택.

이유 셋:
1. **데이터 접근 layer 가 이미 모두 존재**. `DocumentCore::get_char_properties_at_native`·`get_para_properties_at_native`·`get_cell_info_native`·`get_cell_properties_native`·`get_table_dimensions_native`·`get_cell_paragraph_count_native` 가 모두 `pub` 메서드로 정의됨 ([formatting.rs:17-146](../../src/document_core/commands/formatting.rs#L17-L146)·[table_ops.rs:306-441](../../src/document_core/commands/table_ops.rs#L306-L441)·[text_editing.rs:2120-2201](../../src/document_core/commands/text_editing.rs#L2120-L2201)). 서버가 `s.core.X` 로 직접 호출 가능 — *wasm round-trip 불요*.
2. **rhwp 본체 무변경 → 회귀 0**. `ir_view.rs` 의 schema_version 1 그대로 유지. 본체 cargo test 영향 없음.
3. **서버가 SoT (Sub-1 원칙)**. 변환 결과가 *클라이언트마다 다를 수 없는* 정형 응답이어야 함. 노트북 측에서 변환하면 *다른 client (직접 LLM 호출 등)* 가 같은 형식을 못 받음.

### 2.3 옛 ts 코드 ↔ 새 Rust 모듈 매핑

| 옛 ts (`rhwp/rhwp-studio/src/llm-replay/`) | 새 Rust (`UNIVA-rhwp/server/src/ir_compact.rs`) |
|---|---|
| `types.ts::RunStyle` | `pub struct RunStyle` |
| `types.ts::ParagraphStyle` | `pub struct ParagraphStyle` |
| `types.ts::CellStyle` + `CellBorderSpec` | `pub struct CellStyle` + `CellBorderSpec` |
| `types.ts::IRRun` | `pub struct IrRun` |
| `types.ts::IRTextParagraph`·`IRTableParagraph`·`IRTableCell`·`CellLocator` | 동등 struct + `enum IrParagraph::{Text, Table}` |
| `types.ts::IRDocMeta`·`IRSlice` | `pub struct IrDocMeta`·`IrSlice` |
| `style-map.ts::charPropsToRunStyle` | `fn char_shape_to_run_style(cs: &CharShape, raw: &RawCharShape, lang_idx: usize) -> RunStyle` |
| `style-map.ts::paraPropsToParaStyle` | `fn para_shape_to_para_style(ps: &ParaShape) -> ParagraphStyle` |
| `style-map.ts::cellPropsToCellStyle` | `fn cell_to_cell_style(cell: &Cell) -> CellStyle` |
| `style-map.ts::runStyleEquals` | `impl PartialEq for RunStyle` (derive) — 또는 명시적 `fn run_style_equals` |
| `ir-builder.ts::buildIRSlice` | `pub fn build_ir_slice(core: &DocumentCore, opts: &BuildOptions) -> IrSlice` |
| `ir-builder.ts::collectRuns` | `fn collect_runs(...) -> Vec<IrRun>` |
| `ir-builder.ts::computeDocDefaults` | `fn compute_doc_defaults(ir: &IrSlice) -> DocDefaults` |
| `ir-builder.ts::compactIRSlice` | `pub fn compact_ir_slice(ir: IrSlice) -> CompactIrSlice` |
| `ir-builder.ts::omitDefaults`·`compactRun`·`compactText`·`compactCell`·`compactTable`·`compactBorder` | 동등 free 함수, output 은 `serde_json::Value` |

총 약 500 줄 Rust 1 파일 (`server/src/ir_compact.rs`).

## 3. 응답 JSON 의 정확한 형식

### 3.1 init.md 와의 1:1 대조

`init.md §2` 가 약속한 예제 그대로:

```json
{
  "doc_meta": {
    "edit_session_id": "cli_<file_id>",
    "page": 1,
    "total_pages": 1,
    "anchor": {"sec": 0, "para_start": 0, "para_end": 27}
  },
  "paragraphs": [
    { "id": "p_0_0", "sec": 0, "para": 0, "type": "text",
      "style": {"align": "justify"},
      "runs": [
        { "char_offset": 0,
          "text": "굵은 빨강 제목",
          "style": {"bold": true, "color": "#FF0000", "font-size": 22} }
      ] },
    { "id": "p_0_5", "sec": 0, "para": 5, "type": "text",
      "text": "단일 run 문단은 runs 배열 대신 text 한 줄로" },
    { "id": "p_0_10", "sec": 0, "para": 10, "type": "table",
      "rows": 3, "cols": 4,
      "cells": [
        { "row": 0, "col": 0, "paragraphs": [
            { "id": "p_0_10_c0_0_0", "sec": 0, "para": -1, "type": "text",
              "cell_locator": {"table_para": 10, "row": 0, "col": 0, "cell_para": 0},
              "text": "셀 안 문단" }
        ]}
      ] },
    { "id": "p_0_-1", "sec": 0, "para": -1,
      "cell_locator": {"table_para": 10, "row": 0, "col": 0, "cell_para": 0},
      "text": "셀 안 문단" }
  ],
  "defaults": {
    "run": {
      "bold": false, "italic": false, "underline": false, "strikethrough": false,
      "color": "#000000", "highlight": null,
      "char-spacing": 0, "char-width": 100, "vertical-align": "baseline",
      "font-size": 11, "font-name": "함초롬바탕"
    },
    "paragraph": { "align": "left", "indent": 0, "line-height": 160 }
  }
}
```

### 3.2 압축 규칙 (compact 의 본질)

`ir-builder.ts` 의 `compactIRSlice` 가 *원본 그대로* 정의한 4개 규칙. Rust 포팅에서 *동일 적용*:

1. **`run.length` 제거** — `text.length` 로 derive 가능.
2. **style 키 omit** — `defaults` 와 같은 값이면 응답에서 *생략*. 모델은 *생략된 키를 defaults 값으로 복원* 해 읽음 (init.md §3).
3. **단일 run + 스타일 없음** — `paragraph.runs` 대신 `paragraph.text` 직속.
4. **cell.style.border 4면 동일 시** — `{all: ...}` 1키로 축약.

### 3.3 raw 모드 응답 (대조)

raw 모드는 *현재 동작 그대로 유지*. 디버깅·기존 e2e 호환 목적.

```json
{
  "section": 0,
  "para_start": 0,
  "para_end": N,
  "mode": "raw",
  "paragraphs": [ /* Paragraph Serialize derive 의 직접 결과 */ ]
}
```

## 4. 데이터 접근 — DocumentCore 의 어떤 자리를 읽는가

서버의 ir_compact 는 *wasm round-trip 없이* `DocumentCore` 의 내부 struct 를 직접 읽는다.

| 옛 ts 가 호출한 wasm 함수 | 새 Rust 가 읽을 자리 |
|---|---|
| `wasm.getParagraphCount(sec)` | `core.document.sections[sec].paragraphs.len()` |
| `wasm.getParagraphLength(sec, para)` | `core.document.sections[sec].paragraphs[para].text.chars().count()` |
| `wasm.getControlTextPositions(sec, para)` | `core.document.sections[sec].paragraphs[para].controls` 인덱스 |
| `wasm.getTableDimensions(sec, para, ci)` | `core.get_table_dimensions_native(sec, para, ci)` ([table_ops.rs:306](../../src/document_core/commands/table_ops.rs#L306)) → JSON 파싱하거나, `Control::Table(t)` 매치 후 `t.row_count`/`t.col_count` 직접 |
| `wasm.getCharPropertiesAt(sec, para, off)` | `core.get_char_properties_at_native(sec, para, off)` JSON parse, 또는 `paragraph.char_shape_id_at(off)` → `styles.char_styles[id]` (CharShape struct) 직접 + `document.doc_info.char_shapes[id]` (raw base_size) — *후자가 빠르고 깔끔* |
| `wasm.getParaPropertiesAt(sec, para)` | `paragraph.para_shape_id` → `styles.para_styles[id]` (ParaShape struct) 직접 |
| `wasm.getCellInfo(sec, parentPara, ci, cellIdx)` | `core.get_cell_info_native(...)` ([table_ops.rs:341](../../src/document_core/commands/table_ops.rs#L341)) — row/col/rowSpan/colSpan |
| `wasm.getCellProperties(...)` | `core.get_cell_properties_native(...)` ([table_ops.rs:441](../../src/document_core/commands/table_ops.rs#L441)) — fillColor·width·height·border·verticalAlign |
| `wasm.getCellParagraphCount(...)` | `core.get_cell_paragraph_count_native(...)` ([text_editing.rs:2120](../../src/document_core/commands/text_editing.rs#L2120)) |
| `wasm.getCellParagraphLength(...)` | `core.get_cell_paragraph_length_native(...)` ([text_editing.rs:2175](../../src/document_core/commands/text_editing.rs#L2175)) |
| `wasm.getTextInCell(...)` | `core.get_text_in_cell_native(...)` ([text_editing.rs:2201](../../src/document_core/commands/text_editing.rs#L2201)) |
| `wasm.getCellParaPropertiesAt(...)` | `core.get_cell_para_properties_at_native(...)` ([formatting.rs:128](../../src/document_core/commands/formatting.rs#L128)) |
| `wasm.getCellCharPropertiesAt(...)` | `core.get_cell_char_properties_at_native(...)` ([formatting.rs:36](../../src/document_core/commands/formatting.rs#L36)) |
| `wasm.getTextRange(sec, para, off, len)` | `paragraph.text.chars().skip(off).take(len).collect::<String>()` 직접 |

*native 메서드의 String JSON* 과 *struct 직접 접근* 두 path 가 모두 열려 있다. 본 작업에서는 *직접 접근* 을 우선 — round-trip 비용 0, type-safe.

## 5. 값 변환 공식

`style-map.ts` 의 변환을 Rust 로 옮긴다. 단위·enum 매핑·기본값이 핵심.

### 5.1 글자 서식 (CharShape → RunStyle)

```rust
fn char_shape_to_run_style(cs: &CharStyle, raw_cs: &CharShape, lang_idx: usize) -> RunStyle {
    let font_family = primary_font_name(&cs.font_family_for_lang(lang_idx));
    RunStyle {
        bold:           Some(cs.bold),
        italic:         Some(cs.italic),
        underline:      Some(!matches!(cs.underline, UnderlineType::None)),
        strikethrough:  Some(cs.strike_out),
        color:          Some(color_ref_to_css(cs.text_color)),         // 0xRRGGBB → "#RRGGBB"
        highlight:      cs.shade_color.map(color_ref_to_css),          // None 이면 null
        font_size:      Some((raw_cs.base_size as f64) / 100.0),       // HWPUNIT → pt
        font_name:      Some(font_family),
        char_spacing:   Some(cs.spacings.first().copied().unwrap_or(0)),
        char_width:     Some(cs.ratios.first().copied().unwrap_or(100)),
        vertical_align: Some(if cs.subscript { "sub" } else if cs.superscript { "super" } else { "baseline" }),
    }
}
```

핵심 단위 변환:
- `font-size` = `base_size / 100` (HWPUNIT — 1pt = 100). `style-map.ts:34` `+(p.fontSize * 0.01).toFixed(2)` 동등.
- `color` = `ColorRef` → `#RRGGBB` 16진. `helpers::color_ref_to_css` 이미 존재.
- `vertical-align` = `subscript ? "sub" : superscript ? "super" : "baseline"` (셋 중 하나).

### 5.2 문단 서식 (ParaShape → ParagraphStyle)

```rust
fn para_shape_to_para_style(ps: &ParaShape) -> ParagraphStyle {
    ParagraphStyle {
        align:       Some(alignment_to_str(ps.alignment)),  // "left"/"right"/"center"/"justify"/"distribute"/"split"
        indent:      Some(ps.indent),                        // HWPUNIT
        line_height: if matches!(ps.line_spacing_type, LineSpacingType::Percent) {
            Some(ps.line_spacing)                            // % 단위
        } else {
            None                                             // Percent 가 아니면 모델에게 노출하지 않음
        },
    }
}
```

`ps.alignment` 가 enum 이면 `Justify`·`Left` 같은 PascalCase 를 *소문자 + 하이픈* 으로 변환. `style-map.ts:73` 의 `p.alignment as ParagraphStyle['align']` 와 동등 (이미 소문자 문자열 형태).

### 5.3 셀 서식 (Cell → CellStyle)

```rust
fn cell_to_cell_style(cell: &Cell) -> CellStyle {
    CellStyle {
        bgcolor: cell.fill_color.map(color_ref_to_css),
        width:   Some(cell.width),                       // HWPUNIT
        height:  Some(cell.height),                      // HWPUNIT
        border:  cell_border_to_spec(cell),              // 4면 → CellBorderSpec 4개
        vertical_align: Some(match cell.vertical_align {
            0 => "top",
            1 => "middle",
            2 => "bottom",
            _ => "top",
        }),
    }
}
```

### 5.4 RunStyle 비교 — 인접 run 병합 판단

```rust
impl PartialEq for RunStyle { /* derive 가능 — 모든 필드 비교 */ }
```

`style-map.ts:runStyleEquals` 가 *11 필드 비교* — `Eq` derive 동등.

### 5.5 색상·정렬·언어 helper

- `color_ref_to_css(c: ColorRef) -> String` — `format!("#{:02X}{:02X}{:02X}", r, g, b)`. `helpers.rs` 에 이미 있음.
- `primary_font_name(fonts: &str) -> &str` — comma 분리된 폰트 체인의 첫 이름. `renderer::style_resolver::primary_font_name` 이미 있음.
- `detect_lang_category(ch: char) -> usize` — 한글/영문/한자 등 7 분류. 이미 있음. 첫 글자의 lang_idx 로 font 결정.

## 6. defaults 산정 알고리즘

`ir-builder.ts::computeDocDefaults` 의 *원본 그대로*:

```rust
fn compute_doc_defaults(ir: &IrSlice) -> DocDefaults {
    let mut sizes: Vec<f64> = vec![];
    let mut fonts: Vec<String> = vec![];
    visit_paragraphs(ir, |p| match p {
        IrParagraph::Text(tp) => for run in &tp.runs {
            if let Some(s) = run.style.font_size { sizes.push(s); }
            if let Some(f) = &run.style.font_name { fonts.push(f.clone()); }
        },
        IrParagraph::Table(tt) => for cell in &tt.cells {
            for inner in &cell.paragraphs { /* 재귀 */ }
        },
    });
    DocDefaults {
        run: RunStyle {
            bold: Some(false),
            italic: Some(false),
            underline: Some(false),
            strikethrough: Some(false),
            color: Some("#000000".into()),
            highlight: None,
            char_spacing: Some(0),
            char_width: Some(100),
            vertical_align: Some("baseline".into()),
            font_size: Some(mode(&sizes).unwrap_or(10.0)),    // 가장 흔한 size, 없으면 10
            font_name: Some(mode(&fonts).unwrap_or("맑은 고딕".into())),
        },
        paragraph: ParagraphStyle {
            align: Some("left".into()),
            indent: Some(0),
            line_height: Some(160),
        },
    }
}
```

`mode(arr)` = *최빈값*. JSON 직렬화 키로 그룹화해 가장 흔한 값 반환. 동률이면 *먼저 등장한 값* (`ir-builder.ts:321-335` 와 동일).

### 6.1 모델이 응답을 해석하는 규칙 (init.md §3)

모델 입장의 *복원 규칙* 도 정리해 둔다 (spec 변경은 없지만, 본 작업이 *implementation 측 정합* 을 보장해야 함):

- run.style 의 키 중 *생략된 키* → `defaults.run` 의 같은 키 값으로 본다.
- paragraph.style 의 키 중 *생략된 키* → `defaults.paragraph` 의 값.
- `runs` 가 없고 `text` 만 있는 문단 → `runs: [{char_offset:0, text, style: {}}]` 와 동등 (스타일 전부 defaults).
- cell.style.border.all 이 있으면 4면 모두 같음.

## 7. endpoint 동작 — mode 정책

[server/src/main.rs:983-1057](../../server/src/main.rs#L983-L1057) 의 `ir_slice_handler` 변경:

```
query.mode:
  "compact" (default if 미지정)  →  compact 응답 (§3.1)
  "raw"                          →  raw 응답 (§3.3, 현재 동작 그대로)
  "auto"                         →  compact (init.md 의 default 정책 — auto 는 호환 alias 로 유지)
```

기존 `total_chars < 5000` 기반 auto 분기 *제거*. 25자 문서가 raw 로 떨어지는 사고 방지. *auto 와 compact 가 사실상 동의어*.

`IrSliceQuery::mode` 의 `default_ir_slice_mode()` 가 `"auto"` 반환 — 그대로 두되 *auto → compact 매핑* 으로 본문 변경.

## 8. 노트북 라우터 변경 점

[hwp_sub_agent_simulation_ssr.ipynb cell 3 `_handle_get_ir_slice`](../../../hwp_sub_agent_simulation_ssr.ipynb):

```python
# 현재
if 'mode' in payload:
    query['mode'] = str(payload['mode'])

# 변경 후 — compact 키도 인식
if 'mode' in payload:
    query['mode'] = str(payload['mode'])
elif 'compact' in payload:
    query['mode'] = 'raw' if payload['compact'] is False else 'compact'
```

`{"compact": false}` → `mode=raw`, `{"compact": true}` 또는 키 없음 → default(compact). init.md §1 의 "디버깅용 raw 가 정말 필요하면 `{compact: false}` 명시" 와 정합.

## 9. 호환성 정책

| 항목 | 정책 |
|---|---|
| `ir-slice` raw 모드 응답 형식 | *불변*. 기존 e2e 테스트 (sub2-ir-slice.test.mjs 등) 회귀 0. |
| `ir-slice` compact 모드 응답 형식 | *교체*. 기존 3 필드 응답을 사용하는 client 가 있다면 깨짐 — 현재까지 *내부 e2e 도 사용 안 함* 이라 영향 0. |
| `mode=auto` query | *유지 (compact 와 동의어)*. 옛 client 가 auto 를 보내도 compact 받음. |
| 기존 `/sessions/:id/ir` endpoint (전체 IR 뷰, `to_ir_view`) | *불변*. Sub-3 의 범위 외. |
| `IR_VIEW_SCHEMA_VERSION` (`ir_view.rs`) | *불변* — 본 작업은 `ir_view.rs` 와 *별도 path* (ir_compact.rs). |

## 10. 검증 계획

### 10.1 unit test (server/src/ir_compact.rs::tests)

3 fixture 로 핵심 path 검증:

1. **text only** — 빈 hwpx (blank_hwpx.hwpx) 로드 후 `insert_text` 한 번 → ir_compact 호출 → `paragraphs[0].text == "..."`, `defaults.run.font_size` 정상.
2. **mixed runs** — `insert_text` 두 번 + `set_char_shape` 으로 한쪽만 bold → `paragraphs[0].runs.len() == 2`, 첫 run.style.bold == true.
3. **1×1 table with cell text** — `insert_table` + `replace_cell_runs` → `paragraphs` 에 *table 문단* 1 개 + (별도 평탄 entry 로) *cell paragraph* 1 개. `cell_locator.table_para` / `.row` / `.col` 정확.

### 10.2 e2e

신규 `rhwp-studio/e2e/sub3-ir-compact.test.mjs`:
- 빈 문서 + `insert_text` "A" + `set_char_shape bold` + `insert_table 2x2` + `replace_cell_runs(0,0)` "B"
- `GET /ir-slice?mode=compact` 호출
- 응답이 다음 *모두 만족*:
  - `paragraphs[0].runs[0].style.bold == true`
  - `paragraphs` 에 `type:"table"` 문단 존재
  - 그 table 의 `rows==2, cols==2`
  - cell(0,0) paragraphs[0].text == "B"
  - `defaults` 박스 존재
- 같은 호출 `mode=raw` 도 동시 검증 — 기존 raw 형식 그대로

### 10.3 시연

노트북 cell 5·6 *그대로* (변경 없음). cell 3 의 라우터만 fix 후, LLM 이 `hwp-doc-patch get-ir-slice` 호출 시:
- 표가 있는 hwp 파일을 *파일 열기* 로 로드
- 모델이 IR 받음 → `paragraphs` 에 `type:"table"` + `rows`·`cols` + 셀 텍스트 보여야 함
- 모델이 표 셀 좌표를 정확히 박아 `replace-cell-runs` 호출 → 브라우저에 즉시 반영

## 11. DoD

| 조건 | 검증 방법 |
|---|---|
| 1. ir_compact 모듈이 ir-builder.ts 의 12 함수와 1:1 대응 | spec §2.3 매핑표 + cargo test 통과 |
| 2. compact 응답이 init.md §2 예제 형식과 1:1 일치 | unit test 3 fixture + e2e sub3-ir-compact |
| 3. defaults 박스의 11 키 + 3 키 (paragraph) 모두 채워짐 | unit test assertion |
| 4. 압축 4 규칙 (length 제거 / style omit / 단일 run text 직속 / border all) 동작 | e2e assertion |
| 5. raw 모드 회귀 0 | 기존 e2e sub2-ir-slice 그대로 통과 |
| 6. 노트북 라우터가 `compact: true/false` 키 인식 | 노트북 self-test cell 4 + LLM 시연 |
| 7. 사용자 수동 시연 — 표가 있는 hwp 로드 → 모델이 셀 좌표 박아 편집 | 시연 통과 보고 |

## 12. Sub-3 의 다음 단계로 미루는 항목

본 작업이 *읽기 path* 하나에 집중. Sub-3 의 후속 sub 로 미루는 항목:

1. **Control Serialize derive 추가** — raw 모드에서도 표·그림·도형 컨트롤이 직렬화되도록. 본 작업은 *compact 응답에서 표를 노출* 하므로 모델 입장 문제는 해결되지만, *raw 모드 디버깅 사용자* 입장에서는 여전히 controls 가 빠짐.
2. **`doc_meta.total_pages` 정확화** — 현재 ts 도 `1`/`1` 하드코딩. paginator 결과를 받아오는 별도 작업.
3. **ws broadcast 의 IR delta 전파** — 본 작업은 *full IR slice 응답*. 미래 확장 — 편집마다 *변경된 paragraph 만 IR delta 로* WS 로 흘리는 방식. 대용량 문서 모델 응답 토큰 절감.
4. **char_shape_id 변경 broadcast 시 cell_idx 같은 사전 계산 부가 필드** — Sub-2 의 cell_idx 패턴 확장.

## 13. 참고 자료

- *알고리즘 청사진*: [rhwp/rhwp-studio/src/llm-replay/ir-builder.ts](../../../rhwp/rhwp-studio/src/llm-replay/ir-builder.ts)
- *값 변환 청사진*: [rhwp/rhwp-studio/src/llm-replay/style-map.ts](../../../rhwp/rhwp-studio/src/llm-replay/style-map.ts)
- *응답 형식 청사진*: [rhwp/rhwp-studio/src/llm-replay/types.ts](../../../rhwp/rhwp-studio/src/llm-replay/types.ts)
- *모델 가이드 (응답 형식 약속)*: [26ZEPHY-skills/.../hwp-doc-edit/references/init.md](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/init.md)
- *모델 편집 명령 가이드*: [edit-phase.md](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/edit-phase.md)
- *현재 ir-slice 구현*: [server/src/main.rs:983-1057](../../server/src/main.rs#L983-L1057)
- *기존 안정 IR 뷰 (참고만, 변경 없음)*: [src/model/ir_view.rs](../../src/model/ir_view.rs)
- *DocumentCore native 메서드*: [src/document_core/commands/formatting.rs](../../src/document_core/commands/formatting.rs) / [table_ops.rs](../../src/document_core/commands/table_ops.rs) / [text_editing.rs](../../src/document_core/commands/text_editing.rs)
- *Sub-1·2 보고서*: [task_m200_zephy_bridge_report.md](../report/task_m200_zephy_bridge_report.md) (Sub-1) / [task_m200_zephy_bridge_sub2_report.md](../report/task_m200_zephy_bridge_sub2_report.md) (Sub-2)
