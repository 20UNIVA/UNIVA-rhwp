# Task #m600-35 최종 보고서

cell paragraph 자체 자체 자체 *원본 line_segs 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체*. cycle 25 자체 자체 자체 `<= 1` 자체 자체 자체 `is_empty()` 자체 자체 자체.

## 결과

cycle 33+34+35 결합 baseline 65 → 12 (**-53건, 82%**).

| Fixture | 종전 (cycle 31) | cycle 33 | cycle 34 | cycle 35 | 누적 변화 |
|---|---|---|---|---|---|
| baseline_business_table.hwp | 5 | 3 | 0 | 0 | **-5 (완전 정합)** |
| multi_section_nested.hwpx | 41 | 41 | 10 | 10 | **-31** |
| pictures_equations.hwp | 19 | 15 | 5 | 2 | **-17** |
| **합계** | **65** | **59** | **15** | **12** | **-53** |

## 코드 자체

- *수정*: [src/serializer/hwpx/table.rs:339-359](../../src/serializer/hwpx/table.rs#L339-L359) reflow 조건 `<= 1` → `is_empty()`.
- *추가*: [examples/dump_lineseg.rs](../../examples/dump_lineseg.rs) 진단 helper — cell paragraph 자체 자체 자체 자체 자체 line_segs 자체 자체 자체.
- *갱신*: [samples/hwpx_roundtrip/pictures_equations.hwp.baseline.txt](../../samples/hwpx_roundtrip/pictures_equations.hwp.baseline.txt) (5→2).

## 남은 자료 (cycle 36 자체 자체)

12건 — cycle 36 자체 자체:
- *11건* cell controls.len() 1 != 0 — Field 외 variant. cycle 33 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체.
- *1건* top-level s0.p1.controls.len() 1 != 0 — section.rs render_control_slot.
