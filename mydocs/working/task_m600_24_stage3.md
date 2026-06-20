# Task #m600-24 Stage 3 — 단위 테스트 + 회귀 검증 보고서

## 단위 테스트 추가

[src/renderer/composer/tests.rs:443-475](../../src/renderer/composer/tests.rs#L443-L475) 자리에 두 자료 추가:

1. `test_reflow_empty_text_uses_first_char_shape_font_size`
   - 빈 텍스트 + `char_shapes[0]` font_size=1.0px → 첫 char_shape 자료 차용
   - 검증 — `line_height == 75` HWPUNIT (1.0 * 7200/96)
   - 변경 전 자료에서는 *12.0px 폴백 자리 line_height=900* 이 박혔던 자리

2. `test_reflow_empty_text_no_char_shapes_falls_back_12px`
   - 빈 텍스트 + char_shapes 비어 있음 → 12.0px 최종 폴백
   - 검증 — `line_height == 900` HWPUNIT (12.0 * 7200/96, 안전장치 작동)

## 회귀 검증

```
~/.cargo/bin/cargo test --workspace --lib --quiet
test result: ok. 1493 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 53.05s
```

이전 자료 1491 passed → 1493 passed (새 단위 테스트 2개). 회귀 0건.

## clippy 검증

```
~/.cargo/bin/cargo clippy --workspace --lib --quiet
# (무출력 = warning 0)
```

## 자료 정정 사항

Stage 2 보고서 작성 자리에서 *12.0 폴백 단위* 가 *pt* 가 아니라 *px* 였음을 단위 테스트 자료로 확인. spec md (`docs/24-cell-paginate-blowup-spec.md` §3.1·§3.2) 의 "12pt 폴백" 자료는 *12 px ≈ 9pt* 가 정확. 결함 자리 자체 (빈 셀 paragraph 가 *원본 1.0px 자리 자료가 아니라 폴백 자료가 적용되는 자리*) 는 정합 유지.

## 다음 자리 — Stage 4

- WASM 재빌드 (`docker compose --env-file .env.docker run --rm wasm`) 로 클라 pkg/ 갱신.
- studio dist 재빌드.
- server 재기동 (7710 점유 PID 종료 후 새 binary).
- 시크릿탭에서 spec §4 재현 절차 진행 → §6 시각 검증 4 항목.
- sub-agent 시각 비교 보고서 첨부.
