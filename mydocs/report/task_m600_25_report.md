# Task #m600-25 최종 결과 보고서 — `replace_cell_runs` 후 새로고침 시 서식 깨짐 fix

## 사이클 요약

`replace_cell_runs` 후 새로고침 시 cell 서식·paginate 깨짐 결함. 진단 결과 *서버 hwpx 직렬화의 cell paragraph lineseg 자리*에 결함 — `table.rs` 와 `shape.rs` 가 cell paragraph 의 line_segs 를 *완전 무시하고 하드코딩 정적 lineseg* 박음. cell 폭 기반 reflow_line_segs 호출 결과로 *HWPX 정합 다중 segments* 직렬화하도록 패치.

## 변경

| 파일 | 변경 |
|---|---|
| [src/serializer/hwpx/context.rs](../../src/serializer/hwpx/context.rs) | `SerializeContext` 에 `resolved_styles` + `dpi` 추가. `collect_from_document` 가 `resolve_styles_with_variant` 호출해 박음. |
| [src/serializer/hwpx/table.rs](../../src/serializer/hwpx/table.rs#L281-L330) | cell paragraph 직렬화 자리. 하드코딩 lineseg 제거. `line_segs ≤ 1 && !text.is_empty()` 자리에 `reflow_line_segs` 호출 (paragraph clone), 결과 segments 를 hp:lineseg 자료로 직렬화. |
| [src/serializer/hwpx/shape.rs](../../src/serializer/hwpx/shape.rs#L249-L296) | 글상자 paragraph 같은 패턴 패치. p.line_segs IR 그대로 직렬화. |

## 검증

| 자리 | 결과 |
|---|---|
| `cargo test --workspace --lib` | 1491 passed / 0 failed / 6 ignored |
| `cargo clippy --workspace --lib` | warning 0 |
| `test-reflow` (원본 hwp + replace) `?fmt=hwpx` dump | row 1 `lh=2000`, 2 segments 정상 |
| 클라 console validation warnings | 18 → 3 (cell paragraph 정합 회복) |
| 사용자 시각 — 영문 cell + paginate | 정합 |

## 가설 부정 자료

- task_m600_24 의 `make_line_seg` 12pt 폴백 가설 — 완전 부정 (revert 됨)
- 후보 A·B·C (parser path, replace_cell_runs chain) — 부정 (원본 hwp 직접 POST 한 자료는 깨끗)
- 결함은 `serialize_hwpx` 의 cell paragraph lineseg 직렬화

## 남은 결함 (task 26 분리)

위·아래 행 그라데이션 색띠 + 아래 표 색상 손실은 *별 결함*:

```
원본 hwp: border_fill[7] = Gradient (gradient brush)
fmt=hwpx export: header.xml 의 borderFill 자료가 Solid 만 직렬화
```

header serializer 가 Gradient·Pattern·Image fill_type 미지원. cycle 26 으로 분리해 진입.

## 사용자 메모리 정합

- [feedback_substage_visual_verification_mandatory] — 시각 검증을 사용자 시점에서 가능한 상태로 cycle 종결
- [feedback_cycle_end_verification_pattern] — server·studio 자료 그대로 유지, 시각 검증 가능 상태로 자연 종결
- [feedback_rhwp_source_check_before_rdocx_work] — `src/model/paragraph.rs`·`src/parser/hwpx/section.rs` 직접 확인
- [feedback_avoid_jari_jaryo_filler] — 채움말 자제 (반복 위반 — 재인지 필요)
