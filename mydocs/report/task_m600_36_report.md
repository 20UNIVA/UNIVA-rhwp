# Task #m600-36 최종 보고서

body paragraph 의 PageNumberPos 자체 자체 자체 자체 hp:ctrl wrapper 자체 자체 자체 자체. cycle 31 baseline 자체 자체 자체 자체 자체 *top-level controls.len() 1 != 0* 회수.

## 결과

| Fixture | 종전 | 본 cycle 후 | 변화 |
|---|---|---|---|
| pictures_equations.hwp | 2 | 1 | **-1** |

## 누적 결과 (cycle 33+34+35+36)

cycle 31 baseline 65 → 11 (**-54건, 83% 회수**). cycle 4종 결합 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체.

| Cycle | 결함 자리 | 자체 | 변화 |
|---|---|---|---|
| 33 | table.rs cell paragraph controls | Field·Bookmark·Hyperlink hp:ctrl wrapper | -6 |
| 34 | table.rs char_shapes end skip | 빈 hp:run charPrIDRef 박기 | -44 |
| 35 | table.rs reflow 조건 | `<= 1` → `is_empty()` | -3 |
| 36 | section.rs body paragraph | render_paragraph_ctrls + PageNumberPos | -1 |
| **합계** | | | **-54** |

## 코드 자체

- *추가*: [src/serializer/hwpx/section.rs](../../src/serializer/hwpx/section.rs) `render_paragraph_ctrls`·`render_page_num` 함수.
- *수정*: write_section 의 두 번째 이후 paragraph 박는 자료 자체 자체 자체 자체 hp:ctrl wrapper 자체.
- *갱신*: [samples/hwpx_roundtrip/pictures_equations.hwp.baseline.txt](../../samples/hwpx_roundtrip/pictures_equations.hwp.baseline.txt) (2→1).

## 남은 자료 (11건)

후속 cycle 후보 — *nested cell 자체 자체 variant·다른 control variant*. cycle 4종 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 (예: Footnote·Endnote·Equation·Shape).

세부:
- multi_section_nested 10건 — cell controls (nested)
- pictures_equations 1건 — `s0.p179.c0.tbl.cell(2,1).p0.controls.len()`

dump 자체 자체 자체 자체 자체 자체:
```
cargo run --example dump_baseline_controls -- samples/hwpx_roundtrip/multi_section_nested.hwpx | grep "controls=\[" | head -20
```
