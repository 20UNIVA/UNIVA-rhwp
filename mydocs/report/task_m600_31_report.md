# Task #m600-31 최종 결과 보고서 — HWPX round-trip deep diff 회귀 테스트

## 사이클 요약

cycle 24~30 의 5건 fix 가 모두 시각 보고를 통해서만 catch 되었음. `ir-diff` 명령의 비교 범위 (paragraph 단위) 가 *PageDef·Table.common·cell 안쪽 paragraph 자료* 를 다루지 않아 결함을 사전에 잡지 못함. 본 cycle 은 *snapshot 기반 회귀 테스트* 박아 향후 동등 결함을 `cargo test` 단계에서 catch.

## 변경

### `samples/hwpx_roundtrip/` (신설)

| 파일 | 자료 |
|---|---|
| `baseline_business_table.hwp` | cycle 25~30 baseline. 5 tables, 1 nested, 24 cells |
| `multi_section_nested.hwpx` | 3 sections, 18 tables, 4 nested, 3 pictures, 673 cells |
| `pictures_equations.hwp` | 22 tables, 14 pictures, 2 equations, 322 cells |

### `tests/hwpx_roundtrip_deep_diff.rs` (신설)

각 fixture 별 `serialize_hwpx` / `serialize_document` round-trip 의 IR deep diff 박고 baseline 파일과 snapshot 비교. 비교 범위:

- PageDef margin·size
- Section paragraphs 개수
- Paragraph 의 controls 자료 (재귀 — nested table 도 같은 검사)
- Table.common (width·height·wrap·tac·vert_rel·horz_rel) + outer_margin + page_break + size + cells.len()
- Cell paragraph 의 para_shape_id·char_shapes.len()·line_segs.len()·controls

### `samples/hwpx_roundtrip/*.baseline.txt` (자동 생성)

현재 시점의 손실 자료 자체 자체 박힌 snapshot. 처음 실행 시 자동 생성. 이후엔 동일하면 통과. `RHWP_UPDATE_BASELINES=1` 으로 갱신.

### `examples/inspect_hwp.rs` (신설)

fixture 자료 구성 dump (sections·tables·nested·pictures·equations·cells 카운트 + PageDef margin).

## 검증

```
$ cargo test --test hwpx_roundtrip_deep_diff
running 3 tests
test fixture_baseline_business_table ... ok
test fixture_pictures_equations ... ok
test fixture_multi_section_nested ... ok

test result: ok. 3 passed; 0 failed; 0 ignored
```

## baseline 손실 자료 패턴

전부 HWPX path 에서만 발생 (`[HWPX]` 항목). HWP path 는 모두 정합 (`[HWP]` 0건).

| baseline 자료 | 의미 |
|---|---|
| char_shapes.len() (예: 4 != 3) | cycle 30 fix 후에도 일부 cell 의 char_shape 자료 *1개* 줄어듦. cycle 30 의 fix 가 모든 cell 자체 자체 catch 못 함 — start_pos 자료 의미가 cell 마다 다를 가능성 |
| controls.len() (예: 2 != 0) | cell paragraph 의 Field·Footnote·기타 control 자료 손실 (cycle 28 는 Table·Picture 만 박음) |
| line_segs.len() (1 != 2) | cell paragraph 의 reflow lineseg 수 차이 |
| top-level controls.len() | section paragraph 의 일부 control 자료 손실 (cell 안 아닌 자리) |

이 baseline 들이 *현재 알려진 손실* 자료 자체. 향후 새 결함이 들어와 baseline 보다 항목이 늘어나면 `cargo test` fail. fix 가 들어와 baseline 보다 줄어들면 `RHWP_UPDATE_BASELINES=1` 으로 갱신해 좁힘.

## 의의

자동 catch 망 박힘. 사용자 시각 보고 없이도:
- cycle 28 의 PageDef margin 손실 자체 자체 *지금부터 catch 가능*
- cycle 30 의 char_shapes count 손실 자체 자체 *catch 가능*
- 새 cycle 의 동등 결함 자체 자체 *catch 가능*

baseline 자료 자체가 *현재 남은 손실 자료 표면화* — 이걸 좁히는 후속 cycle 박을 수 있음.
