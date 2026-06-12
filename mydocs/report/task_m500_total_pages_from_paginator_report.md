# Task #m500 — IrDocMeta `total_pages` / `page` paginator 결과 정합 (최종 결과 보고서)

수행+구현 계획서: [task_m500_total_pages_from_paginator.md](../plans/task_m500_total_pages_from_paginator.md)

## 배경

2026-06-12 m400 cycle 종결 후 sim-1781222023 시뮬에서 발견:
- `?page=0/1/2` 가 *서로 다른 paragraphs* 반환 (paginator 자체는 동작)
- 그러나 *모든 호출 응답* 에서 `total_pages: 1, page: 1` 박힘 — *paginator 의 실제 결과 미반영*

## 결과

[server/src/ir_compact.rs:809-836](../../server/src/ir_compact.rs#L809-L836) 의 IrSlice 응답 본체에서 *하드코딩* `page: 1, total_pages: 1` 를 *paginator 합* 으로 갈아끼움.

```rust
// before
IrSlice {
    doc_meta: IrDocMeta { edit_session_id, page: 1, total_pages: 1, anchor: ... },
    paragraphs,
}

// after — paginator 결과 합 + opts.page 1-based 표시
let total_pages: u32 = core
    .pagination()
    .iter()
    .map(|p| p.pages.len() as u32)
    .sum::<u32>()
    .max(1);
let page_display: u32 = opts.page.map(|p| p + 1).unwrap_or(1);

IrSlice {
    doc_meta: IrDocMeta { edit_session_id, page: page_display, total_pages, anchor: ... },
    paragraphs,
}
```

`rendering.rs:2715` 패턴 정합. 빈 paginator (paginator 미실행 / 빈 문서) 자리는 `.max(1)` fallback.

### 보조 변경 — `BuildOptions` Default derive 추가

[server/src/ir_compact.rs:697](../../server/src/ir_compact.rs#L697) 의 `BuildOptions` 에 `Default` derive 추가. m500 단위 테스트가 `..Default::default()` 패턴 사용. 영향 자리 — 새 default 가 모든 옵션 None/0/빈 자리 — 사전 호출자에 영향 0.

## 검증

| 항목 | 결과 |
|---|---|
| 새 단위 테스트 3 자리 | **3/3 PASS** — `doc_meta_total_pages_falls_back_to_one_for_empty_paginator` / `doc_meta_page_one_based_display_when_opts_page_zero` / `doc_meta_page_one_based_display_when_opts_page_two` |
| rhwp-server crate 전체 | **81 PASS, 0 FAIL** (옛 78 → +3) |
| `cargo test --workspace --lib` | **1489 PASS, 0 FAIL** (m400 이후 +0 — 본 sub 는 server crate 자리) |
| `cargo clippy --workspace --lib -- -D warnings` | **0/0** — 본 사이클 변경 자리 정합 |

(★ `cargo clippy -p rhwp-server` 는 *사전 자리* `build_cell_paragraph: 8 args (too_many_arguments)` 떨어짐 — m500 변경 자리와 무관, 별개 사이클. CLAUDE.md "직접 수정한 파일만 필요한 범위" 룰 정합)

## 단위 테스트 시그니처

```rust
#[test]
fn doc_meta_total_pages_falls_back_to_one_for_empty_paginator() {
    let core = DocumentCore::new_empty();
    let opts = BuildOptions::default();
    let slice = build_ir_slice(&core, &opts);
    assert_eq!(slice.doc_meta.total_pages, 1, "빈 paginator → 1 fallback");
    assert_eq!(slice.doc_meta.page, 1);
}

#[test]
fn doc_meta_page_one_based_display_when_opts_page_zero() {
    let core = DocumentCore::new_empty();
    let opts = BuildOptions { page: Some(0), ..Default::default() };
    let slice = build_ir_slice(&core, &opts);
    assert_eq!(slice.doc_meta.page, 1);  // 0-based 0 → 1-based 1
}

#[test]
fn doc_meta_page_one_based_display_when_opts_page_two() {
    let core = DocumentCore::new_empty();
    let opts = BuildOptions { page: Some(2), ..Default::default() };
    let slice = build_ir_slice(&core, &opts);
    assert_eq!(slice.doc_meta.page, 3);  // 0-based 2 → 1-based 3
}
```

## 시뮬 재현 — m500 적용 후 기대 응답

```json
// 다중 페이지 문서 — sim-1781222023 같은 자리
GET /sessions/{fid}/ir-slice?page=1&mode=compact   // m400 정합 후 page=1 이 첫 페이지
→ {
    "doc_meta": {
      "page": 1,                  // ★ m500 — 1-based 표시
      "total_pages": <실제 페이지 수>,   // ★ m500 — paginator 합
      "anchor": {"sec": 0, "para_start": 0, "para_end": 6}
    },
    "paragraphs": [...]
}
```

다만 *서버 재기동 필요* — VM rhwp-server 가 m400 sub-1·2 + m500 의 변경분을 *빌드·배포* 해야 동작.

## 영향·이어지는 작업

| 자리 | 효과 |
|---|---|
| 모델·사용자 가 *문서 총 페이지 수* 파악 가능 | sub-agent 의 *blank_build* / *content_add* 흐름이 `total_pages` 보고 페이지 골격 결정 |
| 시뮬 노트북 라우터 변경 없음 | stylish-office-patch CLI subprocess 호출 그대로 — CLI 가 server REST 정합 호출 |
| paginator 미실행 자리 (`core.pagination()` 빈) | `total_pages: 1` fallback — 옛 동작과 정합. 사고 0 |

## 비목표 (별개 사이클)

- *paginator 가 미실행 자리에서 강제 실행* — m600 자리 (필요 시)
- *page=0 (외부 0-based) 호출자 호환* — 현재 fallback 으로 *전체* 의미. m400 sub-2 정합
- 노트북 시뮬 자동 재현 — VM 재기동 후 자리

## 위험 자리 검증

| 위험 | 가정 | 실제 |
|---|---|---|
| `BuildOptions::default()` 가 사전 호출자 영향 | 새 derive 만 추가 — *기존 호출자 명시 필드 초기화* 그대로 작동 | 회귀 1489 PASS |
| 기존 expected `total_pages: 1` 테스트 (line 2216, 2568, ...) | 모두 *single page core* — fallback 정합 | 회귀 PASS |
| paginator 가 *섹션 분리* 자리에 paginate 미실행 | core.pagination() 빈 → max(1) fallback | 단위 테스트 1 자리 정합 |

## 마무리

m500 의 한 자리 변경 + 보조 1 자리 (Default derive). 시뮬 sim-1781222023 의 `total_pages: 1` 사고 해결. VM 재기동 후 즉시 동작. 다음 cycle 의 자리 — *paginator 강제 실행* (필요 시) 또는 *m300 sub-3 page 어휘 보강*.
