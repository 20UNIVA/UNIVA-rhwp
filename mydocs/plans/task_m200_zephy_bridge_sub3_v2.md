# Task #zephy-bridge Sub-3 v2 — IR Compact 응답 토큰 절감 + 페이지 단위 슬라이스

작성일 2026-06-07.

## 목표 한 줄

Sub-3 의 compact 응답이 *실문서에서 249KB (≈62K 토큰)* 까지 비대해진 문제를 해결한다 — *셀 평탄 entry 중복 제거* + *페이지 단위 슬라이스* + *구조 키 omit* 세 방안을 한 통합 작업으로 적용해 응답을 *5-7배 축소* 한다.

## 1. 진단 — 왜 250KB 인가

현재 시점 (`http://127.0.0.1:7710/sessions/sim-1780843626/ir-slice?mode=compact`) 응답 측정:

| 항목 | 건수 | 바이트 | 비중 |
|---|---|---|---|
| 본문 text paragraph | 20 | 6.9 KB | 2.8% |
| 표 본체 (nested cells 포함) | 10 | 171 KB | 68% |
| 셀 평탄 entry (paragraphs[] 안 cell_locator) | 351 | 117 KB | 47% |
| defaults + doc_meta + braces | — | ~6 KB | 2.4% |
| **총합** | 381 | **249 KB** | 100% |

(표 nested 와 셀 평탄 의 합이 100% 를 넘는 이유 — *같은 셀 내용이 두 곳에 모두 들어감*.)

### 1.1 비대 원인 3 가지

1. **셀 중복**. `build_paragraph` ([server/src/ir_compact.rs](../../server/src/ir_compact.rs)) 가 `IrParagraph::Table(table)` 을 push 한 *직후* `flatten_cell_paragraphs(table)` 로 *같은 셀 paragraph* 를 *paragraphs[] 평탄* 에 또 push. 모델 편의를 위한 의도였으나 *내용이 두 배* 로 들어감.
2. **페이지 미분할**. 현재 endpoint 는 `para_start`/`para_end` 만 받는다. *paragraph 번호* 단위 — 모델 입장에서 *어디부터 어디까지가 한 페이지* 인지 모름. 큰 문서 (수십 페이지) 의 *현재 보고 있는 페이지* 만 요청할 방법이 없다.
3. **구조 키 비효율**. `id` (각 paragraph 의 디버그 라벨 — 모델이 사용 안 함), `sec` (한 응답 안 항상 같음), `type:"text"` (table 외엔 기본값), `char_offset:0` (run 의 첫 번째는 항상 0), 빈 `style:{}` / `runs:[{...,text:""}]` 등 *무용 키* 가 응답 부피의 ~15-20% 차지.

### 1.2 한 셀 평탄 entry 의 실제 모양 (현재)

```json
{
  "cell_locator": {"cell_para": 0, "col": 0, "row": 0, "table_para": 0},
  "id": "p_0_0_c2_0_0",
  "para": -1,
  "runs": [{"char_offset": 0, "style": {...}, "text": "A3. 정보화"}],
  "sec": 0,
  "style": {"align": "center"},
  "type": "text"
}
```

7 키. *같은 셀 paragraph* 가 `table.cells[0].paragraphs[0]` 안에도 *동일하게* 존재. 즉 *모델은 둘 중 하나만 보면 됨*.

## 2. 압축 방안 — A + B + C 단발 적용

### 2.1 방안 A — 셀 평탄 entry 제거 (예상 ~47% 절감)

`build_paragraph` 에서 `flatten_cell_paragraphs(table)` 호출 *제거*. table 본체 안의 nested `paragraphs[]` 가 *유일한* 셀 내용 표현이 된다.

**모델 영향**:
- nested 의 `table.cells[i].paragraphs[j]` 가 `cell_locator` 를 *그대로 포함* (Phase 1 부터 이미 들어있음 — `build_cell_paragraph` 가 cell_locator 채움). 즉 모델은 nested 만 순회해도 *셀 내부 명령에 필요한 4 좌표 (table_para/row/col/cell_para) 전부 확보*.
- *모델 가이드 (init.md)* 의 §4 의 "셀 내부 문단" 분류 — `cell_locator 있음` 으로만 정의. nested 안의 entry 도 *같은 분류 통과*. 가이드 본문 정정 *불필요* (또는 §4 의 예제만 갱신).

### 2.2 방안 B — 페이지 단위 슬라이스 (예상 1/N 절감, N = 총 페이지 수)

`IrSliceQuery` 에 *옵셔널 `page: u32`* 추가. 페이지 번호 (0-based) 를 받으면 paginator 결과로 *(sec, para_start, para_end)* 매핑.

**알고리즘**:
1. `DocumentCore::pagination` 이 `pub(crate)` — *최소 accessor* 추가 (Sub-3 의 `styles()` 와 동일 패턴): `pub fn pagination(&self) -> &[PaginationResult]`. 1줄.
2. `paginate()` 결과의 각 `PaginationResult` 가 `pages: Vec<Page>` 를 가짐. 각 `Page` 는 `items: Vec<PageItem>` 보유. `PageItem` 은 `FullParagraph { pi }` / `Table { pi }` 같은 분류 — *pi (paragraph index)* 를 추출.
3. `(page_num: u32) → (sec: usize, para_start: usize, para_end: usize)` — page 의 *최소 pi* 와 *최대 pi + 1*.
4. `page` 파라미터가 들어오면 paginator 매핑 결과를 *para_start/para_end 로 override*.

**Query 정책** (충돌 시):
- `page` 와 `para_start/para_end` 가 *둘 다 지정* → `page` 우선 (옛 호환을 위해 둘 다 허용하되 명시적 page 가 이김).
- `page` 가 범위 초과 → 400 Bad Request (`AppError::bad_request`).
- 다중 섹션 — `page_num` 은 *문서 전체 페이지* (섹션 가로지름). page 의 *section* 도 응답 `doc_meta.anchor.sec` 으로 명시.

**노트북 라우터 갱신**: cell 3 의 `_handle_get_ir_slice` 가 `page` payload key 도 query 로 변환 (한 줄 추가).

### 2.3 방안 C — 구조 키 omit (예상 ~15-20% 절감)

방안 A 적용 후의 nested-only 응답에서 *무용 키 제거*:

| 키 | 제거 조건 | 효과 |
|---|---|---|
| `id` | 항상 omit | paragraph 당 ~15 bytes, 380 paragraph → ~5.5KB |
| `sec` | 한 응답 안 sec 이 *모두 같으면* paragraph 별 omit. doc_meta.anchor.sec 한 번이면 충분 | paragraph 당 ~8 bytes, ~3KB |
| `type` | `"text"` 가 기본 — paragraph 의 type 키 자체 omit. `"table"` 만 명시 | paragraph 당 ~14 bytes, ~5KB |
| `char_offset` | 첫 run 의 `char_offset:0` 은 default — omit | run 당 ~17 bytes |
| 빈 `style:{}` | empty object omit | paragraph 당 ~12 bytes |
| 빈 `runs:[{...,text:""}]` | 단일 빈 run 이면 `text:""` 직속 (이미 Phase 5 의 *단일 run text 직속* 규칙이 처리. *길이 0 도 같은 규칙* 적용 — 정정 한 줄) | ~30 bytes/paragraph |

*모델 가이드 (init.md) 정정*: §3 에 *생략된 type 키는 "text" 로 본다* + *sec 키 생략 시 doc_meta.anchor.sec 를 본다* 추가 한 줄.

### 2.4 미적용 후보 (Sub-3 v3 으로 미룸)

- D. defaults 박스의 *실제로 사용된 키만* 노출 — 절감 작음 (3-5%), 모델 측 변환 비용
- E. 표 안 동일 style 셀 그룹화 / 빈 셀 paragraph omit — 절감 작음 (~10%), 알고리즘 복잡도 큼

## 3. 구현 위치 + 파일 청사진

### 3.1 신규/수정 파일

| 파일 | 변경 |
|---|---|
| `server/src/ir_compact.rs` | (1) `build_paragraph` 에서 `flatten_cell_paragraphs` 호출 제거 (방안 A). (2) `compact_text`·`compact_cell` 에 키 omit 규칙 6 종 추가 (방안 C). |
| `server/src/main.rs` | `IrSliceQuery` 에 `page: Option<u32>` 추가. `ir_slice_handler` 에 page → (sec, para_start, para_end) 매핑 분기. |
| `src/document_core/mod.rs` | `pub fn pagination(&self) -> &[PaginationResult]` accessor 1 줄 (rhwp 본체 — Sub-3 의 `styles()` 와 동일 패턴, 최소). |
| `hwp_sub_agent_simulation_ssr.ipynb` cell 3 | `_handle_get_ir_slice` 에 `page` payload key → query 변환 한 줄 추가. |
| `26ZEPHY-skills/.../hwp-doc-edit/references/init.md` | §1 의 payload 키 표에 `page` 추가, §3 에 *생략된 type/sec 해석* 한 줄, §4 의 셀 내부 분류 예시를 *nested 만* 으로 정정. |
| `rhwp-studio/e2e/sub3-ir-compact.test.mjs` | 셀 평탄 entry 검증 *제거*, nested cell_locator 검증으로 *교체*. page query 시나리오 추가. |

### 3.2 작업 디렉토리·브랜치

- 작업 디렉토리: `/Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp`
- 브랜치: `local/task_m200_zephy_bridge` (Sub-3 그대로 연속)

## 4. 구현 단계 — 4 phase / 11 task

### Phase 1 — 셀 평탄 entry 제거 (방안 A)

#### Task 1.1 — `build_paragraph` 정정

**Files:** `server/src/ir_compact.rs` (build_paragraph 함수)

- [ ] **Step 1**: `flatten_cell_paragraphs` 호출 제거. `build_paragraph` 가 표 발견 시 *table 본체만* push:
  ```rust
  for (ci, ctrl) in p.controls.iter().enumerate() {
      if matches!(ctrl, rhwp::model::control::Control::Table(_)) {
          if let Some(table) = build_table_paragraph(core, sec, para, ci) {
              return vec![IrParagraph::Table(table)];
          }
      }
  }
  ```

- [ ] **Step 2**: `flatten_cell_paragraphs` 함수 자체 제거 (dead code).

- [ ] **Step 3**: 영향 받는 unit test 정정 — `build_ir_slice_text_and_table` 가 *셀 평탄 entry 존재* 를 assert 하면 정정 (nested 만 검사하도록).

- [ ] **Step 4**: 빌드·테스트
  ```bash
  cd server && $HOME/.cargo/bin/cargo test ir_compact::tests::
  ```

- [ ] **Step 5**: commit
  ```bash
  git add server/src/ir_compact.rs
  git commit -m "Task #zephy-bridge Sub-3 v2: build_paragraph 의 셀 평탄 entry 제거 — nested 만 유지"
  ```

#### Task 1.2 — e2e 갱신

**Files:** `rhwp-studio/e2e/sub3-ir-compact.test.mjs`

- [ ] **Step 1**: 검증 5 (`compact.paragraphs.find(p => p.cell_locator)`) 를 *nested 안 cell_locator* 검증으로 교체:
  ```javascript
  // 5. 셀 (0,0) 안의 paragraph 가 cell_locator 4 좌표 보유
  const cellPara0 = table.cells.find(c => c.row === 0 && c.col === 0).paragraphs[0];
  assert.ok(cellPara0.cell_locator, 'nested cell_locator 누락');
  assert.equal(cellPara0.cell_locator.table_para, table.para);
  ```

- [ ] **Step 2**: 서버 가동 + 실행
  ```bash
  ./rhwp-studio/e2e/sub2-server.sh restart
  node rhwp-studio/e2e/sub3-ir-compact.test.mjs
  ```

- [ ] **Step 3**: 실문서 크기 비교 — fix 전 249KB → fix 후 측정. *최소 40% 감소* 기대.

- [ ] **Step 4**: commit
  ```bash
  git add rhwp-studio/e2e/sub3-ir-compact.test.mjs
  git commit -m "Task #zephy-bridge Sub-3 v2: e2e 검증을 nested cell_locator 로 교체"
  ```

### Phase 2 — 페이지 단위 슬라이스 (방안 B)

#### Task 2.1 — 본체 pagination accessor

**Files:** `src/document_core/mod.rs`

- [ ] **Step 1**: `impl DocumentCore` 의 `styles()` 옆에 한 줄 추가
  ```rust
  /// IR slice 빌더용 — Sub-3 v2 의 페이지 단위 슬라이스가 사용.
  pub fn pagination(&self) -> &[PaginationResult] {
      &self.pagination
  }
  ```

- [ ] **Step 2**: 본체 빌드 회귀 0 확인
  ```bash
  $HOME/.cargo/bin/cargo build --lib
  ```

- [ ] **Step 3**: commit
  ```bash
  git add src/document_core/mod.rs
  git commit -m "Task #zephy-bridge Sub-3 v2 [accessor]: DocumentCore::pagination() pub accessor"
  ```

#### Task 2.2 — 페이지 → paragraph 범위 매핑 함수

**Files:** `server/src/ir_compact.rs`

- [ ] **Step 1**: `BuildOptions` 에 `page: Option<u32>` 추가 + 페이지 매핑 함수:
  ```rust
  #[derive(Debug, Clone, Default)]
  pub struct BuildOptions {
      pub sec: usize,
      pub para_start: usize,
      pub para_end: Option<usize>,
      pub edit_session_id: Option<String>,
      pub page: Option<u32>,
  }

  /// 페이지 번호 → (sec, para_start, para_end). 페이지가 범위 외면 None.
  /// page_num 은 *문서 전체 페이지* (섹션 가로지름) — paginate() 결과 그대로.
  pub fn page_to_para_range(
      core: &DocumentCore,
      page_num: u32,
  ) -> Option<(usize, usize, usize)> {
      let mut global_idx: u32 = 0;
      for (sec_idx, pr) in core.pagination().iter().enumerate() {
          for page in &pr.pages {
              if global_idx == page_num {
                  // page.items 의 pi 의 min/max 추출
                  let pi_min = page.items.iter().filter_map(|it| it.paragraph_idx()).min();
                  let pi_max = page.items.iter().filter_map(|it| it.paragraph_idx()).max();
                  return match (pi_min, pi_max) {
                      (Some(start), Some(end)) => Some((sec_idx, start, end + 1)),
                      _ => None,
                  };
              }
              global_idx += 1;
          }
      }
      None
  }
  ```

  *`PageItem::paragraph_idx()` 가 존재하지 않을 가능성* — `PageItem` 의 정확한 variants (`FullParagraph { pi, .. }` 등) 를 `src/renderer/page_layout.rs` 또는 `src/renderer/pagination.rs` 에서 확인 후 match arm 으로 직접 추출.

- [ ] **Step 2**: unit test (mock 또는 실 paginator 결과)
  ```rust
  #[test]
  fn page_to_para_range_first_page() {
      // 빈 hwpx 로드 → 1 페이지 → (0, 0, 1) 예상
      ...
  }
  ```

- [ ] **Step 3**: build_ir_slice 안에서 *page 가 지정되면 para_start/para_end 를 덮어씀*:
  ```rust
  let (sec, start, end) = if let Some(p) = opts.page {
      page_to_para_range(core, p).unwrap_or((opts.sec, opts.para_start, opts.para_end.unwrap_or_default()))
  } else {
      let sec = opts.sec;
      let total = core.document().sections[sec].paragraphs.len();
      let start = opts.para_start.min(total);
      let end = opts.para_end.unwrap_or(total).min(total);
      (sec, start, end)
  };
  ```

- [ ] **Step 4**: commit
  ```bash
  git add server/src/ir_compact.rs
  git commit -m "Task #zephy-bridge Sub-3 v2: page_to_para_range + BuildOptions::page 추가"
  ```

#### Task 2.3 — endpoint 페이지 query 분기

**Files:** `server/src/main.rs`

- [ ] **Step 1**: `IrSliceQuery` 에 `page: Option<u32>` 추가
  ```rust
  #[derive(Deserialize)]
  struct IrSliceQuery {
      #[serde(default)] sec: Option<usize>,
      #[serde(default)] para_start: Option<usize>,
      #[serde(default)] para_end: Option<usize>,
      #[serde(default)] page: Option<u32>,  // 신규
      #[serde(default = "default_ir_slice_mode")] mode: String,
  }
  ```

- [ ] **Step 2**: compact 분기의 `BuildOptions` 생성에 page 전달
  ```rust
  let opts = ir_compact::BuildOptions {
      sec, para_start, para_end: Some(para_end),
      edit_session_id: Some(format!("cli_{}", file_id)),
      page: q.page,
  };
  ```

  page 가 지정되면 build_ir_slice 안에서 sec/para_start/para_end 가 자동 덮어씀.

- [ ] **Step 3**: top-level 호환 필드 (section/para_start/para_end/mode) 도 *실제 사용된 값* 으로 채움. anchor.sec/para_start/para_end 와 일관.

- [ ] **Step 4**: 빌드 + 가동 + 수동 smoke
  ```bash
  cd server && $HOME/.cargo/bin/cargo build
  cd .. && ./rhwp-studio/e2e/sub2-server.sh restart
  curl -s "http://127.0.0.1:7710/sessions/sim-1780843626/ir-slice?mode=compact&page=0" | wc -c
  ```
  → 페이지 0 만의 응답 크기 측정. 큰 문서일수록 절감 폭.

- [ ] **Step 5**: commit
  ```bash
  git add server/src/main.rs
  git commit -m "Task #zephy-bridge Sub-3 v2: ir_slice_handler 의 page query 분기"
  ```

#### Task 2.4 — e2e 페이지 시나리오

**Files:** `rhwp-studio/e2e/sub3-ir-compact.test.mjs`

- [ ] **Step 1**: 표 + 본문 다수 paragraph 시나리오 + `?page=0` 호출 → 응답 paragraph 수 확인 (전체 < total).

- [ ] **Step 2**: 실행
  ```bash
  node rhwp-studio/e2e/sub3-ir-compact.test.mjs
  ```

- [ ] **Step 3**: commit
  ```bash
  git add rhwp-studio/e2e/sub3-ir-compact.test.mjs
  git commit -m "Task #zephy-bridge Sub-3 v2: e2e — page query 시나리오"
  ```

### Phase 3 — 구조 키 omit (방안 C)

#### Task 3.1 — `compact_text` 의 키 omit 규칙

**Files:** `server/src/ir_compact.rs::compact_text`

- [ ] **Step 1**: 다음 키 omit
  ```rust
  // 변경 후 compact_text 의 본문:
  fn compact_text(p: &IrTextParagraph, defaults: &DocDefaults, omit_sec: bool) -> serde_json::Value {
      let runs: Vec<serde_json::Value> = p.runs.iter().enumerate().map(|(i, r)| {
          compact_run(r, defaults, i == 0)  // i==0 이면 char_offset 생략 후보
      }).collect();
      let para_style = omit_para_style_defaults(&p.style, &defaults.paragraph);

      let mut out = serde_json::Map::new();
      // id 항상 omit (모델 미사용 디버그 라벨)
      // sec — 응답 전체에서 같으면 omit. doc_meta.anchor.sec 가 진실.
      if !omit_sec {
          out.insert("sec".into(), serde_json::json!(p.sec));
      }
      out.insert("para".into(), serde_json::json!(p.para));
      // type:"text" 는 기본값 — omit. table 만 명시.
      if let Some(cl) = &p.cell_locator {
          out.insert("cell_locator".into(), serde_json::to_value(cl).unwrap_or_default());
      }
      if let Some(s) = para_style {
          out.insert("style".into(), s);
      }
      // 단일 run + 스타일 없음 → text 직속 (빈 text 포함)
      if runs.len() == 1 && runs[0].get("style").is_none() {
          out.insert("text".into(),
              runs[0].get("text").cloned().unwrap_or(serde_json::Value::String(String::new())));
      } else {
          out.insert("runs".into(), serde_json::Value::Array(runs));
      }
      serde_json::Value::Object(out)
  }

  fn compact_run(run: &IrRun, defaults: &DocDefaults, is_first: bool) -> serde_json::Value {
      let style = omit_run_style_defaults(&run.style, &defaults.run);
      let mut out = serde_json::Map::new();
      // 첫 run 의 char_offset 0 은 omit
      if !(is_first && run.char_offset == 0) {
          out.insert("char_offset".into(), serde_json::json!(run.char_offset));
      }
      out.insert("text".into(), serde_json::json!(run.text));
      if let Some(s) = style {
          out.insert("style".into(), s);
      }
      serde_json::Value::Object(out)
  }
  ```

- [ ] **Step 2**: `compact_table` 도 *table 의 sec/id 동일 omit*. type 은 *유지* (`"table"` 명시 — 기본값 아님).

- [ ] **Step 3**: `compact_cell` — id 키 자체 cell 에 없음 (이미 안 만들었음). cell 안 paragraphs 의 compact_text 가 *cell_locator 가지면 nested 라는 컨텍스트* → cell_locator 의 *table_para/row/col* 은 *cell.row/col 과 일치* (중복) → cell 안 nested paragraph 의 cell_locator 에서 *row/col 만 omit* 검토. 또는 *cell_para 만 cell_locator 안에 남김*. *결정*: cell_locator 전체 keep — table_para/row/col/cell_para 4 키가 명령 인자에 그대로 들어가므로 *모델 편의* 우선. 키 omit 효과 5KB 미만.

- [ ] **Step 4**: 영향 받는 unit test 정정 — `compact_text_single_run_inline` 등이 `id`/`sec`/`type` 키 *존재* 를 assert 했다면 *부재* 로 정정.

- [ ] **Step 5**: 빌드 + test
  ```bash
  cd server && $HOME/.cargo/bin/cargo test ir_compact::tests::
  ```

- [ ] **Step 6**: commit
  ```bash
  git add server/src/ir_compact.rs
  git commit -m "Task #zephy-bridge Sub-3 v2: compact_text/run 의 구조 키 omit — id/type/sec/char_offset:0"
  ```

#### Task 3.2 — `compact_ir_slice` 진입에서 sec 단일성 판단

**Files:** `server/src/ir_compact.rs::compact_ir_slice`

- [ ] **Step 1**: 모든 paragraph 의 sec 가 *같으면* compact_text 에 `omit_sec=true` 전달:
  ```rust
  pub fn compact_ir_slice(ir: IrSlice) -> CompactIrSlice {
      let defaults = compute_doc_defaults(&ir);
      // sec 단일성 판단
      let secs: std::collections::HashSet<usize> = ir.paragraphs.iter().map(|p| match p {
          IrParagraph::Text(t) => t.sec,
          IrParagraph::Table(t) => t.sec,
      }).collect();
      let omit_sec = secs.len() <= 1;

      let paragraphs: Vec<serde_json::Value> = ir.paragraphs.iter().map(|p| match p {
          IrParagraph::Text(t) => compact_text(t, &defaults, omit_sec),
          IrParagraph::Table(tt) => compact_table(tt, &defaults, omit_sec),
      }).collect();
      CompactIrSlice { doc_meta: ir.doc_meta, paragraphs, defaults }
  }
  ```

- [ ] **Step 2**: e2e 갱신 — 셀 안 nested paragraph 가 cell_locator 가지는 것은 그대로, id/type/sec 부재 확인. 본 문서의 sec 가 단일이면 sec 키도 부재.

- [ ] **Step 3**: 실문서 크기 측정
  ```bash
  curl -s "http://127.0.0.1:7710/sessions/sim-1780843626/ir-slice?mode=compact" | wc -c
  ```
  → Phase 1 후 ~130KB, Phase 3 후 ~100KB 기대.

- [ ] **Step 4**: commit
  ```bash
  git add server/src/ir_compact.rs
  git commit -m "Task #zephy-bridge Sub-3 v2: compact_ir_slice 가 sec 단일성 판단 후 omit 결정"
  ```

### Phase 4 — 노트북 라우터 + init.md 가이드 갱신

#### Task 4.1 — 노트북 라우터 page 키 변환

**Files:** `hwp_sub_agent_simulation_ssr.ipynb` cell 3 `_handle_get_ir_slice`

- [ ] **Step 1**: query 변환에 한 줄 추가
  ```python
  if 'page' in payload:
      query['page'] = str(payload['page'])
  ```

- [ ] **Step 2**: self-test cell 4 재실행 확인.

#### Task 4.2 — init.md 가이드 갱신

**Files:** `26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/init.md`

- [ ] **Step 1**: §1 의 payload 키 표에 `page` 추가
  ```markdown
  | `page` | (전체) | 페이지 번호 (0-based). 지정 시 해당 페이지의 paragraph 만 응답. para_start/para_end 와 동시 지정 시 page 우선. |
  ```

- [ ] **Step 2**: §3 (defaults 복원 규칙) 끝에 *키 생략 해석* 한 줄 추가
  ```markdown
  - `type` 키 생략 → `"text"` (기본). `"table"` 만 명시됩니다.
  - `sec` 키 생략 → `doc_meta.anchor.sec` 값 (응답 전체에서 같은 섹션일 때 paragraph 마다 sec 가 생략됨).
  - `char_offset` 키 생략 → 0 (첫 run 의 char_offset).
  - paragraph 의 `id` 키는 *항상 생략* — sec/para/cell_locator 로 식별.
  ```

- [ ] **Step 3**: §4 (좌표 결정) 의 셀 내부 분류 갱신 — *paragraphs[] 평탄에서 cell_locator 찾는 대신* nested 방식 명시:
  ```markdown
  3. **셀 내부 문단** — `paragraphs[]` 안 type:"table" 문단의 `cells[].paragraphs[]` 안에서 발견. 각 셀 안 paragraph 가 `cell_locator: {table_para, row, col, cell_para}` 4 좌표를 직접 보유.
  ```

- [ ] **Step 4**: 가이드 갱신 commit (26ZEPHY-skills 디렉토리에서)
  ```bash
  cd /Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/26ZEPHY-skills
  git add skills/document_edit/hwp-doc-edit/references/init.md
  git commit -m "init.md: Sub-3 v2 정합 — page 키 + 키 생략 해석 + nested 셀 분류"
  ```

#### Task 4.3 — 최종 측정 + 보고

- [ ] **Step 1**: 실문서 응답 측정 (Phase 1+2+3 모두 적용 후)
  ```bash
  curl -s "http://127.0.0.1:7710/sessions/sim-1780843626/ir-slice?mode=compact" | wc -c
  curl -s "http://127.0.0.1:7710/sessions/sim-1780843626/ir-slice?mode=compact&page=0" | wc -c
  ```

- [ ] **Step 2**: 절감률 계산 — 시작 249KB → Phase 1 후 → Phase 3 후 → Phase 2 페이지 0 만.

- [ ] **Step 3**: 최종 보고서 `mydocs/report/task_m200_zephy_bridge_sub3_v2_report.md` 작성
  - 진단 + 적용 방안 + 절감률 측정 + 시연 안내

- [ ] **Step 4**: commit
  ```bash
  git add UNIVA-rhwp/mydocs/report/task_m200_zephy_bridge_sub3_v2_report.md
  git commit -m "Task #zephy-bridge Sub-3 v2: 최종 보고서 + 절감률 측정"
  ```

## 5. 검증 — DoD

| 조건 | 검증 |
|---|---|
| 1. 셀 평탄 entry 가 응답에서 사라짐 | `curl ... | grep cell_locator | wc -l` 가 *nested entry 수만* (본 문서는 351 → table.cells 안의 cell 당 paragraph 수 합산만) |
| 2. nested cell_locator 가 *table.cells[i].paragraphs[j]* 안에 들어있음 | sub3-ir-compact.test.mjs 검증 5 정정 통과 |
| 3. `page=N` query 동작 | sub3-ir-compact.test.mjs 의 *Phase 2 시나리오* 통과 + curl 으로 다른 페이지 응답 크기 비교 |
| 4. id/sec/type/char_offset:0 omit 동작 | unit test 갱신본 통과 |
| 5. raw 모드 회귀 0 | 기존 sub2-ir-slice 등 e2e 통과 |
| 6. 실문서 응답 *최소 50% 절감* | wc -c 비교 — 249KB → 125KB 이하 |
| 7. 노트북 + init.md 가이드 정합 | LLM 시연 통과 (사용자 확인) |

## 6. 절감 예상

| 단계 | 250KB 기준 | 절감률 |
|---|---|---|
| 시작 (현재) | 249 KB | — |
| Phase 1 (셀 평탄 제거) | ~132 KB | ~47% |
| Phase 3 (구조 키 omit) | ~105 KB | ~58% |
| Phase 2 의 `page=0` (페이지 1/N) | *문서 크기/페이지 수에 비례* | 본 30 paragraph 문서가 1 페이지면 효과 적음, 50 페이지면 ~1/50 |

큰 문서 (수십 페이지) 의 *현재 페이지 만* 요청 시 — **수 KB 수준** 으로 떨어짐.

## 7. 후속 sub 후보 (v3)

본 작업이 *큰 4건* 만 처리. v3 (또는 별도 sub) 로 미루는 항목:
1. **D**. defaults 박스의 *실제 사용된 키만* — mode 가 무용한 다양 폰트 문서 정리
2. **E**. 표 안 동일 style 셀 그룹화 + 빈 cell.paragraphs omit
3. **F**. font-name 등 *dictionary 압축* — 반복 글꼴 이름을 dictionary 키로 대체
4. **G**. WS broadcast 의 IR delta 전파 — 편집마다 *변경 paragraph 만* WS 로
