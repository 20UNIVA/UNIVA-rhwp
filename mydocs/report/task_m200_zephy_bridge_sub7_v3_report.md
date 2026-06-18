# Sub-7 v3 — 브라우저 wasm 의 native fn 이 광고 키 직접 받게 alias

## 배경

Sub-7 / Sub-7 v2 push 후 *서버 IR 응답은 정상* 이지만 *브라우저 캔버스 색이 안 바뀜*. 사용자 VM 사고 재현:

```
보냄: set-cell-style {"bgcolor":"#FFB6C1"}
서버 응답: changed:true + after.cell.style.bgcolor:"#FFB6C1"   ← 서버 OK
브라우저: 셀 색 그대로                                          ← 화면 미반영
```

원인 — WS broadcast 페이로드는 *원본 EditOperation* 형태 (광고 키 `bgcolor` 포함). main.ts onServerEvent 가 `wasm.setCellProperties(op.style)` 호출 → wasm 의 `set_cell_properties_native` 는 *fillColor 만* 받음 → bgcolor silent drop → 캔버스 변화 0.

같은 메커니즘으로 *모든 셀 style / run style / paragraph style 의 광고 키 브라우저 적용 실패*:

| main.ts 분기 | wasm 호출 | 영향 |
|---|---|---|
| set_cell_style | setCellProperties(op.style) | bgcolor / border / padding-* 광고 키 silent drop |
| replace_runs / replace_cell_runs | replaceRuns/replaceCellRuns(runs JSON) | runs 안 PartialRunStyle 의 color/highlight/font-size 광고 키 silent drop |
| insert_text_in_cell | applyCharFormatInCell(op.style) | 동일 |
| set_paragraph_style | applyParaFormat(op.style) | align/line-height 광고 키 silent drop |

## 해결 — native fn 진입부에 광고 키 alias

main.ts 변경 *0*. *native fn 한 쪽* 만 광고 키 정규화 + alias 처리 → 서버·브라우저 양쪽 통합.

### 1. table_ops.rs — set_cell_properties_native

신규 `normalize_cell_style_keys(json: &str) -> String` — JSON 진입 시 광고 키를 native 키로 *재작성*:

```rust
// bgcolor → fillType=solid + fillColor
// border (nested) → borderLeft/Right/Top/Bottom (all 우선, 개별 override)
// padding-left → paddingLeft (kebab → camel)
// vertical-align → verticalAlign (kebab → camel)
// verticalAlign 값 "top"|"middle"|"center"|"bottom" → u8 (0/1/1/2)
```

`set_cell_properties_native` 본문 첫 줄:
```rust
let json_owned = normalize_cell_style_keys(json);
let json = json_owned.as_str();
```

기존 native 키 (fillColor / borderLeft 등) 도 그대로 통과. 광고 + native 동시 입력 시 *native 우선* (entry().or_insert 패턴).

### 2. helpers.rs — parse_char_shape_mods / parse_para_shape_mods

광고 키 alias 추가:

| 광고 키 | native 키 | 함수 |
|---|---|---|
| color | textColor | parse_char_shape_mods |
| highlight | shadeColor | parse_char_shape_mods |
| font-size / font_size | fontSize | parse_char_shape_mods |
| align | alignment | parse_para_shape_mods |
| line-height / line_height / lineHeight | lineSpacing | parse_para_shape_mods |

광고 키 처리 → native 키 처리 순서로 작성 — 둘 다 들어오면 native 가 마지막 set 으로 이김.

### 3. formatting.rs — font-name → fontId 변환

신규 `inject_font_id_if_present(core, json) -> String` — JSON 에 `fontName` / `font-name` / `font_name` 키 있으면 `find_or_create_font_id_native` 로 변환해 `fontId` 키로 재작성. 폰트 키 없으면 no-op. native `fontId` 있으면 native 우선.

`apply_char_format_native`, `apply_char_format_native_with_base`, `apply_char_format_in_cell_native`, `apply_char_format_in_cell_native_with_base` *4 함수 진입부* 모두 호출.

## 검증

### 단위 (15 신규)

- `table_ops.rs` 6건 — bgcolor → fillType+fillColor, border.all → 4 방향, kebab padding, native passthrough, native wins, *Live e2e* (Table 생성 → bgcolor 단독 호출 → SolidFill background_color 0x000000FF 검증)
- `helpers.rs` 6건 — color/highlight/font-size kebab/font-size snake/align right/line-height kebab
- `formatting.rs` 3건 — fontName converts/no_op/native_wins

### 빌드 + 회귀

- `cargo test --lib` rhwp: **1485 pass / 0 fail / 6 ignored** (신규 15 + Sub-7 v2 의 1470)
- `cargo test` rhwp-server: **78 pass / 0 fail**
- `cargo build --release` ok (rhwp + rhwp-server)
- `cargo clippy --tests --no-deps` 신규 경고 0
- `npm run build` (rhwp-studio) ok

### e2e (32 시나리오 회귀)

- sub7-style-round-trip: **15/15 PASS**
- sub7v2-cell-style-round-trip: **5/5 PASS**
- sub4-patch-diff: **9/9 PASS**
- sub6-ws-echo-skip: **3/3 PASS**

### Live curl (로컬 7710)

- `set_cell_style + bgcolor:"#FFB6C1"` → `changed:true` + `after.bgcolor:"#FFB6C1"` (서버 회귀 0)
- 단위 e2e `set_cell_properties_native_with_bgcolor_alias_applies_fill` 통과 = wasm 도 같은 native fn 사용하므로 *브라우저 측에서도 bgcolor 적용 보장*

## 효과

1. *VM 사용자 사고 해소* — `set-cell-style bgcolor` 가 *브라우저 캔버스에도 실제로 반영*
2. *진실 단일* — main.ts 변경 0, native fn (Rust) 한 쪽만 alias → 서버·클라 코드 분기 동기화 부담 0
3. *호환성* — 기존 native 키 그대로 처리, 광고+native 동시 시 native 우선
4. *광고 카탈로그 완성* — SKILL.md 의 모든 광고 키가 실제로 *wasm 레벨에서 작동*

## 트레이드오프

- `normalize_cell_style_keys` 가 JSON parse/serialize 1회 추가 — 셀 호출당 microsecond 비용 (무시 가능)
- `inject_font_id_if_present` 가 fontName 있을 때만 fontId 변환 — 폰트 자동 등록 부수효과 (find_or_create_font_id_native 의 의도된 동작)

## 다음 (사용자)

VM 재배포 → 동일 시나리오 (`set-cell-style {"bgcolor":"#FFB6C1"}`) 다시 → *브라우저 캔버스 색이 실제로 바뀌는지* 확인. SW 캐시 강제 갱신 (DevTools → Application → Service Workers → Unregister → Hard Reload) 필요.
