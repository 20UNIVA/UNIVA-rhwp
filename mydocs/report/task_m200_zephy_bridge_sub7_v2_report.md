# Sub-7 v2 — 셀 내부 run.style 적용 실패 + replace_runs cascade 동작 정리

## 배경

Sub-7 본편 push 후 로컬 직접 검증에서 두 새 사고 발견:

### 사고 A — 셀 내부 char_format 완전 미적용

`replace_cell_runs` / `insert_text_in_cell` 에 run.style 보내면 *모든 style 손실 + runs 1개로 merge*. 본문 `replace_runs` 는 정상 — 셀 내부 경로만 무효.

```
보냄:  {"runs":[{"text":"굵게","style":{"bold":true}}]}
받음:  [{"style": {"highlight": "#FFFFFF"}, "text": "굵게"}]   ← bold 흔적 0
```

### 사고 B — 본문 replace_runs 의 style cascade

각 run 의 미지정 키가 *직전 run 의 char_shape 를 상속*:
- "노랑" 에 color #FF0000 따라옴 (직전 "빨강" 에서)
- "굵게" 에 highlight #FFFF00 따라옴 (직전 "노랑" 에서)

## 진단 (Phase 0)

단위 테스트 `diag_sub7v2_cell_char_shape_after_apply` 로 셀 paragraph 상태 직접 확인:

```
[diag] after insert: text="원본" char_offsets=[0, 1] char_shapes=[]
[diag] after apply: text="원본" char_offsets=[0, 1] char_shapes=[]
[diag] doc char_shapes count=8   ← find_or_create_char_shape 은 정상 새 ID 생성
```

| 가설 | 결과 | 증거 |
|---|---|---|
| A1 (apply_char_shape_range 좌표 오류) | **부분 입증** | [paragraph.rs:992-1050](../../src/model/paragraph.rs#L992) 의 `for csr in self.char_shapes.iter()` 루프가 빈 char_shapes 위에서 0회 실행 → 어떤 새 ref 도 추가되지 않는 no-op |
| A2 (extract_compact_cell 무시) | 기각 | [ir_compact.rs build_cell_paragraph](../../server/src/ir_compact.rs) 는 본문과 동일 코드. char_shape_id_at 가 빈 char_shapes 위에서 항상 None 반환 |
| A3 (insert + apply 범위 문제) | 기각 | insert_text_at 자체는 정상. char_shapes 가 비어있으면 새 ref 를 push 하지 않음 |

**근본 원인** — `Cell::new_empty` → `Paragraph::new_empty()` 가 char_shapes 를 빈 Vec 으로 초기화. `create_table_native` 는 `raw_header_extra[0..2] = 1u16` 로 *n_char_shapes=1* 헤더만 기록할 뿐, 실제 `cp.char_shapes` 벡터에는 CharShapeRef 를 push 하지 않음.

본문은 `normalize_hwpx_paragraphs` 같은 별도 normalize 경로로 baseline 1건이 보장 → 정상 동작.

**사고 B 의도 결정** — paragraph default 에서 시작 (cascade 제거). 근거:
- `replace` 의 "통째 교체" 명칭은 매번 깨끗한 default 시작이 자연스러움
- 사용자 사고 시나리오에서 각 run 을 독립으로 봄
- 셀 fix 후 통일성 유지
- 의도된 cascade 가 필요한 호출자는 *각 run 에 모든 style 키 explicit 지정* 으로 표현 가능 — 의미 더 명확

기존 e2e 24 시나리오 (sub2 + sub7) 모두 무영향 확인.

## 변경

### 1. src/document_core/commands/object_ops.rs — create_table_native baseline ref

[object_ops.rs:891-901](../../src/document_core/commands/object_ops.rs#L891)

```rust
// [Sub-7 v2] 셀 paragraph 의 char_shapes 가 비면 baseline CharShapeRef 푸시.
// raw_header_extra 의 n_char_shapes=1 헤더와 짝을 맞춰 실제 벡터에도
// baseline 1건이 존재해야 apply_char_shape_range 가 동작.
if cp.char_shapes.is_empty() {
    cp.char_shapes.push(CharShapeRef {
        start_pos: 0,
        char_shape_id: default_char_shape_id,
    });
}
```

### 2. src/model/paragraph.rs — apply_char_shape_range defensive guard

[paragraph.rs:992~](../../src/model/paragraph.rs#L992)

빈 char_shapes 진입 시 baseline `{start_pos:0, char_shape_id:0}` 박는 defensive guard 추가. 정상 경로 (Phase 1.1 fix) 가 항상 baseline 푸시하므로 이 guard 는 *방어선* 으로만 동작.

### 3. src/document_core/commands/formatting.rs — base 명시 helper 신규

기존 `apply_char_format_native` / `apply_char_format_in_cell_native` 는 `char_shape_id_at(start)` 를 base 로 사용 (단일 호출 시 누적 적용 의미 보존). 신규 helper:

```rust
pub fn apply_char_format_native_with_base(
    &mut self, sec, para, start, end, props_json, base_char_shape_id: u32,
) -> Result<String, HwpError>;

pub fn apply_char_format_in_cell_native_with_base(
    &mut self, sec, table_para, ctrl, cell, cell_para, start, end, props_json,
    base_char_shape_id: u32,
) -> Result<String, HwpError>;
```

호출자가 *base 를 명시 지정* 할 수 있어 replace_runs 의 cascade 제거에 사용.

### 4. src/document_core/commands/text_editing.rs — replace_runs cascade 제거

`replace_runs_native` / `replace_cell_runs_native` 가 루프 진입 *전* base_char_shape_id 캡처 후 신규 helper 호출:

```rust
let base_char_shape_id = paragraph.char_shape_id_at(0).unwrap_or(0);
for run in runs {
    // ... insert text
    if has_style {
        self.apply_char_format_native_with_base(
            sec, para, cursor, cursor + len, &style_json, base_char_shape_id,
        )?;
    }
    cursor += len;
}
```

각 run 의 style 이 paragraph 의 *원래* default 에서 출발 → cascade 없음.

### 5. 신규 e2e — rhwp-studio/e2e/sub7v2-cell-style-round-trip.test.mjs

5 시나리오:
1. `replace_cell_runs` 단일 run + bold → IR 의 run.style.bold === true
2. `replace_cell_runs` 3 runs 다른 style → cascade 없이 각자 분리
3. `insert_text_in_cell` + style → 새 run + 기존 run 분리, 각자 style
4. `replace_cell_runs` 빈 runs → 셀 비움
5. `replace_cell_runs` 후 GET /ir-slice cross-check → diff 와 일치

### 6. SKILL.md 갱신

[edit-phase.md](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/edit-phase.md) §2.1 `replace-runs` 정의 직후:

> 각 run 의 style 은 paragraph 의 원래 default 에서 출발 — *직전 run 의 style 을 상속하지 않음*. 같은 규칙이 `replace-cell-runs` 에도 적용된다. (Sub-7 v2 (2026-06-08) 에서 cascade 동작이 의도 외로 식별되어 *깨끗한 default 시작* 으로 통일.)

## 검증

### 단위

- `cargo test --lib` rhwp: **1470 pass / 0 fail / 6 ignored** (Sub-7 v2 신규 4건 포함)
- `cargo test` rhwp-server: **78 pass / 0 fail**

### 빌드

- `cargo build --release` ok (rhwp + rhwp-server)
- `npm run build` (rhwp-studio) ok

### e2e

- 신규 `sub7v2-cell-style-round-trip` — **5/5 PASS**
- 회귀 — 19 시나리오 전수 PASS (sub2 5 + sub3 + sub4 + sub6 + sub7 + ws-bridge)

### Live curl 검증 (로컬 7710)

| 시나리오 | 이전 결과 | Sub-7 v2 결과 |
|---|---|---|
| `replace_cell_runs + bold` | bold 흔적 0 | **bold:true 정상 노출** |
| 3 runs 각자 color/highlight/bold | 모두 merge, style 손실 | 3 runs 분리, 각자 style, cascade 없음 |
| `insert_text_in_cell + color/bold` | runs merge, style 손실 | **새 run + 기존 run 분리, 각자 style 정확** |

## 효과

1. *셀 내부 char_format 적용* — replace_cell_runs / insert_text_in_cell 의 run.style 이 본문과 동일하게 IR 응답에 정확히 반영
2. *replace_runs 의 의미 명확화* — cascade 제거로 "통째 교체" 라는 명칭과 동작 일치, 의도된 cascade 는 explicit 지정으로 표현
3. *방어선 추가* — apply_char_shape_range 의 defensive guard 로 빈 char_shapes 진입 시도 자체를 막음

## 트레이드오프

- 기존 cascade 동작에 *의존* 한 호출자가 있다면 영향. 현 codebase 의 e2e 24건 + 단위 1470건 모두 무영향 확인.
- `apply_char_format_native` (단일 호출) 는 *기존 동작 그대로* 유지 — set_cell_style·set_paragraph_style 등 누적 적용 시나리오 보존. 만약 향후 단일 호출에서도 base 통일 필요해지면 별도 sub.
- `apply_char_shape_range` 의 defensive guard 가 `char_shape_id=0 이 base` 라는 가정에 의존. 정상 경로 (Phase 1.1 fix) 가 항상 baseline 푸시하므로 도달 안 함 — 정교화 필요 시 별도 sub.

## 다음

`feature/jerry-command-expansion` 에 push → VM 재배포 → 사용자가 셀 내 multi-run style 적용 시나리오 직접 확인.
