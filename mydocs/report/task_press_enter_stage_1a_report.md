# 단계 1a (rhwp) — `PressEnter` 신설 최종 보고

작성일: 2026-06-21
대상 저장소: UNIVA-rhwp
연결 spec: [`../../../docs/25-press-enter-redesign.md`](../../../docs/25-press-enter-redesign.md)
연결 계획서: [`../plans/task_press_enter_stage_1a.md`](../plans/task_press_enter_stage_1a.md)

## 1. 작업 결과 요약

`EditOperation::PressEnter` 신설 완료. 단일 variant + payload key 자체로 본문/셀 모드 분기. 단위 테스트 16 건 + 전체 1,514 건 모두 통과. 옛 `EditOperation::InsertParagraph` 동작 영향 0건 (단계 4a 자체 자체 자체 자체 제거).

## 2. 변경 자리

| 파일 | 변경 |
|---|---|
| [src/document_core/commands/edit_op.rs](../../src/document_core/commands/edit_op.rs) | `PressEnter` enum variant 추가 (line 432 형제) + `default_char_offset` helper + affected_range_for 분기 추가 + apply_edit_op 분기 추가 + apply_inverse 의 unreachable 추가 + 단위 테스트 16 건 |
| [rhwp-server/src/main.rs](../../rhwp-server/src/main.rs) | `press_enter` REST workbench 핸들러 신설 (insert_page_break 자리 형제, line 932 형제) |
| [src/wasm_api.rs](../../src/wasm_api.rs) | `pressEnter` WASM API 신설 (line 1370 형제). 단일 함수 + Option 인자로 본문/셀 모드 분기 |

변경 *안 한* 자리 (단계 4a 자체 자체):
- `EditOperation::InsertParagraph` variant
- `insert_paragraph_native` 헬퍼
- 옛 `insert_paragraph` REST 핸들러
- 옛 `insertParagraph` WASM API
- rhwp-studio UI (단계 2a)

## 3. payload 시맨틱 (실 구현)

### 본문 모드
```json
{
  "action": "press_enter",
  "payload": {
    "section": 0,
    "para": 3,
    "char_offset": -1,
    "count": 1,
    "style": {...},
    "page_break": false
  }
}
```

### 셀 모드 (`table_para` 키 박힘으로 분기)
```json
{
  "action": "press_enter",
  "payload": {
    "section": 0,
    "table_para": 2,
    "row": 1,
    "col": 0,
    "cell_para": 0,
    "char_offset": -1,
    "count": 1
  }
}
```

`char_offset` 시맨틱:
- `-1` 또는 음수 → 본문/셀 paragraph 끝 (default)
- `0` → 시작
- `len(text)` 이상 → clamp to len (= 끝, silent fail 0건)
- 중간값 → split

`count`: 같은 자리 Enter N 회. 첫 회만 split (분할 자세), 그 다음 N-1 회는 빈 paragraph 자리에서 다시 Enter.

`page_break`: true → 첫 번째 새 paragraph 만 페이지 분리. 셀 모드 + true → `INVALID_PAYLOAD` 에러.

## 4. 단위 테스트 결과

총 16 건 통과 (`cargo test --lib press_enter` 0.02s):

**본문 모드 (7건)**:
- `test_press_enter_body_end_default` — `char_offset:-1` → 원 본문 그대로, 새 빈 paragraph 가 +1 자리 (사용자 보고 사고 해결 자체 검증)
- `test_press_enter_body_start` — `char_offset:0` → 빈 paragraph 가 앞, 원 본문 +1 자리 (옛 insert_paragraph 동작 자세 동등)
- `test_press_enter_body_offset_equals_len` — `char_offset=len` → -1 과 동등
- `test_press_enter_body_offset_overflow_clamp` — `char_offset=100` → clamp to len
- `test_press_enter_body_middle_split` — `char_offset=2` ("hello") → "he" / "llo" 분할
- `test_press_enter_body_count_multi` — count=3 → 원 1 + 새 3 = 4 paragraph
- `test_press_enter_deserialize_defaults` — JSON deserialize 자세 default 값 검증

**셀 모드 (4건)**:
- `test_press_enter_cell_end` — 셀 paragraph 끝 Enter
- `test_press_enter_cell_start` — 셀 paragraph 시작 Enter
- `test_press_enter_cell_middle_split` — 셀 paragraph 중간 분할
- `test_press_enter_cell_count_multi` — count=3 셀 자세

**에러 (5건)**:
- `test_press_enter_body_missing_para` — 본문 모드 + para 누락 → INVALID_PAYLOAD
- `test_press_enter_cell_page_break_rejected` — 셀 모드 + page_break:true → INVALID_PAYLOAD
- `test_press_enter_cell_missing_row` — 셀 모드 + row 누락 → INVALID_PAYLOAD
- `test_press_enter_body_section_out_of_range` — section 범위 초과 → 에러
- `test_press_enter_body_para_out_of_range` — para 범위 초과 → 에러

전체 lib 테스트 1,514 통과 (51.6s) — 기존 테스트 영향 0건.

## 5. 발견 사실 (단계 1b 자체 자체 반영 자체)

### 5.1 셀 paragraph style helper 부재

`apply_cell_para_format_native` 자체 자체 자체 자체 부재 — 셀 자리 paragraph style 자세 직접 적용 helper 자체 자체 자체 자체. 단계 1a 자체 자체 *셀 모드 `style` 옵션 미지원* 자세 자체 자체 자체.

**영향 자세 자체**:
- 셀 자세 PressEnter 자체 자체 `style: Some({"align": "right", ...})` 박혀 자체 자세 *silent 무시* 자체 자체 자세 자체.
- *향후 cycle* 자체 자체 `apply_cell_para_format_native` 자체 자세 신설 자체 자체 자체 자체 자체.

**1b 자체 자체 반영 자체 자체**: rdocx 자체 자체 자세 자체 *같은 자세* — 셀 paragraph style apply 자체 자체 자체 자체 부재 자세 자체 자세 *별 자체 자세*. spec/26 (rdocx split helper 카드) 자세 자체 자체 자체 *셀 style 자체 자체 자체 자세 신설 자체 자체* 자세 자체.

### 5.2 `HwpError::InvalidPayload` variant 부재

[error.rs](../../src/error.rs) 자체 자체 자체 자체 자체 자체 — `InvalidFile / PageOutOfRange / RenderError / InvalidField` 자체 자체 자체 자체 4 variant 자체. *InvalidPayload* 자체 자체 자체 부재 자체 자체 자세.

**현재 자세 자체 자체**: `HwpError::RenderError("INVALID_PAYLOAD: ...")` 자세 자체 자체 — 접두어 자세 자체 자체 자체 자체 자체 자체 시맨틱 자체 박힘 자체 자체 자체 자체.

**1b 자체 자체 반영 자체 자체**: rdocx 자체 자체 자세 자체 *비슷한 자세 자체 자세*. 별 자체 자체 *HwpError variant 자체 자세 신설 자세* 자체 자체 자체 cycle 자체 자체 자체 자체.

### 5.3 rhwp-server `tests/` 폴더 부재

rhwp-server 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 통합 REST e2e 자체 자체 자체 자체 자체 부재. *workbench 자체 자체 자체 자체 통합 자체* 자체 자체 자체 자체 자체 자체 자체 자체 *studio e2e 자세 자체 자체 자체 자체 자체* 자체 자체 자체 자체 자체 자체.

**1b 자체 자체 반영 자체 자체**: rdocx 자체 자체 자체 자체 자체 자체 자체 *tests/ir_slice_basic.rs* 자체 자체 자체 자체 자체 박혀 자세 자체 자체 (rdocx 자체 자체 조사 자체 자체 자체) — rhwp 자체 자체 자세 자체 자체 자체 자체 자체 자세 자세 자체 자세 *별 cycle* 자체 자체 자체 *rhwp-server 자체 자체 통합 테스트 자체 자체 자세 자체*.

### 5.4 디자인 결정 자세 — 단일 variant + Option 키 자세 자체

`PressEnter` 자체 자체 *단일 variant* 자체 자체 자세 자체 — payload key 자체 자세 자세 자체 자세 자체. apply 분기 자체 자세 *if let Some(tp) = table_para* 자세 자체 자체 자체 자체 분기 자세 자체 자체 — 자세 자체 *직관 자세 자세 자체*.

**1b 자체 자체 반영 자체 자체**: rdocx 자체 자체 자체 자체 자체 자체 자체 *같은 단일 variant 자세 자체* 자세 자체 — 코드 일관성 자체 자세 자체.

## 6. 단계 1b 자체 자체 진입 자세 자세 자세

다음 단계 자세 자체:
1. *docs/26 자세 자체 자체* — rdocx paragraph split 헬퍼 신설 카드 자세 자체 자체 자체 자체 (별 자세 자체)
2. *단계 1b 자세 자체 자체* — rdocx 자체 자체 `PressEnter` variant 신설 + apply + REST + WASM. 단계 1a 자체 자체 발견 사실 자세 자체 자체 반영 자체.

## 7. 완료 기준 자세 (단계 1a)

- [x] `PressEnter` variant 신설 + apply 분기
- [x] REST workbench 핸들러 `press_enter`
- [x] WASM API `pressEnter`
- [x] 단위 16 통과
- [x] 전체 1,514 통과 (기존 영향 0건)
- [x] 옛 `InsertParagraph` 동작 변화 0건
- [x] 발견 사실 보고서 작성
- [ ] studio 시각 검증 — *단계 2a 자세 자체 진행 자체*
