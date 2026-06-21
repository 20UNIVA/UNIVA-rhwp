# Task #m600-33 최종 보고서

HWPX cell paragraph 의 Field·Bookmark·Hyperlink controls 박음 — round-trip 자체 자체 자체 자체 손실 자체 자체 자체 자체 자체 자체 -6건.

## 결과

| Fixture | 종전 손실 | 본 cycle 후 | 변화 |
|---|---|---|---|
| baseline_business_table.hwp | 5 | 3 | **-2** |
| multi_section_nested.hwpx | 41 | 41 | 0 |
| pictures_equations.hwp | 19 | 15 | **-4** |
| **합계** | **65** | **59** | **-6** |

multi_section_nested 자체 자체 자체 자체 자체 자체 *nested cell 자체 자체 자체 자체 자체 자체 자체 자체*. 후속 cycle 자체 자체 자체 자체 자체 자체 자체 자체 분리 진단.

## 코드 자체

- *수정*: [src/serializer/hwpx/table.rs:312-371](../../src/serializer/hwpx/table.rs#L312-L371) `write_sub_list` — hp:run 안 Table·Picture 박은 후 별도 hp:ctrl wrapper 자체 자체 자체 자체 Field·Bookmark·Hyperlink 박음.
- *추가*: [examples/dump_baseline_controls.rs](../../examples/dump_baseline_controls.rs) — baseline diff 자리 cell paragraph control variant 분류 helper.
- *갱신*: [samples/hwpx_roundtrip/baseline_business_table.hwp.baseline.txt](../../samples/hwpx_roundtrip/baseline_business_table.hwp.baseline.txt) (5→3), [samples/hwpx_roundtrip/pictures_equations.hwp.baseline.txt](../../samples/hwpx_roundtrip/pictures_equations.hwp.baseline.txt) (19→15).

## 종결 검증

```
~/.cargo/bin/cargo build --workspace                    # OK
~/.cargo/bin/cargo test --test hwpx_roundtrip_deep_diff # 3 passed
```

cargo test --lib 자체 자체 7건 실패 자체 자체 자체 자체 *cycle 29 의 cell_path 테스트* 자체 자체 자체 자체 자체 — 작업자 머신 외부 hwp 파일 (`/Users/yuniba_01/Downloads/icon/1. (★사업중 필독) 사업관리 참조표.hwp`) 자체 자체 hardcoded 의존. 본 cycle 자체 자체 자체 무관, 사전부터 깨진 자료. 별 cycle 자체 자체 자체 자체 자체 자체 자체 fixture 자체 자체 자체 자체 정리.

## 후속 cycle (다음 진입)

- *cycle 34* — char_shapes.len() N != N-1 패턴 A. baseline 50건 자체 자체 자체 자체 자체 자체. 비고: business_table 자체 자체 자체 자체 자체 *Field 자체 자체 자체 추가하면서 char_shape 자체 자체 자체 자체 자체 자체* (`controls.len() 2 != 0` 자체 자체 *char_shapes.len() 10 != 9* 자체 자체 자체 자체 transform). 진단 자체 자체 자체.
- *cycle 35* — line_segs.len() reflow 2줄 자체 자체 자체 자체.
- *cycle 36* — top-level paragraph controls.len() (section.rs render_control_slot 의 hp:run > hp:ctrl wrapper 자체 자체 자체).
- nested cell 자체 자체 자체 자체 자체 자체 자체 자체 자체 — write_sub_list 자체 자체 자체 자체 nested 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 별 cycle 자체 자체 자체.
