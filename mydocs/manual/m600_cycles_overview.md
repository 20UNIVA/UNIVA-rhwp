# M600 (HWPX 라운드트립 결함) 사이클 인덱스

원본 hwp `1. (★사업중 필독) 사업관리 참조표.hwp` 의 시각 결함 자체 시작해 rhwp-server / WASM client 의 HWPX put_snapshot 워크플로우에서 round-trip 결함을 cycle 단위로 fix 한 자료.

## 한눈에 보기

| Cycle | 결함 자리 / 자료 | 결과 | spec / report |
|---|---|---|---|
| 24 | cell paragraph make_line_seg fallback (시각 회귀) | **revert** | [plans](../plans/task_m600_24.md) · [report](../report/task_m600_24_report.md) |
| 25 | cell paragraph line_segs 하드코딩 (vertsize=1000) | fix | [plans](../plans/task_m600_25.md) · [report](../report/task_m600_25_report.md) |
| 26 | header borderFill fillBrush 빈 래퍼 (그라데이션·셀 색상 손실) | fix | [plans](../plans/task_m600_26.md) · [report](../report/task_m600_26_report.md) |
| 27 | section.rs `replace_first_linesegs` 호출 순서 (paragraph 0.0 ↔ cell 첫 lineseg swap) | fix | [plans](../plans/task_m600_27.md) · [report](../report/task_m600_27_report.md) |
| 28 | HWPX put_snapshot round-trip 4건 (raw_ctrl_data·page_break·cell controls·PageDef margin) | fix | [plans](../plans/task_m600_28.md) · [report](../report/task_m600_28_report.md) |
| 29 | 이중 표 (nested table) cell 편집 op 추가 — `ReplaceCellRunsAtPath` | feat | [plans](../plans/task_m600_29.md) · [report](../report/task_m600_29_report.md) |
| 30 | cell paragraph 의 부분 char_shape 박지 못함 (들여쓰기·굵게 손실) | fix | [plans](../plans/task_m600_30.md) · [report](../report/task_m600_30_report.md) |
| 31 | round-trip deep diff snapshot 회귀 테스트 박음 (fixture 3개 + baseline) | infra | [plans](../plans/task_m600_31.md) · [report](../report/task_m600_31_report.md) |

## 결함의 공통 패턴

cycle 24~30 의 fix 자료는 모두 *같은 골격*:

> **HWPX serializer 의 `<hp:tbl>` → `<hp:subList>` → `<hp:p>` → `<hp:run>` 4단계 안쪽 자료가 "최소 박음" 으로 멈춰 부분 스타일·자료 보존을 빠뜨림.**

자체 자체 자체 자체 — HWP 직렬화기는 *원본 자료에 가까운 라운드트립* 자체 자체 정합되어 있지만 HWPX 직렬화기의 *cell 안 자료 박는 자리* 가 단순 자료에 멈춰있음. cycle 31 의 자동 catch 망이 이 표면을 표면화.

## 각 cycle 의 핵심 자료

### Cycle 24 — make_line_seg fallback 의 시각 회귀 (revert)

**증상**: `replace_cell_runs` 후 새로고침하면 표 안 cell 자료의 vertical layout 자체 자체 깨짐.

**시도**: `make_line_seg` 의 12pt fallback 자료 자체 자체 추가.

**결과**: 시각 회귀 발견 자체 자체 *완전 revert* (`7d2b37dc` → `9e96fa14`). 자체 자체 자체 자체 자체 결함 자리가 fallback 이 아닌 *직렬화 path* 자체 자체 확정.

### Cycle 25 — cell paragraph line_segs IR pass-through

**증상**: server export hwpx 의 cell paragraph 자체 자체 자체 vertsize=1000 정적 lineseg 박음. WASM 의 LinesegTextRunReflow 경고 + auto-fix 부수효과.

**결함 자리**: [src/serializer/hwpx/table.rs:write_sub_list](../../src/serializer/hwpx/table.rs) — cell paragraph 자체 자체 자체 단일 정적 lineseg 박음.

**Fix**: cell 폭 자체 자체 `reflow_line_segs` 호출 + IR 의 lineseg 6 필드 그대로 직렬화. `SerializeContext` 에 `resolved_styles`·`dpi` 추가.

**commit**: `b605b4c6` → `966fa67b` (merge)

### Cycle 26 — header borderFill fillBrush 자료 직렬화

**증상**: 그라데이션 색띠 + 셀 색상 자료 손실. 단색 셀에 줄무늬 박힘.

**결함 자리**: [src/serializer/hwpx/header.rs:write_border_fill](../../src/serializer/hwpx/header.rs) — `<hc:fillBrush>` 빈 래퍼만 박음.

**Fix**: `write_fill_inner` 신설 — Solid (`<hc:winBrush>`) / Gradient (`<hc:gradation>` + `<hc:color>`) / Image (`<hc:imgBrush>` + `<hc:img>`) 직렬화. `pattern_type < 1` 자체 `hatchStyle` 자체 자체 박지 않음 (줄무늬 결함 fix).

**commit**: `e2593dc5` → `1e7e8200` (merge)

### Cycle 27 — section.rs replace_first_linesegs 호출 순서 결함

**증상**: 3행 1열 그라데이션 표의 row 0 / row 2 가 row 1 만큼 두꺼워짐.

**결함 자리**: [src/serializer/hwpx/section.rs:write_section](../../src/serializer/hwpx/section.rs#L72) — `replace_first_linesegs(out, first_linesegs)` 자체 자체 `replacen(TEXT_SLOT, first_t, 1)` 뒤에 호출. first_t 안에 박힌 *표 cell 의 `<hp:linesegarray>`* 가 `find("<hp:linesegarray>")` 의 첫 매칭으로 잡혀 cell[0] 의 vertsize 자체 paragraph 0.0 의 lh 자료로 덮어쓰임.

**Fix**: 두 호출 순서 뒤집음 — `replace_first_linesegs` 가 template 의 유일한 linesegarray 자리만 매칭.

**commit**: `23eb3d89` → `ab46fd9a` (merge). 별도 ir-slice 응답 옵션 commit `9d12c42d`.

### Cycle 28 — HWPX put_snapshot round-trip 결함 4건

사용자 워크플로우 — WASM client 가 HWPX 자료로 PUT /snapshot 박은 후 HWP export 하면 *왼쪽/오른쪽 여백 손실, 표 칸별 찢어짐, 이중 표 내부 표 사라짐* 결함.

**Fix 4건**:

| # | 자리 | 자료 |
|---|---|---|
| 1 | [control.rs:455](../../src/serializer/control.rs#L455) `serialize_table` | `raw_ctrl_data.is_empty()` 자체 자체 common 자체 자체 합성 fallback (`build_table_ctrl_data_from_common`). HWPX parser 가 `raw_ctrl_data = Vec::new()` 박는 자료 자체 자체 자체 자체 자체 손실 catch. |
| 2 | [hwpx/table.rs:438](../../src/serializer/hwpx/table.rs#L438) `table_page_break_str` | 매핑 뒤집음 — HWPX `"CELL" ↔ HWP5 RowBreak`, HWPX `"TABLE" ↔ HWP5 CellBreak` (한컴 명명 규약 정합). |
| 3 | [hwpx/table.rs:278](../../src/serializer/hwpx/table.rs#L278) `write_sub_list` | cell paragraph 의 `controls` 자료 박음 (nested table·picture 자체 자체 자체 자체 자체). |
| 4 | [hwpx/section.rs](../../src/serializer/hwpx/section.rs) `replace_page_pr` 신설 | EMPTY_SECTION_XML template 의 `<hp:pagePr><hp:margin/></hp:pagePr>` 영역 자체 자체 PageDef 자료 자체 자체 자체 교체. 종전 template 의 고정값 (L=8504 R=8504 등) 박혀 원본 PageDef 자료 자체 손실. |

**commit**: `bb31a566` → `fa9e68d0` (merge) + 진단 helper examples `4dfb8c7b` → `7e1e3e9c` (merge).

### Cycle 29 — 이중 표 cell 편집 op 추가

기존 cell 편집 op 4종 (`ReplaceCellRuns`·`InsertTextInCell`·`DeleteRangeInCell`·`SetCellStyle`) 자체 자체 *최상위 표 cell* 만 가리킴. nested cell 자체 자체 자체 가리키는 op 부재.

**자료**: `src/document_core/commands/cell_path.rs` 신설.
- `CellPath { steps: Vec<CellStep> }` + `CellStep { para, ctrl_idx, row, col }`
- `DocumentCore::get_cell_mut_at_path` — path 따라 cell mut ref 박음
- `DocumentCore::replace_cell_runs_at_path_native` — cell paragraph 자체 자체 자체 자체 자체 자체 paginate_if_needed
- 단위 테스트 7개 (depth 1·2·nested 6 cells·empty·invalid·replace) 모두 통과

**EditOperation**: `ReplaceCellRunsAtPath { section, path, cell_para, runs }` variant 추가. workbench action `"replace_cell_runs_at_path"` 라우트 박음. WS broadcast 자료 자체 자체 자동 전파.

**검증**: 원본 hwp 의 nested 3x2 cell(1, 1) 의 p0.text 자체 자체 "NESTED_CELL_EDIT_OK" 자체 자체 round-trip 보존.

**commit**: `8b11f8d5` → `4ae9a481` (merge).

### Cycle 30 — cell paragraph 부분 char_shape 박지 못함

**증상**: cell 안 paragraph 의 들여쓰기·굵게 자체 자체 새로고침 후 사라짐.

**결함 자리**: [src/serializer/hwpx/table.rs:270](../../src/serializer/hwpx/table.rs#L270) `write_sub_list` 안 hp:run 박는 자리가 `char_shapes.first()` 자료만 박음. 부분 char_shape 손실.

**Fix**: char_shapes 별로 별도 `<hp:run charPrIDRef>` 박음. start_pos 자료 (utf16 offset) 자체 자체 paragraph.text 분할. start_pos 자체 자체 paragraph end marker·control 자체 자체 utf16 위치 포함이라 `text_u16.len()` clamp. empty text 자체 자체 자체 자체 빈 hp:run 박아 char_shape_id 자체 보존.

**검증**: 원본 cell(1, 1) 의 4 paragraph char_shapes count (3, 1, 9, 1) 모두 round-trip 보존. 사용자 시각 OK.

**commit**: `271a55a1` → `032940c9` (merge).

### Cycle 31 — round-trip deep diff snapshot 회귀 테스트

cycle 24~30 의 5건 fix 자료가 모두 사용자 시각 보고를 통해서만 catch. 기존 `ir-diff` 명령은 paragraph 단위 비교만 박아 PageDef·cell 안쪽 자료 손실 자체 자체 자체 못 잡음.

**자료**: `tests/hwpx_roundtrip_deep_diff.rs` + `samples/hwpx_roundtrip/` fixture 3개.

- `baseline_business_table.hwp` — cycle 25~30 baseline (이중 표·HWP)
- `multi_section_nested.hwpx` — 3 sections·18 tables·4 nested·3 pictures·673 cells
- `pictures_equations.hwp` — 22 tables·14 pictures·2 equations·322 cells

**비교 항목**: PageDef margin·size / Section paragraphs / Paragraph controls (재귀) / Table.common (width·height·wrap·tac·vert_rel·horz_rel·outer_margin·page_break·size·cells.len()) / Cell paragraph 의 para_shape_id·char_shapes.len()·line_segs.len()·controls.

**baseline 자료**: `samples/hwpx_roundtrip/*.baseline.txt` 자체 자체 자체 *현재 알려진 손실 자료 자체 자체 박힌 snapshot*. 새 결함 시 fail. fix 후 좁힐 때 `RHWP_UPDATE_BASELINES=1` 자체 갱신.

**현재 baseline 의 손실 패턴** (전부 HWPX path):
- `char_shapes.len()` 손실 — cycle 30 fix 후에도 *일부 cell* 의 char_shapes 자체 1개 줄어듦
- `controls.len()` 손실 — cell paragraph 의 Field·Footnote 자체 자체 자체 자체 (cycle 28 는 Table·Picture 만)
- `line_segs.len()` 손실 — cell paragraph reflow lineseg 수 차이
- top-level paragraph `controls.len()` — section paragraph 의 일부 control 자체

HWP path 는 0건 (정합).

**commit**: `f434eba5` → `41cd805a` (merge).

## 새 세션 진입 시 참고

1. 이 문서 자체 자체 cycle 단위 자료 한눈에 박혀있음.
2. 각 cycle 의 *결함 자리·fix 코드 path·commit 해시* 자체 자체 자체 자체 — 직접 코드 정독 시 이 자료 자체 자체 박은 후 진입.
3. `tests/hwpx_roundtrip_deep_diff.rs` 의 baseline 자료 자체 자체 *현재 알려진 손실 표면화*. 후속 cycle 자체 자체 자체 좁힘.
4. `mydocs/feedback/task_m600_30_visual_issues.md` 자체 자체 *사용자 시각 보고 트래커*. 새 시각 결함 추가 시 박음.
5. `examples/dump_cell_para_shapes.rs`·`examples/dump_nested.rs`·`examples/dump_cell_paragraphs.rs`·`examples/inspect_hwp.rs`·`examples/dump_margins.rs`·`examples/dump_nested_cell_text.rs`·`examples/dump_nested_cell_text2.rs`·`examples/roundtrip_hwp.rs`·`examples/hwp_to_hwpx.rs` 자체 자체 진단 helper. 비슷한 결함 catch 시 활용.

## 후속 cycle 자료 (열린 자료)

- cycle 31 baseline 자체 자체 자체 손실 자료 좁힘 — `char_shapes` 자료 자체 자체 자체 일부 cell 자체 자체 자체 catch 못 함, `controls.len()` 자체 자체 자체 Field·Footnote 자체.
- 이중 표 cell 의 `InsertTextInCellAtPath`·`DeleteRangeInCellAtPath`·`SetCellStyleAtPath` variant 자체 자체 자체 자체.
- WASM UI 의 nested cell click → path 박는 자료 자체 자체 자체 자체.
- 기존 cell 변종 4종의 path variant 자체 자체 자체 자체 마이그레이션 (호환 위해 별도).
- Picture·Shape 자료 자체 자체 deep diff 추가 (cycle 31 의 비교 항목 자체 자체 자체 그림·도형 자체).

## 진단 helper 분포

`examples/` 자체 자체:

| 파일 | 자료 |
|---|---|
| `roundtrip_hwp.rs` | hwp parse → serialize → write + raw_ctrl_data·common 자체 자체 dump |
| `hwp_to_hwpx.rs` | hwp → hwpx 자료 변환 |
| `dump_nested.rs` | Top·nested table 재귀 dump (raw_ctrl_data.len 포함) |
| `dump_cell_paragraphs.rs` | cell 안 paragraph 의 text·controls 자료 |
| `dump_margins.rs` | PageDef margin + ParaShape margin 자료 |
| `dump_cell_para_shapes.rs` | cell paragraph 의 para_shape·char_shape count·text |
| `dump_nested_cell_text.rs`·`dump_nested_cell_text2.rs` | nested cell text 자체 dump |
| `inspect_hwp.rs` | hwp 자료 자체 자체 자체 자체 구조 (sections·tables·nested·pictures·equations·cells·PageDef) |

이 자료 자체 자체 자체 자체 자체 결함 catch 시 *시각 보고만으로는 안 보이는 자료* 자체 자체 자체 자체 자체 자체 직접 dump.
