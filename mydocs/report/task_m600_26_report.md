# Task #m600-26 최종 결과 보고서 — hwpx header borderFill fillBrush 자료 직렬화

## 사이클 요약

cycle 25 종결 후 *그라데이션 색띠 + 셀 색상 손실* 발견. 결함 자리 — [src/serializer/hwpx/header.rs:223-229](../../src/serializer/hwpx/header.rs#L223-L229) 의 `fillBrush` 빈 래퍼 (Stage 1 미완). Solid/Gradient/Image fill 자료 전부 손실되어 클라가 받은 hwpx 자료에 색·패턴 자료 없음.

## 변경

### `src/serializer/hwpx/header.rs`

- `write_fill_inner` 함수 신설 — fill_type 별 자식 자료 직렬화
  - **Solid** → `<hc:winBrush faceColor hatchColor [hatchStyle]/>`
  - **Gradient** → `<hc:gradation type angle centerX centerY step stepCenter><hc:color value/>*</hc:gradation>`
  - **Image** → `<hc:imgBrush mode bright contrast><hc:img binaryItemIDRef/></hc:imgBrush>`
- 헬퍼 — `hatch_style_str`, `gradient_type_str`, `image_fill_mode_str` (parser utils 의 parse_* 역방향)
- `write_border_fill` 의 빈 래퍼 자리에 `write_fill_inner(w, &bf.fill)` 호출
- Solid 의 `pattern_type < 1` (parser 기본값 -1 = 무늬 없음) 자리에 hatchStyle 속성 박지 않음 — 단색 셀 줄무늬 결함 fix

## 검증

| 자리 | 결과 |
|---|---|
| `cargo test --workspace --lib` | 1491 passed / 0 failed |
| `cargo clippy --workspace --lib` | warning 0 |
| sim-1781951056 의 bf 8/11 (Gradient) | LINEAR `#3057B9 ↔ #DFE6F7` stops 정확 박힘 |
| sim-1781951056 의 bf 7 (Solid) | winBrush hatchStyle 없음, 단색 표시 |
| 사용자 시각 — 그라데이션·셀 색상 | 정상 |

## 남은 결함 (cycle 27)

`DocumentCore::from_bytes` 의 `paginate()` 호출이 *cell[0] paragraph.line_segs[0].line_height* 자료를 `100 → 3603` (= 표 전체 높이) 으로 mutate. cell[2] 영향 없음 — *cell[0] 만 paragraph 0.0 의 첫 줄 자료로 잘못 박힘*. 결과 — row 0 cell 높이가 row 1 자료처럼 두꺼워짐. paginate path 의 cell[0] 부수효과 차단이 cycle 27 본질.
