# Task #m600-27 최종 결과 보고서 — hwpx export 의 paragraph 0.0 ↔ 표 cell 첫 lineseg 덮어쓰기 fix

## 사이클 요약

cycle 26 종결 시 *3행 1열 그라데이션 표의 row 0 / row 2 가 row 1 만큼 두꺼워지는* 결함 잔존. cycle 26 spec md 의 가설 — `paginate()` 의 cell[0] 부수효과 — 은 정정. 실제 결함 자리는 *hwpx 직렬화 자체* — paragraph 0.0 의 first_t 안에 박힌 표 cell 의 `<hp:linesegarray>` 가 `replace_first_linesegs` 의 첫 매칭으로 잡혀 cell[0] 의 vertsize 가 paragraph 0.0 의 IR lh 자료로 덮어쓰임.

## 결함 자리

[src/serializer/hwpx/section.rs:65-73](../../src/serializer/hwpx/section.rs#L65-L73) 의 호출 순서 결함.

```rust
// 결함 자리 (cycle 27 fix 전)
let mut out = EMPTY_SECTION_XML.replacen(TEXT_SLOT, &first_t, 1);
out = replace_first_linesegs(&out, &first_linesegs);
```

흐름:
1. `render_paragraph_parts(first_para)` 의 first_t 안에 *표 cell paragraph 의 `<hp:linesegarray>...</hp:linesegarray>`* 가 박힘 (`render_run_content` → `render_control_slot` → `table::write_table` → `write_sub_list` 사슬).
2. `replacen(TEXT_SLOT, &first_t, 1)` — `<hp:t/>` 자리 (template 의 paragraph 0.0 linesegarray 보다 *앞쪽*) 에 first_t 박힘. 결과 — cell 의 linesegarray 가 paragraph 0.0 의 template linesegarray 보다 *앞* 에 위치.
3. `replace_first_linesegs(&out, &first_linesegs)` — `xml.find("<hp:linesegarray>")` 의 첫 매칭이 *first_t 안의 첫 cell linesegarray*. 그 자리의 내용이 first_linesegs (paragraph 0.0 의 IR lh=3603) 로 교체됨.
4. paragraph 0.0 의 template linesegarray 는 그대로 — 정적 vertsize=1000 (`push_lineseg_static` 출력값) 잔존.

## 변경

### `src/serializer/hwpx/section.rs`

`replace_first_linesegs` 호출을 `TEXT_SLOT` 교체 *앞* 으로 옮김. first_t 가 박히기 전 상태에서는 template 안 linesegarray 가 유일하므로 `find` 가 paragraph 0.0 의 자리를 정확히 매칭한다.

```rust
let mut out = replace_first_linesegs(EMPTY_SECTION_XML, &first_linesegs);
out = out.replacen(TEXT_SLOT, &first_t, 1);
```

## 검증

### 코드 회귀

| 자리 | 결과 |
|---|---|
| `cargo test --workspace --lib` | 1491 passed / 0 failed |

### 자동 e2e — sim-task27-test 의 hwpx export section0.xml

| lineseg 자리 | 패치 전 vertsize | 패치 후 vertsize | IR 값 |
|---|---|---|---|
| 첫 (= 표 cell[0]) | `3603` (덮어쓰임) | `100` | 100 |
| 두 번째 (= cell[1]) | `2000` | `2000` | 2000 |
| 세 번째 (= cell[2]) | `100` | `100` | 100 |
| 네 번째 (= paragraph 0.0) | `1000` (template 정적) | `3603` | 3603 |

cell paragraph 의 vertsize 가 IR 값 그대로 박힘. paragraph 0.0 자체의 vertsize 도 IR 값 그대로 박힘.

### 사용자 시각 검증

3행 1열 그라데이션 표의 row 0 / row 2 가 *얇음* 으로 회복. row 1 만 두꺼움. 사용자 OK 확인.

## cycle 26 spec md 가설의 정정

cycle 26 종결 시점의 가설 — *`paginate()` 가 cell[0] paragraph.line_segs[0].line_height 자료를 mutate* — 은 사실과 다름. IR 자체는 paginate() 후에도 mutate 없이 원본 값 (cell[0]=100·cell[2]=100·paragraph 0.0=3603) 보존. 결함은 *export 직렬화 path 안* 의 호출 순서.

cycle 26 의 진단이 paginate() 후 IR 을 검사할 때 *export 직전 자료* 가 아닌 *export 결과 자료* 를 직접 봤다면 원인 자리를 좁히기 쉬웠음. 다음 cycle 에서 *export 직전 IR + export 결과 hwpx* 양쪽을 비교하는 진단 도구 박으면 동등 결함을 빠르게 catch 가능.
