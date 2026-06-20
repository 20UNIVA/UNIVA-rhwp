# Task #m600-24 Stage 1 — §3 가설 코드 path 정독 보고서

## 결론 요약

- spec §3.1 가설 (`make_line_seg` 12pt 폴백) 자료는 *방향은 맞으나 직접 원인 자리가 다름*. 빈 paragraph 945-947 자리만이 아니라 *fill_lines 결과 자리 997-1002* 도 같은 12pt 폴백 패턴.
- spec §3.2 가설 (row 0·row 2 빈 paragraph 의 945-947 자리 폴백) 은 *부분 부정*. `replace_cell_runs_native` chain 이 row 0·row 2 빈 paragraph 를 *touch 하지 않으므로* 945-947 자리는 직접 호출되지 않음. *그럼에도 클라 paginate 결과 cell.height 가 폭증*하는 자리는 *클라 측정 path 안의 다른 자리* (recompose_for_cell_width 또는 measured_tables 캐시) 가 원인일 가능성.
- 후보 A 패치 자리 *945-947 + 997-1002 + 1005-1007* 세 자리 모두 *paragraph 첫 char_shape font_size 차용*으로 정정. 이래야 *make_line_seg 의 12pt 폴백이 다다르는 모든 자리* 가 자료 보존 자리로 바뀜.

## 1. make_line_seg 호출 자리 전수 ([line_breaking.rs:922-1007](../../src/renderer/composer/line_breaking.rs#L922-L1007))

| 자리 | 라인 | 호출 형태 | max_font_size 자료 | 폴백 자리 |
|---|---|---|---|---|
| A | 945-947 | `make_line_seg(0, 0.0)` (literal 0.0) | 빈 paragraph 분기 | `make_line_seg` 안 `fs=12.0` |
| B | 997-1002 | `make_line_seg(utf16_start, fs)` | `lb.max_font_size`. 0 이면 *호출 자리에서 12.0 폴백* | 호출자 자리 12.0 + `make_line_seg` 안 분기는 진입 안 함 |
| C | 1005-1007 | `make_line_seg(0, 12.0)` (literal 12.0) | new_line_segs 비어 있을 때 안전장치 | 호출자 자리 literal 12.0 |

자리 A·B·C 모두 *클라가 12pt 자리 line_height 를 산출하는 경로*. 후보 A 패치는 셋 모두에 적용해야 자료 보존 정합.

## 2. CharShapeRef + styles lookup 정정 ([paragraph.rs:135-140](../../src/model/paragraph.rs#L135-L140))

```rust
pub struct CharShapeRef {
    pub start_pos: u32,         // spec 가설서의 `utf16_start` 자리
    pub char_shape_id: u32,     // spec 가설서의 `style_id` 자리 정정
}
```

`styles: &ResolvedStyleSet` 안 `char_styles: Vec<ResolvedCharStyle>` 자리 lookup:

```rust
styles.char_styles.get(cs.char_shape_id as usize).map(|s| s.font_size)
```

`ResolvedCharStyle.font_size: f64` (px 단위, [src/renderer/style_resolver.rs:19-78](../../src/renderer/style_resolver.rs#L19-L78)).

## 3. replace_cell_runs_native chain ([text_editing.rs:825-920](../../src/document_core/commands/text_editing.rs#L825-L920))

| 자리 | 영향 셀 | char_shapes touch |
|---|---|---|
| L851-855 | row 1 cell_para 0 (변경 칸) | 첫 char_shape_id 보관 |
| L859-868 `delete_text_in_cell_native` | row 1 | 부분 변경 + reflow_line_segs 호출 |
| L882-890 `insert_text_in_cell_native` | row 1 | 부분 변경 + reflow_line_segs 호출 |
| L899-909 `apply_char_format_in_cell_native_with_base` | row 1 | 부분 변경 |

*row 0·row 2 빈 paragraph 는 chain 전체에서 touch 자리 없음*. 그럼에도 클라 paginate 결과 cell.height 가 폭증하는 자리 → 측정 path 안의 *재측정 자리* (caching·paginate) 가 변경 영향 받음.

## 4. recompose_for_cell_width 자료 ([composer.rs:1209-1258](../../src/renderer/composer.rs#L1209-L1258))

```rust
if !para.line_segs.is_empty() {
    return;  // 기존 line_segs 있으면 재측정 스킵
}
```

자료 — *기존 line_segs 살아 있으면 재측정 스킵*. row 0·row 2 의 빈 paragraph 가 *line_segs 자료를 유지하고 있는 한* 이 자리는 안전. spec §3.4 의심 2 순위는 *현재 자료 자리*에서 직접 트리거되지 않음.

## 5. font_size_to_line_height 산술 ([line_breaking.rs:1108-1110](../../src/renderer/composer/line_breaking.rs#L1108-L1110))

```rust
fn font_size_to_line_height(font_size_px: f64, dpi: f64) -> i32 {
    px_to_hwpunit(font_size_px, dpi)
}
```

| fs (pt) | px (96 dpi 자리) | HWPUNIT |
|---|---|---|
| 1.0 | 1.33 | 100 |
| 12.0 | 16.0 | 1200 |

(`f` 가 px 단위로 들어온다면 px → HWPUNIT 환산 자리. pt → px 자리는 호출자 측에서 처리.)

spec §3.2 의 row 0 380 → 508 자리 폭증 폭 128 ≈ *fs 1.5pt 자리 자료 + 패딩 합산*. row 2 380 → 2639 자리 폭증 폭 2259 ≈ *fs 12pt 자리 자료 + 누적 패딩*. 정량 정합은 *추가 자리* 자료 확보 필요 (paginate 안 패딩·spacing 합산 자리).

## 다음 자리 — Stage 2

후보 A 패치 자리 확정:

1. L946 — `make_line_seg(0, fs_first)` 로 교체. `fs_first` 는 `para.char_shapes.first()` → `styles.char_styles.get(id)` → `font_size`. 빈 자리 0.0 폴백 유지.
2. L997-1002 — `fs` 산출 자리에 `lb.max_font_size > 0.0` 분기 자료 보존, 폴백 자리에서 *paragraph 첫 char_shape* 자료 사용 (line 의 첫 글자 자리 char_shape 보다 *paragraph 첫* 자료가 안전. line 자체가 빈 자리일 때 자료 보존 자리).
3. L1005-1007 — 같은 자리 *paragraph 첫 char_shape* 자료 사용.

자료 보존 자리는 *paragraph 안 char_shapes first 자리 자료* 단일 지점 자료. helper 함수 `first_char_shape_font_size(para, styles) -> f64` 자리 신설로 자료 중복 자리 자료 없앰.

## 시각 검증 가설

후보 A 패치 후 *row 1 (변경 칸) 의 paginate cell.height 가 12pt 폴백 적용 없이 계산*된다. spec §2.3 의 2563 → 15855 폭증이 *완화될 가능성*. row 0·row 2 폭증은 *직접 해결되지 않을 가능성* — 시각 검증에서 *부분 해결 vs 완전 해결* 확인 후 후속 단계 진입.

후속 단계 보류:
- spec §3.5 (measured_tables 캐시 stale) — row 0·row 2 폭증의 재현 원인일 가능성. 시각 검증에서 미해결 시 진입.
