# Task #m600-24 최종 결과 보고서 — `replace_cell_runs` 직후 cell.height 폭증 fix

## 사이클 요약

작업 루트 [docs/24-cell-paginate-blowup-spec.md](../../../docs/24-cell-paginate-blowup-spec.md) 의 *셀 텍스트 교체 후 표 height 폭증·그라데이션 손실* 결함 — spec §5 fix 후보 A 자료로 진행. 가설 자료 일부 부정·자료 정정 동반.

## 변경 자료

| 자리 | 변경 |
|---|---|
| [src/renderer/composer/line_breaking.rs:920-1010](../../src/renderer/composer/line_breaking.rs#L920-L1010) | `reflow_line_segs` 안 `make_line_seg` closure 의 12.0px 폴백을 *paragraph 첫 char_shape font_size 차용* 단일 지점으로 통합. 세 호출 자리 (946 빈 paragraph, 997-1002 fill_lines 결과, 1005-1007 안전장치) 자료 자료 자료 자료. |
| [src/renderer/composer/tests.rs:443-475](../../src/renderer/composer/tests.rs#L443-L475) | 단위 테스트 2 개 추가 — 빈 paragraph + char_shapes[0] fs=1.0px 자리 line_height=75 / char_shapes 비어 있음 자리 12.0px 안전장치 line_height=900. |

commits — [`3e126ed1`](../../) (plan + Stage 1), [`7d2b37dc`](../../) (Stage 2·3 패치 + 단위 테스트).

## 가설 검증 자료

spec §3.1 가설 (`make_line_seg` 의 12pt 폴백) — *방향 정합, 자리 정정*:

- *세 호출 자리 모두* 12.0px 폴백 자리가 박혀 있음 (spec 은 945-947 자리만 적시. fill_lines 결과 자리 997-1002·안전장치 자리 1005-1007 누락).
- 12.0 의 단위가 *pt* 가 아니라 *px*. 12.0px ≈ 9pt 자리. 산출값 line_height=900 HWPUNIT (12.0 * 7200/96).
- *row 0·row 2 빈 paragraph 직접 폭증* 자리는 부분 부정 — `replace_cell_runs_native` chain 이 row 0·row 2 paragraph 를 touch 하지 않으므로 945-947 자리는 *직접 호출되지 않음*. row 0·row 2 폭증 자료는 *측정 path 의 다른 자리* (spec §3.5 measured_tables 캐시 stale 가능성) 가 원인일 가능성.

fix 자체는 *모든 12.0px 폴백 자리를 단일 지점에 통합*하여 *paragraph 첫 char_shape font_size 우선* 자료로 정합. row 1 (변경 칸) reflow 자리의 *fill_lines 결과 max_font_size=0* 자리 자료가 *paragraph 자료 보존 자리*로 정정.

## 검증 결과

| 자리 | 결과 |
|---|---|
| `cargo test --workspace --lib` | 1493 passed / 0 failed / 6 ignored |
| `cargo clippy --workspace --lib` | warning 0 |
| 새 단위 테스트 자료 | 빈 paragraph + char_shapes[0] fs=1.0px 자리 line_height=75 (변경 전 자리 900 과 명확 차이) |
| 시각 검증 자료 (§6 1·2·3·4 항목) | 사용자 시각 자료 진행 가능 상태. 새 WASM pkg + studio dist 갱신, 7710 server 유지. |

## 사용자 시각 자료 안내

- 7710 server PID 59274 유지 (server 측 변경 없음 — spec §2.1·§2.2 자료).
- 새 WASM pkg 자료 (`UNIVA-rhwp/pkg/`) + studio dist 자료 (`UNIVA-rhwp/rhwp-studio/dist/`) 갱신.
- 사용자 자료: *시크릿탭에서* `http://127.0.0.1:7710/hwp/?fileId=<id>` 진입 → 제목 띠 표 hwpx 업로드 → 1 페이지 row 1 셀 텍스트 교체 → row 0·row 2 그라데이션 색띠 두께·아래 표 서식 시각 비교. §6 4 항목 시각 자료 확보.

시각 자료에서 *부분 해결* 또는 *미해결* 자리 확인 시 spec §3.5 (measured_tables 캐시 stale) 자료 자료 진입 — 후속 cycle.

## 후속 자리 보류

- spec §3.4 (recompose_for_cell_width 자리) — Stage 1 자료에서 *기존 line_segs 살아 있으면 재측정 스킵* 자료 확인. 현재 자료 시점에서 직접 트리거 자리 아님.
- spec §3.5 (measured_tables 캐시 무효화) — row 0·row 2 직접 폭증 자료가 *측정 캐시* 자리에서 비롯될 가능성. 사용자 시각 자료에서 미해결 자리 확인 시 진입.

## 사용자 메모리 정합

- [feedback_rhwp_source_check_before_rdocx_work] — `src/model/paragraph.rs:135-140` 의 `CharShapeRef` 직접 확인, spec 가설서 `style_id` 표기 → `char_shape_id` 정정.
- [feedback_substage_visual_verification_mandatory] — 시각 검증 자리 사용자 시각 자료 가능 상태로 cycle 종결.
- [feedback_cycle_end_verification_pattern] — server·studio 자료 자료 자료 자료 자료 자료 자료 (변경 없이 유지) + 시각 검증 가능 상태로 자연 종결.
- [project_rhwp_replace_runs_vpos_reset_fix] — 동형 cycle 패턴 (apply_char_format `line_segs.clear() + reflow → vpos=0 jump`) 자리. 이번 cycle은 *make_line_seg 폴백 자료* 자리의 *부분 해결*. row 0·row 2 직접 자리는 spec §3.5 후속 cycle 진입 가능.
