# Task #m600-28 최종 결과 보고서 — HWPX put_snapshot round-trip 자리 결함 자료 4건

## 사이클 요약

사용자 보고 — `1. (★사업중 필독) 사업관리 참조표 (1).hwp` 를 studio 에서 다시 열면 표 형태·여백·이중 표 자료가 깨짐. 진단 결과 — WASM client 가 HWPX 자료로 PUT /snapshot 보낸 후 HWP export 한 자료가 *원본 hwp 와 다른 형태* 로 박힘. 직렬화 path 안 결함 4건 식별·fix.

## 결함 메커니즘

1. 클라이언트 워크플로우 — server 에서 받은 HWPX 자료로 편집 + `PUT /sessions/:id/snapshot` 으로 HWPX 박음.
2. server 의 `parse_document` (HWPX parser) 가 IR 구성. *Table.raw_ctrl_data* 자료가 `Vec::new()` 자료. *PageDef* 자료는 `<hp:pagePr>` 자료로부터 정상 복원.
3. 사용자 export hwp → `serialize_document` → 직렬화 path 안 4 곳에서 자료 손실:
   - serialize_table 의 fallback 부재 → table common 손실
   - HWPX page_break 매핑 역방향 → RowBreak/CellBreak 자료 바뀜
   - cell paragraph controls 무시 → nested table·picture 손실
   - HWPX template pagePr/margin 고정값 → PageDef 손실

## 변경

### `src/serializer/control.rs` — Fix 1

`build_table_ctrl_data_from_common` 함수 신설. `serialize_table` 안에서 `table.raw_ctrl_data` 가 비어있을 때 common + `outer_margin_*` 자료로 ctrl_data 합성.

### `src/serializer/hwpx/table.rs:438-444` — Fix 2

`table_page_break_str` 매핑 뒤집음. HWPX parser 의 주석 정합:
- `RowBreak → "CELL"` (종전 `"TABLE"`)
- `CellBreak → "TABLE"` (종전 `"CELL"`)

### `src/serializer/hwpx/table.rs:278` — Fix 3

`write_sub_list` 안 cell paragraph 자료 박는 자리에 `controls` 순회 추가. `Control::Table` → `write_table` 재귀, `Control::Picture` → `write_picture` 호출.

### `src/serializer/hwpx/section.rs` — Fix 4

`replace_page_pr` 헬퍼 신설. `write_section` 안에서 EMPTY_SECTION_XML template 의 `<hp:pagePr>` 영역을 `section.section_def.page_def` 자료로 교체.

## 검증

### 코드 회귀

| 자리 | 결과 |
|---|---|
| `cargo test --workspace --lib` | 1491 passed / 0 failed |

### 자동 e2e — 사용자 워크플로우 round-trip

원본 hwp → server → 편집 op 4건 (insert_text + replace_cell_runs×2 + replace_runs) → HWPX export → put_snapshot → HWP export → IR 비교:

| 자리 | Fix 전 | Fix 후 |
|---|---|---|
| `tbl outer_margin` (5표) | (0,0,0,0) | 원본 값 (140·283·141) |
| `tbl size` | (0,0) | 원본 값 |
| `tbl wrap` | Square | TopAndBottom |
| `tbl tac` | false | 원본 값 |
| `tbl vert_rel/horz_rel` | Paper | Para/Column |
| `tbl page_break` | 서로 바뀜 | 정합 |
| nested 3x2 table | 사라짐 | 보존 |
| PageDef margin | L=8504 R=8504 | L=5669 R=5669 (원본) |
| ir-diff (편집 자료 round-trip) | 45건 | 12건 (모두 편집 차이) |

### 사용자 시각 검증

- 왼쪽 여백 회복 ✓ (Fix 1)
- 오른쪽 여백 회복 ✓ (Fix 4)
- 2페이지 표 정상 ✓ (Fix 2)
- 3페이지 이중 표 내부 표 회복 ✓ (Fix 3)
- 표 편집 후 위아래 늘어남 결함 해소 ✓
- 편집 op 적용 후 studio 무너짐 자료 없음 ✓

## 부산물 자료

ir-diff 의 비교 범위에 PageDef 자료가 포함되지 않음 — 자동 e2e 가 *45 → 1건* 으로 좁혀진 시점에서도 사용자 시각 보고가 없었으면 Fix 4 의 결함 자체를 catch 못 함. 차후 ir-diff 확장 권장 (별도 cycle).
