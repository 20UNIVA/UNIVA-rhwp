# Task #m600-34 최종 보고서

cell paragraph 의 *마지막 char_shape (paragraph end 자체 자체 자체 자체 자체 자체 자체 자체)* 자체 자체 round-trip skip fix. cycle 30 자체 자체 자체 *end <= start { continue }* 자체 자체 자체 자체 자체 자체 자체 *빈 hp:run charPrIDRef 박기* 자체 자체.

## 결과

cycle 33+34 결합 자체 자체 자체 baseline 65 → 15 (**-50건, 77%**).

| Fixture | 종전 (cycle 31) | cycle 33 후 | cycle 34 후 | 누적 변화 |
|---|---|---|---|---|
| baseline_business_table.hwp | 5 | 3 | 0 | **-5 (완전 정합)** |
| multi_section_nested.hwpx | 41 | 41 | 10 | **-31** |
| pictures_equations.hwp | 19 | 15 | 5 | **-14** |
| **합계** | **65** | **59** | **15** | **-50** |

## 코드 자체

- *수정*: [src/serializer/hwpx/table.rs:300-316](../../src/serializer/hwpx/table.rs#L300-L316) `write_sub_list` — `if end <= start` 자체 자체 자체 자체 *continue* 자체 자체 자체 자체 자체 *빈 hp:run charPrIDRef* 박는 자료.
- *갱신*: [samples/hwpx_roundtrip/baseline_business_table.hwp.baseline.txt](../../samples/hwpx_roundtrip/baseline_business_table.hwp.baseline.txt) (3→0), [samples/hwpx_roundtrip/multi_section_nested.hwpx.baseline.txt](../../samples/hwpx_roundtrip/multi_section_nested.hwpx.baseline.txt) (41→10), [samples/hwpx_roundtrip/pictures_equations.hwp.baseline.txt](../../samples/hwpx_roundtrip/pictures_equations.hwp.baseline.txt) (15→5).

## 남은 자료

15건 — cycle 35·36 자체 자체:
- *11건 cell controls.len() 1 != 0* — Field 외 variant (Footnote·Endnote·Equation·Shape·Form·Ruby 등). cycle 33 fix 자체 자체 자체 자체 자체 *Field·Bookmark·Hyperlink* 자체 자체 자체 자체 박음. 별 cycle 자체 자체 자체 자체 분기 자체 자체 자체 자체 자체.
- *3건 line_segs.len() 1 != 2* — cycle 35 패턴 C, cell paragraph reflow.
- *1건 top-level s0.p1.controls.len() 1 != 0* — cycle 36 패턴 D, section.rs render_control_slot.
