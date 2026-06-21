# Task #m600-30 최종 결과 보고서 — HWPX cell paragraph char_shapes 부분 스타일 보존

## 사이클 요약

cycle 28·29 종결 후 사용자 시각 보고 — *cell 안 paragraph 의 들여쓰기·굵게 자체 자체 새로고침 후 사라짐*. 원인 — HWPX serializer 의 `write_sub_list` 가 cell paragraph 의 *char_shapes 자체 자체 자체 자체 자체 단일 hp:run 박음*. 부분 char_shape (run-level styling) 손실.

## 결함 자리

[src/serializer/hwpx/table.rs:270](../../src/serializer/hwpx/table.rs#L270) `write_sub_list` — cell paragraph 자료 박는 자리가 `char_shapes.first()` 하나로 단일 `<hp:run>` 박고 텍스트 전체 묶음.

## 변경

### `src/serializer/hwpx/table.rs`

`write_sub_list` 안 hp:run 박는 자리 — char_shapes 별로 별도 `<hp:run charPrIDRef>` 박음. start_pos 자료 (utf16 offset) 따라 paragraph.text 분할. start_pos 자료가 paragraph end marker·control 자체 자체 utf16 위치 포함이라 `text_u16.len()` 자체 clamp. empty text 자체 자체 빈 hp:run 박아 char_shape_id 자료 보존.

### `examples/dump_cell_para_shapes.rs` (신설)

cell paragraph 자료 자체 자체 자체 dump 도구 — para_shape·char_shape count·text 자체 자체 출력. 진단 도구로 영구 보존.

### `mydocs/feedback/task_m600_30_visual_issues.md` (신설)

시각 튜닝 이슈 트래커. 사용자 시각 보고 자체 자체 누적·정리.

## 검증

### 코드 회귀

| 자료 | 결과 |
|---|---|
| `cargo test --workspace --lib` | 1498 passed / 0 failed |

### 자동 e2e — round-trip cell(1,1) char_shapes count

| paragraph | 원본 | fix 전 (round-trip) | fix 후 |
|---|---|---|---|
| p0 ("❶ NIPA NXT...") | count=3 | count=1 (손실) | count=3 ✓ |
| p1 (빈 단락) | count=1 cs=16 | count=1 cs=16 | count=1 cs=16 ✓ |
| p2 ("- 변경개요...") | count=9 | count=1 (손실) | count=9 ✓ |
| p3 (빈 단락) | count=1 cs=25 | count=1 cs=25 | count=1 cs=25 ✓ |

### 사용자 시각 검증

들여쓰기·굵게 자료 회복 확인 (cycle 30 종결 시점).

## 남은 자료 (cycle 31 자료)

cycle 31 의 deep diff baseline 자체 자체 자체 — *일부 cell 의 char_shapes 자료가 여전히 1개 줄어듦* (예: 4 → 3, 3 → 2). cycle 30 fix 가 모든 cell 자체 자체 catch 못 함. start_pos 자료 의미가 cell 마다 다를 가능성 — 후속 cycle 자체 자체 좁힘.
