# Sub-7 v2 — 셀 내부 run.style 적용 실패 + replace_runs partial merge 동작 정리

## 배경

Sub-7 본편이 *본문 replace_runs / set_cell_style / set_paragraph_style* 에서 광고 키 정합 + deny_unknown_fields + PatchSummary.noChangeWarning 까지 완성. 그러나 로컬 직접 검증에서 두 새 사고 발견:

### 사고 A — 셀 내부 char_format 완전 미적용

`replace_cell_runs` 또는 `insert_text_in_cell` 로 *셀 내부 run* 에 `{"color":"#FF0000","bold":true}` 같은 style 보내면:

```
보냄:  {"runs":[{"text":"굵게","style":{"bold":true}}]}
받음:  [{"style": {"highlight": "#FFFFFF"}, "text": "굵게"}]   ← bold 흔적 0
```

`PatchDiff.diff.after.cell` 와 별도 `GET /ir-slice` 모두 *style 없음 + 모든 runs 가 1개로 merge*. 본문 `replace_runs` 는 정상 — 즉 *셀 내부 경로만 무효*.

### 사고 B — 본문 replace_runs 의 style 상속

본문 `replace_runs` 에서:

```
보냄: [
  {"text":"빨강","style":{"color":"#FF0000"}},
  {"text":"노랑","style":{"highlight":"#FFFF00"}},
  {"text":"파랑","style":{"color":"#0000FF","bold":true}}
]
받음: [
  {"style":{"color":"#FF0000","highlight":"#FFFFFF"},"text":"빨강"},
  {"style":{"color":"#FF0000","highlight":"#FFFF00"},"text":"노랑"},   ← color #FF0000 이전 run 에서 상속
  {"style":{"bold":true,"color":"#0000FF","highlight":"#FFFF00"},"text":"파랑"}  ← highlight 상속
]
```

각 run 의 미지정 키가 *직전 run 의 char_shape 를 base 로 상속* 됨. 의도된 partial merge 의미인지 (PartialRunStyle 의 None 은 *현재 값 유지*) 또는 의도 외 누수인지 확인 필요.

## 가설 — 사고 A 메커니즘

### 가설 A1 — apply_char_format_in_cell_native 의 char_shape 변경이 IR 에 반영 안 됨

[formatting.rs:959](../../src/document_core/commands/formatting.rs#L959) apply_char_format_in_cell_native 의 흐름:
1. `parse_char_shape_mods(props_json)` → `CharShapeMods { bold: Some(true), ... }`
2. `get_cell_paragraph_ref` → 셀 문단 immutable ref
3. `base_id = para.char_shape_id_at(start_offset).unwrap_or(0)`
4. `new_id = self.document.find_or_create_char_shape(base_id, &mods)`
5. `get_cell_paragraph_mut` → 셀 문단 mutable ref
6. `cell_para.apply_char_shape_range(start_offset, end_offset, new_id)`
7. `rebuild_section` + event_log

가설 — 6번 `apply_char_shape_range` 의 *char_shape_ids 배열 인덱스 기준* 이 본문과 다를 가능성. 셀 내 paragraph 의 `char_shape_ids` 가 *상대 인덱스 (셀 내부 0-base)* 인지 *문서 전역 인덱스* 인지 확인 필요.

### 가설 A2 — extract_compact_cell 이 char_shape_id 무시

[server/src/ir_compact.rs](../../server/src/ir_compact.rs) 의 `extract_compact_cell` / `cell_paragraph_to_compact` — 셀 내 paragraph 의 run.style 추출 시 *char_shape_id 가 아닌 셀 defaults 만* 보는 것일 가능성. 본문은 paragraph_to_compact 가 char_shape_id 마다 분리 + style 추출, 셀은 다른 path 일 수도.

### 가설 A3 — insert_text_in_cell_native 가 *항상 default char_shape* 로 텍스트 삽입

replace_cell_runs_native 의 흐름 (text_editing.rs:809):
1. 기존 텍스트 삭제 (`delete_text_in_cell_native`)
2. 각 run:
   a. `insert_text_in_cell_native` — 텍스트 삽입
   b. `apply_char_format_in_cell_native` — style 적용

가설 — 2a 가 *셀의 default char_shape* 로 텍스트를 넣고, 2b 의 char_shape 변경이 *그 텍스트 범위에 새 ID 적용* 까지는 하지만, *cell paragraph 의 char_shape_ids 배열 구조* 가 잘못 갱신되어 IR 추출 시 default 만 보이는 것일 수 있음.

## 가설 — 사고 B 메커니즘

PartialRunStyle 의 *None = 현재 값 유지* 의미가 *replace_runs 내부에서도 직전 run 의 style 을 base 로 함* 으로 구현된 것일 가능성. 즉:

- run[0] 의 style = base (paragraph default) + {color: #FF0000}
- run[1] 의 style = run[0] 의 결과 + {highlight: #FFFF00} ← *color 가 #FF0000 그대로 상속*
- run[2] 의 style = run[1] 의 결과 + {color: #0000FF, bold: true} ← *highlight 가 #FFFF00 그대로 상속*

본문 replace_runs_native ([text_editing.rs:756](../../src/document_core/commands/text_editing.rs#L756)) 가 각 run 의 apply_char_format_native 호출 시 *직전 cursor 위치의 char_shape* 를 base 로 새 ID 만들기 때문일 수 있음. 본문은 `find_or_create_char_shape(base_id, &mods)` 가 *기존 base 의 모든 필드 + mods 의 일부만 덮어쓰기*.

의도 판단:
- *의도된* 동작: 사용자가 한 run 에 color 만 지정하면 *직전 run 의 bold/italic/underline 등 다른 속성은 유지* — partial merge 일관성
- *의도 외* 동작: 각 run 이 *paragraph default 에서 시작* + 명시한 키만 적용 — replace_runs 의 "통째 교체" 의미 = 매번 깨끗한 default 에서 시작

코드 + 기존 e2e + SKILL.md 의미 확인 후 결정.

## 목표

1. *사고 A 진단 + fix* — `replace_cell_runs` / `insert_text_in_cell` 의 run.style 이 *본문 replace_runs 와 동일하게* IR 응답에 정확히 반영되어야 함.
2. *사고 B 의도 확인* — partial merge 가 의도면 SKILL.md 명시 + 단위 테스트로 잠금. 의도 아니면 fix.
3. 신규 e2e 시나리오 추가 — 셀 내 multi-run style 분리 검증.

## 비목표

- 본문 replace_runs 의 fix (이미 정상)
- SetCellStyle / SetParagraphStyle 의 fix (이미 정상)
- 광고 카탈로그 확장

## 단계

### Phase 0 — 진단 (Explore)

1. apply_char_format_in_cell_native 가 호출됐는지 — 디버그 로그 또는 단위 테스트로 *셀 paragraph 의 char_shape_ids 가 변경되는지* 확인
2. find_or_create_char_shape 가 *partial mods 만으로 새 ID 를 생성하는지* — 빈 mods 시 base_id 그대로 반환하는지 확인
3. extract_compact_cell / cell_paragraph_to_compact 가 *run 의 char_shape_id 를 어떻게 추출하는지* — paragraph_to_compact 와 비교
4. apply_char_shape_range 가 *셀 paragraph 의 char_shape_ids* 에 정확히 쓰는지

진단 결과를 *plan 본문에 Phase 0 결과로 갱신*.

#### 진단 결과 (2026-06-08)

단위 테스트 `diag_sub7v2_cell_char_shape_after_apply` 로 셀 paragraph 상태 직접 확인:

```
[diag] after insert: text="원본" char_offsets=[0, 1] char_shapes=[]
[diag] after apply: text="원본" char_offsets=[0, 1] char_shapes=[]
[diag] doc char_shapes count=8   ← find_or_create_char_shape 은 정상 새 ID 생성
```

| 가설 | 결과 | 증거 |
|------|------|------|
| A1 (apply_char_shape_range 좌표 오류) | **부분 입증** | `apply_char_shape_range` ([paragraph.rs:992-1050](../../src/model/paragraph.rs#L992-L1050)) 가 `for csr in self.char_shapes.iter()` 루프로 새 ref 를 만드는데, 빈 char_shapes 위에서는 루프 0회 → new_refs 빈 채 종료 |
| A2 (extract_compact_cell 무시) | 기각 | `build_cell_paragraph` ([ir_compact.rs:466-543](../../server/src/ir_compact.rs#L466-L543)) 가 본문과 동일한 `char_shape_id_at` + `char_shape_to_run_style` 사용. 호출 자체는 정상, 단지 `char_shapes=[]` 위에서 항상 0/default 만 반환 |
| A3 (insert + apply 범위 문제) | 기각 | insert_text_at 자체는 정상 텍스트/offset 갱신. 단, char_shapes 가 비어있으면 새로 push 하지 않음 — *셀 paragraph 의 초기 char_shapes 가 빈 상태로 시작* 하는 게 근본 원인 |

**근본 원인**: `Cell::new_empty` → `Paragraph::new_empty()` 가 char_shapes 를 빈 Vec 으로 초기화. `create_table_native` ([object_ops.rs:869-890](../../src/document_core/commands/object_ops.rs#L869-L890)) 는 `raw_header_extra[0..2] = 1u16` 로 *n_char_shapes=1* 만 헤더에 기록할 뿐, 실제 `cp.char_shapes` 벡터에는 CharShapeRef 를 push 하지 않는다. 결과:
- 셀 paragraph 의 `char_shapes` 가 영원히 빈 채 유지
- `apply_char_shape_range` 호출해도 빈 루프로 no-op
- `char_shape_id_at` 가 None 반환 → IR 응답에 모든 run 이 default style (`highlight:#FFFFFF` 만)

본문은 `create_blank_document_native` 가 별도 normalize 경로로 `[CharShapeRef{start_pos:0, char_shape_id:0}]` 1건을 보장하므로 정상.

**사고 B (replace_runs cascade) 의도 결정**:

- 현재 동작: `apply_char_mods_to_paragraph` ([formatting.rs:1507-1524](../../src/document_core/commands/formatting.rs#L1507-L1524)) 가 `char_shape_id_at(start_offset)` 를 base 로 사용. replace_runs 루프 내에서 두 번째 이후 run 의 시작 위치는 *직전 run 이 방금 채운 char_shape* 가 적용된 자리 → 직전 run 의 style 이 base 가 됨
- PartialRunStyle 의도: `edit_op.rs:140` doc comment "None 인 필드 유지" — patch 의미. 하지만 *어떤 base 에 대한 patch 인지*는 명시 없음
- replace_runs 의 "통째 교체" 명칭, 사용자 사고 시나리오 (각 run 을 독립으로 봄), 그리고 *셀 fix 후 통일성* 을 고려해: **각 run 은 paragraph 의 기존 default char_shape 를 base 로 시작** 으로 결정. 이 결정의 영향:
  - 첫 번째 run 은 종전과 동일 (어차피 char_shape_id_at(0) 이 paragraph default)
  - 두 번째 이후 run 은 직전 run 의 style 을 *상속하지 않고* paragraph default 에서 출발
  - 의도된 cascade 가 필요한 호출자는 *각 run 에 모든 style 키 explicit 지정* 으로 표현 가능 — 의미가 더 명확

### Phase 1 — 사고 A fix

진단에 따라 *하나 이상* 의 fix:

- (가설 A1 면) `apply_char_shape_range` 호출 좌표 정정 또는 char_shape_ids 배열 구조 수정
- (가설 A2 면) `extract_compact_cell` 에서 cell paragraph 의 run 추출 경로를 본문과 동일 코드로 통합
- (가설 A3 면) `insert_text_in_cell_native` + `apply_char_format_in_cell_native` 의 순서·범위 보정

#### Phase 1 적용 결과 (2026-06-08)

A1 입증에 따라 *두 군데* 수정:

1. **`create_table_native`** ([object_ops.rs:891-901](../../src/document_core/commands/object_ops.rs#L891-L901)) — 셀 paragraph 의 `char_shapes` 가 빈 Vec 이면 `CharShapeRef { start_pos: 0, char_shape_id: default_char_shape_id }` 를 푸시. `raw_header_extra` 의 *n_char_shapes=1* 헤더 기록만 있던 옛 동작과 짝을 맞춰 실제 벡터에도 baseline ref 가 1개 존재하도록 한다.
2. **`Paragraph::apply_char_shape_range`** ([paragraph.rs:954-963](../../src/model/paragraph.rs#L954-L963)) — 빈 `char_shapes` 방어. 외부 호출 진입 시점에 비어 있으면 `{start_pos:0, char_shape_id:0}` 한 건을 박아두고 본 루프 진행. (1) 의 fix 이전에 생성된 옛 데이터나, 다른 경로로 빈 상태에 진입한 paragraph 도 정상 동작.

검증: 단위 4 건 (`test_sub7v2_cell_paragraph_has_initial_char_shape`, `..._replace_cell_runs_with_distinct_styles`, `..._insert_text_in_cell_with_style`, `..._replace_runs_no_style_cascade`) + e2e 5 시나리오 모두 PASS.

### Phase 2 — 사고 B 의도 결정 + 대응

본문 `replace_runs_native` 가 partial merge 를 *의도* 한다면:
- SKILL.md / edit-phase.md 에 "각 run 의 미지정 style 키는 직전 run 의 값을 유지" 명시
- 단위 테스트로 *현재 동작 잠금*
- 깨끗한 default 에서 시작하려면 *명시적으로 모든 키 지정* 안내

의도 *아니면*:
- 각 run 마다 *paragraph default* 를 base 로 새 char_shape 생성
- 단위 테스트로 *새 동작 잠금*

판단 기준:
- 기존 e2e 가 어느 쪽으로 작동하길 기대하는지
- SKILL.md / README 의 PartialRunStyle 설명 (currently "None 인 필드는 현재 값 유지")
- 사용자 입장에서 자연스러운 동작 — *통째 교체* 라는 명칭은 매번 default 시작이 자연스러움

#### Phase 2 결정 (2026-06-08)

**결정**: 각 run 은 paragraph 의 *원래 default char_shape* 를 base 로 시작. cascade 제거.

**근거**:
- replace 의 명칭상 "통째 교체" 의미와 정합 — 각 run 은 독립
- 기존 e2e (sub2-replace-runs, sub7-style-round-trip 15 시나리오) 가 cascade 의존 없음 — 모두 통과 확인됨
- PartialRunStyle 의 "None = 현재 값 유지" 는 *patch base 가 무엇인지* 별도 결정 — base 를 paragraph default 로 잡으면 자연스럽고, run 간 상호작용 없음
- 의도된 cascade 가 필요한 호출자는 *각 run 에 모든 style 키를 explicit 지정* 으로 표현 가능 (의미가 더 명확)

**구현**:

1. `apply_char_format_native_with_base` ([formatting.rs:958-1032](../../src/document_core/commands/formatting.rs#L958-L1032)) — 새 helper. 호출자가 `base_char_shape_id` 를 직접 넘김. 본문용.
2. `apply_char_format_in_cell_native_with_base` ([formatting.rs:1124-1199](../../src/document_core/commands/formatting.rs#L1124-L1199)) — 셀 내 동등 helper.
3. `replace_runs_native` ([text_editing.rs:756-820](../../src/document_core/commands/text_editing.rs#L756-L820)) — 루프 진입 전 `base_char_shape_id` 를 paragraph 의 첫 char_shape 에서 capture. 각 run 의 style 적용에 (1) 사용.
4. `replace_cell_runs_native` ([text_editing.rs:822-915](../../src/document_core/commands/text_editing.rs#L822-L915)) — 동등. 셀 paragraph 의 첫 char_shape 를 base 로 capture, (2) 사용.

기존 `apply_char_format_native` / `apply_char_format_in_cell_native` 는 *그대로 유지* — 단일 호출 (예: `set_cell_style`) 에서는 의도된 누적 동작이 필요하므로 base 를 옛 방식 (`char_shape_id_at(start)`) 으로 둔다. 영향 범위는 *replace_runs / replace_cell_runs 내부 루프 한정*.

### Phase 3 — 단위 테스트 + e2e

- 셀 내 multi-run style 분리 시나리오 (replace_cell_runs + 3 run + 각 다른 color)
- 셀 내 insert_text_in_cell + style {color, bold} 적용 후 IR 검증
- 본문 replace_runs partial merge 동작 잠금 (의도 결정 따라)

#### Phase 3 — 추가된 테스트 (2026-06-08)

**단위 4 건** ([text_editing.rs:2786-2952](../../src/document_core/commands/text_editing.rs#L2786-L2952)):

- `test_sub7v2_cell_paragraph_has_initial_char_shape` — `create_table_native` 후 모든 셀 paragraph 의 `char_shapes` 비어있지 않음 잠금.
- `test_sub7v2_replace_cell_runs_with_distinct_styles` — 3 run (textColor / bold / shadeColor) 모두 다른 char_shape_id 로 분리되는지 잠금.
- `test_sub7v2_insert_text_in_cell_with_style` — insert + apply_char_format_in_cell_native 후 cell paragraph char_shapes 갱신 + bold 비트 확인.
- `test_sub7v2_replace_runs_no_style_cascade` — 본문 run[1] color 가 run[0] color 와 다른지 (cascade 회피).

**e2e 5 건** ([sub7v2-cell-style-round-trip.test.mjs](../../rhwp-studio/e2e/sub7v2-cell-style-round-trip.test.mjs)):

- A1: `replace_cell_runs + bold` → diff.after.cell.runs[0].style.bold === true
- A2: 3 run distinct (color/bold/highlight) → 응답·IR 모두 3개로 분리, 각자 정확 style
- A3: `insert_text_in_cell` + style → IR 응답에 color 노출
- B1: 본문 replace_runs 의 run[1] color 가 run[0] color 상속 안 함
- B2: 셀 replace_cell_runs 동등

### Phase 4 — SKILL.md / edit-phase.md 갱신

사고 B 의도 결정 결과 명시 (한 줄).

#### Phase 4 — 갱신 위치 (2026-06-08)

[edit-phase.md §2.1 replace-runs 직후 별표 문단](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/edit-phase.md) — 한 문단 추가:

> ★ *각 run 의 style 은 paragraph 의 원래 default 에서 출발*. run[1] 이 `color` 를 지정하지 않았다면 *run[0] 의 color 를 상속하지 않고* paragraph default (보통 검정) 가 적용된다. 매 run 이 다른 색을 가지려면 *각 run 에 color 를 명시적으로 지정*. 같은 규칙이 `replace-cell-runs` 에도 적용된다 — Sub-7 v2 (2026-06-08) 에서 cascade 동작이 의도 외로 식별되어 *깨끗한 default 시작* 으로 통일.

### Phase 5 — 보고서 + commit + push

`mydocs/report/task_m200_zephy_bridge_sub7_v2_report.md` + `feature/jerry-command-expansion` 에 push.

## 검증 체크리스트

- [ ] `cargo test` 회귀 0 + Sub-7 v2 신규 단위 통과
- [ ] `cargo build --release` ok
- [ ] `npm run build` ok
- [ ] 신규 e2e — 셀 내 multi-run style 분리 PASS
- [ ] Live curl 검증:
  - `replace_cell_runs + [{text, style:{color:#F00,bold:true}}]` → IR 의 run.style 에 color + bold 정확히 노출
  - `insert_text_in_cell + style:{color}` → IR 의 새 run 에 color 노출
- [ ] 사고 B 의도 결정 + SKILL.md 명시
- [ ] sub7-style-round-trip + 기존 18 e2e 회귀 0

## 리스크

- *char_shape_ids 배열 구조 변경* — IR 추출 외 SVG 렌더링·HWPX export 까지 영향. 회귀 광범위.
- *partial merge 동작 변경 (사고 B)* — 기존 e2e 가 *현재 동작에 의존* 한다면 깨질 수 있음. Phase 0 에서 e2e 영향 분석.

## 다음

승인 즉시 sub-agent dispatch (Phase 0 진단 → Phase 1-5 일괄).
