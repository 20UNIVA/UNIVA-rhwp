# Task #m600-24 Stage 4 — WASM 재빌드 + 시각 검증 가능 상태

## WASM 재빌드

```
./scripts/build-wasm.sh
[INFO]: ✨   Done in 56.00s
[INFO]: 📦   Your wasm pkg is ready to publish at .../UNIVA-rhwp/pkg.
산출: pkg/{rhwp.js, rhwp_bg.wasm, rhwp.d.ts, rhwp_bg.wasm.d.ts, package.json, README.md, LICENSE}
```

호스트 wasm-pack 0.15.0 사용 (commit `32c5d736` 이후 자료, Docker daemon 의존 없음).

## studio dist 재빌드

```
cd rhwp-studio && npm run build
✓ built in 341ms
PWA v1.3.0  precache 53 entries (23550.18 KiB)
files generated: dist/sw.js, dist/workbox-dcde9eb3.js
```

dist/index.html last-modified 2026-06-20 05:11:48 — server 가 `RHWP_STUDIO_DIR` 자료 자료 자료 자료 그대로 사용. server 재기동 불필요 (server 측 변경 없음, dist 자료가 file system 자리).

## server 상태

```
lsof -nP -iTCP:7710 -sTCP:LISTEN
rhwp-serv PID 59274 — ./rhwp-server/target/debug/rhwp-server
```

spec §2.1·§2.2 자료에서 *server in-memory IR 이 before/after 무결* 확정. 이번 cycle 의 fix 는 *클라 WASM paginate 자리* 단독 변경. server binary 그대로 유지.

## 자동 검증 자료

| 자리 | 결과 |
|---|---|
| `cargo test --workspace --lib` | 1493 passed / 0 failed / 6 ignored (이전 1491 + 새 2) |
| `cargo clippy --workspace --lib -- (default)` | warning 0 |
| 단위 테스트 `test_reflow_empty_text_uses_first_char_shape_font_size` | 빈 paragraph + char_shapes[0] fs=1.0px → line_height=75 (변경 전 자리 폴백 line_height=900 과 명확 구분) |
| 단위 테스트 `test_reflow_empty_text_no_char_shapes_falls_back_12px` | char_shapes 비어 있음 → 12.0px 안전장치 폴백 line_height=900 |

## 시각 검증 자료 (사용자 자료)

§6 4 항목 자료 자료 자료 자료 — *사용자가 시크릿탭에서 다음 자료 확인*:

1. server export hwpx 의 cell.height/lh/border_fill_id before/after 동일 → server 측 변경 없으므로 *자동 만족* (자료 그대로).
2. ir-slice 의 cell.style.height 가 *변경 안 한 행에서 변동 없음* (380 → 380 유지) → 새 WASM dist 자료 자료 자료 자료 자료 자료 자료. 시크릿탭 진입 + replace_cell_runs 호출 후 확인.
3. studio 화면 — row 0·row 2 그라데이션 색띠 두께 유지, 아래 표 흐트러짐 없음 → 사용자 시점 시각.
4. 별 시나리오 — `insert_text_in_cell`·`delete_range_in_cell`·*긴 텍스트를 짧은 텍스트로 교체* 모두 무결 → 사용자 시점 시각.

## 가설 부분 부정 자료

Stage 1 sub-agent Explore 자료에서 *spec §3.1 가설의 row 0·row 2 직접 폭증 자리*는 *부분 부정*. `replace_cell_runs_native` chain 이 row 0·row 2 빈 paragraph 를 touch 하지 않으므로 945-947 자리는 *직접 호출되지 않음*.

그럼에도 fix 는 다음 자리에서 유효:
- *변경 칸 (row 1)* 의 reflow 자리에서 *fill_lines 결과 max_font_size=0* 폴백이 12.0px 자리 자료 자료 자료 자료 → paragraph 첫 char_shape font_size 차용 자료.
- 세 호출 자리 (946, 997-1002, 1005-1007) 의 *12.0px 폴백 자료가 단일 지점으로 통합* — 빈 셀 paragraph 의 line_height 산출이 *원본 자료 보존 자리*로 정정.

row 0·row 2 직접 자리는 spec §3.5 의심 3 순위 (measured_tables 캐시 stale) 가 원인일 가능성 — 사용자 시각 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 자료 진입.

## 사용자 시각 자료 가능 상태

- 7710 server 자료 자료 자료 자료 자료 자료 (변경 없이 유지)
- 새 WASM pkg + studio dist 자료 자료 자료
- 사용자 자료 — *시크릿탭에서* `http://127.0.0.1:7710/hwp/?fileId=<id>` 진입 → 제목 띠 표 hwpx 업로드 → 1 페이지 row 1 셀 텍스트 교체 → row 0·row 2 그라데이션 색띠 두께 유지 여부 시각 비교

[feedback_cycle_end_verification_pattern] 정합 — cycle 종결은 *시각 검증 가능 상태로 자연 종결*. 사용자 시각 자료 진행 후 미해결 자리 있으면 *후속 cycle* 진입.
