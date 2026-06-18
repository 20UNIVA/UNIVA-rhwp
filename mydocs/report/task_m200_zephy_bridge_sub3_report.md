# Task #zephy-bridge Sub-3 최종 결과 보고서 — IR Compact 응답 서버 포팅

작성일 2026-06-07.

## 작업 요약

옛 rhwp 원본의 `rhwp/rhwp-studio/src/llm-replay/ir-builder.ts` 알고리즘을 서버 측 Rust 모듈 (`server/src/ir_compact.rs`) 로 옮겨, `GET /sessions/:id/ir-slice` 가 init.md 가이드의 *모델 친화적 평탄 형식* (type/runs/cell_locator/defaults) 으로 응답하도록 한다. *서버가 모든 IR 표현의 SoT (진실 원천)* 라는 Sub-1/2 의 원칙을 *읽기 path* 에 확장.

총 22 commit, 신규 1 파일 (1951 lines), 수정 3 파일.

상세 설계는 [task_m200_zephy_bridge_sub3.md](../plans/task_m200_zephy_bridge_sub3.md), 구현 계획은 [task_m200_zephy_bridge_sub3_impl.md](../plans/task_m200_zephy_bridge_sub3_impl.md).

## 발견된 문제 (Sub-2 종료 후)

[Sub-2 보고서](task_m200_zephy_bridge_sub2_report.md) 까지의 *workbench 12 액션* 으로 *쓰기 path* 는 완성됐으나, *모델이 현재 문서 좌표를 받아오는* `get-ir-slice` 응답이 *세 군데에서 갈라져* 모델 입장에서 문서를 해석할 수 없었다.

| layer | 현재 동작 | 모델 입장 |
|---|---|---|
| `Paragraph` Serialize derive | `char_shapes`·`line_segs`·`raw_header_extra` 등 *내부 raw 필드 24개* + controls 통째 `#[serde(skip)]` | *표가 들어있는 문단* 이 `text:""` + `controls_len:1` 만 보임. 어떤 표인지 알 수 없음 |
| `/sessions/:id/ir-slice` compact 분기 | `{para, text, para_shape_id}` *세 필드만* | run-level 글자 서식 (bold/color/font-size) 평탄화 0. "문단이 있다" 정도만 알 수 있음 |
| 노트북 cell 3 라우터 | `mode` 키만 query 변환 | `compact: true/false` 키는 *무시* — 모델 요청과 무관하게 default(auto) → 25자 미만은 raw |

init.md 가이드가 약속한 `paragraphs[].type` / `runs[]` (평탄 style) / `rows`/`cols`/`cells` / `cell_locator` / `defaults` 박스 형식은 *어느 layer 에도 구현된 적이 없었다*.

## DoD (Definition of Done) 통과 여부

| 조건 | 결과 |
|---|---|
| 1. ir_compact 모듈이 ir-builder.ts 의 12+ 함수와 1:1 대응 | ✅ ir_compact.rs 의 약 30개 함수·struct 가 ts 원본 정합. 41 cargo test pass. |
| 2. compact 응답이 init.md §2 예제 형식과 1:1 일치 | ✅ sub3-ir-compact.test.mjs 의 6 검증 (defaults / type:"text" / runs/style.bold / type:"table" / rows·cols·cells / cell_locator 평탄 entry) 통과. |
| 3. defaults 박스의 run 11 키 + paragraph 3 키 모두 채워짐 | ✅ `compute_doc_defaults_from_empty_slice` test 가 bold/italic/underline/strikethrough/color/highlight/char-spacing/char-width/vertical-align/font-size/font-name + align/indent/line-height 검증. |
| 4. 압축 4 규칙 (length 제거 / style omit / 단일 run text 직속 / border all) 동작 | ✅ `compact_text_single_run_inline`·`compact_text_styled_run_keeps_runs`·`compact_border_4sides_same_to_all`·`omit_run_defaults_drops_matching` 4 test 검증. |
| 5. raw 모드 회귀 0 | ✅ Sub-1 의 `ws-bridge.test.mjs` + Sub-2 의 `sub2-replace-runs`·`sub2-canvas-insert-text`·`sub2-audit-diff-ir-slice`·`sub2-partial-update`·`sub2-set-paragraph-style` 6 e2e 모두 통과. |
| 6. 노트북 라우터가 `compact: true/false` 키 인식 | ✅ cell 3 `_handle_get_ir_slice` 의 `compact` 키 분기 추가. `{"compact": false}` → `mode=raw`, 그 외 default(compact). |
| 7. 사용자 수동 시연 — 표가 있는 hwp 로드 → 모델이 셀 좌표 박아 편집 | ⏳ 사용자 검증 영역 — *시연 안내는 §시연 안내* 참조. |

## 신규·수정 파일

### 신규

| 파일 | 줄 수 | 역할 |
|---|---|---|
| `server/src/ir_compact.rs` | 1951 | IR 타입 + 변환 + 압축 + 41 unit test |
| `rhwp-studio/e2e/sub3-ir-compact.test.mjs` | 123 | compact 응답 6 검증 + raw 회귀 |
| `UNIVA-rhwp/mydocs/plans/task_m200_zephy_bridge_sub3.md` | — | spec (13 절) |
| `UNIVA-rhwp/mydocs/plans/task_m200_zephy_bridge_sub3_impl.md` | — | 구현 계획서 (7 phase / 20 task) |

### 수정

| 파일 | 변경 |
|---|---|
| `src/document_core/mod.rs` | `DocumentCore::styles()` *read-only accessor* 7 줄 추가. `document()` 는 이미 존재 (`commands/document.rs:626`). 본체의 *유일한 변경*. |
| `server/src/main.rs` | `mod ir_compact;` 등록 + `ir_slice_handler` 의 compact 분기 교체 + mode 정책 (`"raw"` 외 전부 compact). raw 분기는 *원형 보존*. |
| `rhwp-studio/e2e/sub2-helpers.mjs` | `getIrSlice(base, fileId, opts)` helper 추가 (이미 일부 존재했을 수 있음 — implementer 가 정합 확인). |
| `hwp_sub_agent_simulation_ssr.ipynb` cell 3 | `_handle_get_ir_slice` 에 `compact: true/false` 키 → `mode` query 변환 분기 6 줄 추가. *git 외부 파일* — 본 보고서가 변경 기록의 권위. |

## 큰 그림 — 어떻게 동작하는가

```
                          ┌─ 노트북 (LLM 측) ────────────┐
                          │ GET /sessions/:id/ir-slice  │
                          │ payload={compact:true/false} │
                          │   → query{mode:compact/raw} │ ← 새 변환 (Phase 7)
                          └─────────────┬───────────────┘
                                        │ HTTP
                                        ▼
┌───────────────────────────────────────────────────────────────────────┐
│ 서버 (rhwp-server, Rust, 7710)                                          │
│                                                                       │
│  ir_slice_handler (main.rs)                                           │
│   ─ mode == "raw"   → 기존 Paragraph Serialize derive (회귀 0)         │
│   ─ 그 외 (compact/auto/default) → ir_compact::build_compact_ir_slice  │
│                                                                       │
│  ir_compact.rs (Sub-3 신규, 1951 lines)                                │
│   ─ 타입: RunStyle/ParagraphStyle/CellStyle/IrRun/IrParagraph (untagged)│
│           /IrSlice/DocDefaults/CompactIrSlice                          │
│   ─ 값 변환: char_shape_to_run_style (HWPUNIT→pt, ColorRef→#RRGGBB,    │
│              vertical-align enum→문자열, shade_color sentinel 처리)    │
│              + para_shape_to_para_style (Percent 만 line-height 노출)  │
│              + cell_to_cell_style (BorderFill 테이블 1-indexed 조회)   │
│   ─ 빌더: build_text_paragraph + build_cell_paragraph (cell_locator)   │
│           + try_build_cell + build_table_paragraph                     │
│           + build_paragraph (control 검사 분기, 표 + 셀 평탄 entry)    │
│           + collect_runs (인접 동일 스타일 병합)                        │
│   ─ 압축: compute_doc_defaults (mode() 최빈값 + 동률 시 first)         │
│           + omit_run/para_style_defaults                               │
│           + compact_run/text/border/cell/table                         │
│           + compact_ir_slice 진입                                      │
│                                                                       │
│  DocumentCore 접근 — `core.document()` (기존) + `core.styles()` (Sub-3)│
└────────────────────┬──────────────────────────────────────────────────┘
                     │ WS 양방향 broadcast — 변경 없음
                     ▼
┌───────────────────────────────────────────────────────────────────────┐
│ rhwp-studio (브라우저)                                                  │
│  변경 없음 — 본 작업은 *읽기 path* 만 다룸.                              │
│  사용자 키 입력·LLM 명령·파일 열기 흐름은 Sub-1/2 그대로.                 │
└───────────────────────────────────────────────────────────────────────┘
```

## 커밋 이력 (Sub-3 범위, 22 commit)

브랜치 `local/task_m200_zephy_bridge` (Sub-1·2 와 같은 브랜치). 단계 순서 그대로:

```
Phase 1 — 모듈 + 타입
  8636e3a5  ir_compact 모듈 scaffolding
  0dff129a  글자·문단·셀 서식 타입 + Serialize 키 검증
  96439ff2  IrRun·IrParagraph·IrSlice·DocDefaults 타입 정의
  2fdc02a3  CellBorder::all 의도 주석 추가 (code review I-1)

Phase 2 — 값 변환 5 종
  79154d1c  보조 helper — color/alignment/vertical-align 변환
  6883014b  char_shape_to_run_style + 변환 검증
  79ab3163  para_shape_to_para_style + percent-only line-height
  1397de7b  cell_to_cell_style + bgcolor/border 변환
  61643de5  [accessor] DocumentCore::styles() pub accessor 추가 (본체)

Phase 3 — 텍스트 path
  c5bac16f  collect_runs — 인접 동일 스타일 run 병합
  ae10c89d  build_text_paragraph + run 분할 검증
  ecb949c2  build_ir_slice 진입점 + 텍스트 path 동작 검증

Phase 4 — 표·셀 처리
  e47347be  build_cell_paragraph + cell_locator 채움
  ad7c925e  build_table_paragraph + try_build_cell + 2x2 표 검증
  49b42b37  build_paragraph 분기 + 셀 평탄 entry

Phase 5 — compact 압축
  e7f6c5fc  compute_doc_defaults + mode() tie-break
  b0a01bc6  omit_run_style_defaults + omit_para_style_defaults
  f5bcfaa9  compact_run + compact_text — 단일 run text 직속
  b234d62b  compact_ir_slice 진입 + 압축 4 규칙 통합

Phase 6 — endpoint + e2e
  29d2ff03  ir_slice_handler compact 분기를 ir_compact 호출로 교체
  4cb75346  e2e sub3-ir-compact — compact 응답 6 검증

Phase 7 — 정리
  4e201004  [cleanup] ir_compact 의 allow(dead_code) 제거
```

## 구현 중 발견된 사항 + 정정

implementer 들이 본체 구조와 plan 의 가정을 1:1 비교하면서 발견한 *plan 정정 필요* 사항 (모두 합리적):

1. **`cell_to_cell_style` 시그니처 변경** — plan 은 `(&Cell)` 단독이었으나, `Cell` 자체에 `fill_color`/`border_*` 가 없고 `border_fill_id: u16` 만 있다는 본체 구조 정합으로 `(&Cell, Option<&ResolvedBorderStyle>)` 두 인자. spec reviewer 검증 통과.
2. **`ColorRef` 가 struct 가 아닌 `u32` (0x00BBGGRR)** — plan 의 `ColorRef { r, g, b }` 형태 가정을 정정. `let r = (c & 0xFF) as u8;` 형태로 본체 `helpers.rs::color_ref_to_css` 와 동일 결과.
3. **`shade_color` sentinel `0x00FFFFFF`** — `ResolvedCharStyle::default` 가 흰색을 *highlight 없음* 의 sentinel 로 사용. `Option<String>` 으로 key omit.
4. **`DocumentCore::styles()` accessor** — 본체에 *최소 read-only accessor 한 개* 추가가 불가피. `document()` 는 이미 `commands/document.rs:626` 에 존재해서 재정의 불필요. spec §9 (rhwp 본체 무변경) 의 *예외 한 건* — 본체 기존 invariant 무변경.
5. **`set_char_shape` workbench 액션 미존재** — Sub-2 의 12 액션 명단에 없음. e2e 에서 `replace_runs` 로 의미 동등 대체. *Sub-3 후속 sub 후보*.
6. **노트북 라우터의 `compact` 키 미지원** — Phase 7 에서 *6 줄 분기 추가* 로 해결.

## 시험 결과

```
cargo test ir_compact::tests::   41 passed / 0 failed
cargo build --lib (본체)         warning 0
cargo build (server)             warning 0

e2e (server 가동 후):
  sub3-ir-compact.test.mjs              PASS (6 검증)
  ws-bridge.test.mjs (Sub-1 회귀)        PASS
  sub2-replace-runs.test.mjs            PASS
  sub2-canvas-insert-text.test.mjs      PASS
  sub2-audit-diff-ir-slice.test.mjs     PASS
  sub2-partial-update.test.mjs          PASS
  sub2-set-paragraph-style.test.mjs     PASS
```

`#![allow(dead_code)]` 도 *Phase 7 정리* 에서 제거 — 모든 helper 가 endpoint 의 transitive caller 라 dead 가 없음.

## 응답 형식 예 (compact)

`GET /sessions/<id>/ir-slice?mode=compact` (또는 `?mode=auto` / 기본):

```json
{
  "doc_meta": {
    "edit_session_id": "cli_<file_id>",
    "page": 1,
    "total_pages": 1,
    "anchor": {"sec": 0, "para_start": 0, "para_end": 3}
  },
  "paragraphs": [
    { "id": "p_0_0", "sec": 0, "para": 0, "type": "text",
      "runs": [ { "char_offset": 0, "text": "굵은 A", "style": {"bold": true} } ] },
    { "id": "p_0_1", "sec": 0, "para": 1, "type": "text", "text": "" },
    { "id": "p_0_2", "sec": 0, "para": 2, "type": "table",
      "rows": 2, "cols": 2,
      "cells": [
        { "row": 0, "col": 0, "paragraphs": [
            { "id": "p_0_2_c0_0_0", "sec": 0, "para": -1, "type": "text",
              "cell_locator": {"table_para": 2, "row": 0, "col": 0, "cell_para": 0},
              "text": "CELL_TEXT" }
        ]}
      ] },
    { "id": "p_0_2_c0_0_0", "sec": 0, "para": -1, "type": "text",
      "cell_locator": {"table_para": 2, "row": 0, "col": 0, "cell_para": 0},
      "text": "CELL_TEXT" }
  ],
  "defaults": {
    "run": {
      "bold": false, "italic": false, "underline": false, "strikethrough": false,
      "color": "#000000", "char-spacing": 0, "char-width": 100,
      "vertical-align": "baseline", "font-size": 10.0, "font-name": "맑은 고딕"
    },
    "paragraph": { "align": "left", "indent": 0, "line-height": 160 }
  },
  "section": 0, "para_start": 0, "para_end": 3, "mode": "compact"
}
```

모델 입장에서:
- *bold A* 가 `runs[0].style.bold == true` 로 직접 보임 (defaults 와 다른 키만 명시)
- *표* 가 `rows`/`cols`/`cells[]` 로 분명히 노출 — 셀 (0,0) 에 'CELL_TEXT'
- *셀 안 문단* 이 `paragraphs[]` 평탄에 다시 한 번 등장 — `cell_locator` 4 좌표 (table_para/row/col/cell_para) 로 *셀 내부 명령의 직접 인자* 가 됨

## 시연 안내 (사용자 검증 단계)

1. *서버 가동*:
   ```bash
   cd /Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp
   ./rhwp-studio/e2e/sub2-server.sh restart
   ```

2. *브라우저 (시크릿 탭)*: 노트북 cell 1 출력의 URL `http://127.0.0.1:7710/?fileId=sim-<ts>` 진입.

3. *파일 열기* — 표가 있는 hwp/hwpx 파일 로드 (samples/ 에 적절한 sample 이 없으면 직접 만들거나 한컴편집기로 작성).

4. *노트북 (cell 1-5 정의 + cell 6 의 sub_agent_run 호출)* — 예:
   ```python
   await sub_agent_run(
       "표의 (0,0) 셀에 '제목' 텍스트를 굵게 입력해줘.",
       file_id=SESSION_FILE_ID,
   )
   ```

5. *모델 출력 확인*:
   - `get-ir-slice` 응답에 `type:"table"` + `rows`·`cols` + `cell_locator` 가 보여야 함
   - 모델이 *정확한 cell 좌표* 로 `replace-cell-runs` 호출
   - 브라우저에 *셀 텍스트* 즉시 등장

6. *시연 결과* — 통과 또는 실패 보고. 실패 시 모델이 어디서 막혔는지 (`get-ir-slice` 응답 형식 / cell 좌표 박기 / 적용 결과) 알려주면 후속 fix 진행.

## Sub-3 의 후속 sub 로 미루는 항목

본 작업이 *읽기 path* 하나에 집중. 향후 sub 로 미루는 4건:

1. **Control Serialize derive 추가** — raw 모드에서도 표·그림·도형 컨트롤이 직렬화되도록. 본 작업은 *compact 응답에서 표를 노출* 하므로 모델 입장 문제는 해결됐지만, *raw 모드 디버깅 사용자* 입장에서는 여전히 controls 가 빠짐.
2. **`set_char_shape` workbench 액션 신설** — Sub-2 의 12 액션에 누락. e2e 가 `replace_runs` 로 우회 — bold/color/font-size 만 변경하고 *텍스트 유지* 하려는 모델 의도에는 더 부적합.
3. **`doc_meta.total_pages` 정확화** — 현재 ts 와 동일하게 `1`/`1` 하드코딩. paginator 결과를 받아오는 별도 작업.
4. **WS broadcast 의 IR delta 전파** — 본 작업은 *full IR slice 응답*. 향후 — 편집마다 *변경된 paragraph 만 IR delta 로* WS 로 흘리는 방식. 대용량 문서 모델 응답 토큰 절감.

## 결론

*읽기 path* 가 옛 rhwp 원본의 ir-builder.ts 알고리즘 그대로 서버 측 Rust 로 옮겨졌다. 모델은 이제 `get-ir-slice` 한 번에 *type/runs/cell_locator/defaults* 가 모두 평탄화된 형식의 IR JSON 을 받아 *표·셀 좌표를 정확히 박아* 편집 명령을 발행할 수 있다.

- 자동 검증: 41 unit test + sub3-ir-compact e2e + Sub-1/2 의 6 회귀 e2e 모두 통과
- 본체 회귀 0 (단 1줄 accessor 추가)
- spec / plan 의 13 절 / 7 phase 의 모든 약속 충실 — DoD 7 조건 중 6 조건 자동 검증 통과, 1 조건 (수동 시연) 만 사용자 영역

수동 시연 결과 보고 후 *남은 한계 항목* (`set_char_shape` 등) 의 우선순위를 결정하면 Sub-3 의 후속 sub 로 진행 가능.
