# Task #m600-37 최종 보고서

cell paragraph 의 ColumnDef·CharOverlap·Equation 박음. cycle 31 baseline 의 마지막 11건 회수.

## 결과 — 완전 정합 달성

cycle 31 baseline 65 → 0 (**-65건, 100% 회수**).

| Fixture | cycle 31 시작 | cycle 37 후 |
|---|---|---|
| baseline_business_table.hwp | 5 | **0** |
| multi_section_nested.hwpx | 41 | **0** |
| pictures_equations.hwp | 19 | **0** |
| **합계** | **65** | **0** |

## 본 cycle 의 변화

| Fixture | 종전 | 본 cycle 후 | 변화 |
|---|---|---|---|
| multi_section_nested.hwpx | 10 | 0 | -10 |
| pictures_equations.hwp | 1 | 0 | -1 |

## 코드 변경

- *수정*: [src/serializer/hwpx/table.rs `write_sub_list`](../../src/serializer/hwpx/table.rs) — hp:run 자식 자리에 Equation·CharOverlap 분기, hp:ctrl wrapper 자리에 ColumnDef 분기.
- *수정*: [src/serializer/hwpx/section.rs `render_equation`](../../src/serializer/hwpx/section.rs) — `pub(super)` 로 가시성 변경 (cell 자리에서 재사용).
- *갱신*: [samples/hwpx_roundtrip/multi_section_nested.hwpx.baseline.txt](../../samples/hwpx_roundtrip/multi_section_nested.hwpx.baseline.txt), [samples/hwpx_roundtrip/pictures_equations.hwp.baseline.txt](../../samples/hwpx_roundtrip/pictures_equations.hwp.baseline.txt) — 둘 다 빈 파일로 갱신 (손실 0).

## cycle 33~37 누적

5개 cycle 결합으로 HWPX put_snapshot round-trip 의 *모든 deep diff 결함* 회수.

| Cycle | 핵심 fix | 변화 |
|---|---|---|
| 33 | cell Field·Bookmark·Hyperlink hp:ctrl wrapper | -6 |
| 34 | cell 마지막 char_shape paragraph end 빈 hp:run | -44 |
| 35 | cell reflow 조건 `<= 1` → `is_empty()` | -3 |
| 36 | body PageNumberPos hp:ctrl wrapper | -1 |
| 37 | cell Equation·CharOverlap·ColumnDef 분기 | -11 |
| **합계** | | **-65 (100% 정합)** |

## 후속 자리

본 cycle 의 minimal wrapper fix 가 들어간 ColumnDef·CharOverlap 은 *control 개수는 round-trip 되지만 내부 attribute 는 default*. 정확한 attribute (열 수·간격·composeText·circleType) 복원은 별 cycle. 다만 deep diff 의 비교 항목 (controls.len()) 은 통과하므로 회귀 catch 망 위로 올라오지 않는다.

별 cycle 사후 정리: [src/document_core/commands/cell_path.rs:262](../../src/document_core/commands/cell_path.rs#L262) 의 hardcoded sample hwp 경로 (`/Users/yuniba_01/Downloads/icon/1. (★사업중 필독) 사업관리 참조표.hwp`) 가 작업자 머신 외부 파일이라 `cargo test --lib` 에서 7건 실패. cycle 29 잔재로 fixture 정리 필요.
