# Task #m500 — IrDocMeta `total_pages` / `page` paginator 결과 정합 (수행+구현 통합 계획서)

## 배경

2026-06-12 sim-1781222023 시뮬에서 `get-ir-slice` 응답이 *어떤 호출이든* `total_pages: 1`, `page: 1` 박힘. 실제로는 `?page=0`/`1`/`2` 가 *서로 다른 paragraphs* 반환 (paginator 자체는 동작), 즉 *문서가 다중 페이지인데 doc_meta 만 잘못 박힌* 자리.

근본 원인 — [ir_compact.rs:809-821](../../server/src/ir_compact.rs#L809-L821) 의 IrSlice 응답 본체에서 `page: 1, total_pages: 1` 이 *하드코딩*. paginator 의 실제 결과 (`core.pagination()`) 가 *반영 안 됨*.

## 해결

`rendering.rs:2715` 패턴 ([rendering.rs:2715](../../src/document_core/queries/rendering.rs#L2715)) 정합:

```rust
let total_pages: u32 = self.pagination.iter().map(|p| p.pages.len() as u32).sum();
```

ir_compact.rs:810-821 의 doc_meta 박는 자리에 같은 합 + `opts.page` 1-based 변환 적용.

## 진입 전제

```bash
cd UNIVA-rhwp
git branch --show-current  # feature/jerry-command-expansion
git status --short          # m400 커밋 후 dirty 0
~/.cargo/bin/cargo build --workspace --quiet  # 베이스
```

## Stage 분해

| Stage | 패치 | 검증 |
|---|---|---|
| 1 | `ir_compact.rs:810-821` 의 doc_meta 박는 자리 — total_pages / page 계산 갈아끼움 | `cargo build` |
| 2 | 단위 테스트 — 빈 paginator (fallback 1) + 다중 페이지 paginator (sum) | `cargo test --lib` |
| 3 | 회귀 — 기존 `total_pages: 1` expected 테스트들 정합 (대부분 single page core) | `cargo test --workspace --lib` |
| 4 | clippy + 커밋 + 보고서 | clippy 0/0 |

## Stage 1 — 패치

### before (line 802-821)

```rust
let mut paragraphs = Vec::with_capacity(end.saturating_sub(start));
for p in start..end {
    paragraphs.extend(build_paragraph(core, sec, p));
}

IrSlice {
    doc_meta: IrDocMeta {
        edit_session_id,
        page: 1,
        total_pages: 1,
        anchor: IrAnchor { sec, para_start: start, para_end: end },
    },
    paragraphs,
}
```

### after

```rust
let mut paragraphs = Vec::with_capacity(end.saturating_sub(start));
for p in start..end {
    paragraphs.extend(build_paragraph(core, sec, p));
}

// m500 — paginator 결과로 실제 페이지 수 계산. rendering.rs:2715 패턴 정합.
// 빈 paginator (paginator 미실행 / 빈 문서) 자리는 1 fallback.
let total_pages: u32 = core
    .pagination()
    .iter()
    .map(|p| p.pages.len() as u32)
    .sum::<u32>()
    .max(1);
// opts.page 는 *0-based 내부 인덱스* (BuildOptions 문서 정합).
// 응답 doc_meta.page 는 *1-based 표시* — page=1 이 첫 페이지.
// m400 sub-2 의 main.rs 변환 (외부 1-based → 내부 0-based) 과 정합.
let page_display: u32 = opts
    .page
    .map(|p| p + 1)
    .unwrap_or(1);

IrSlice {
    doc_meta: IrDocMeta {
        edit_session_id,
        page: page_display,
        total_pages,
        anchor: IrAnchor { sec, para_start: start, para_end: end },
    },
    paragraphs,
}
```

## Stage 2 — 단위 테스트

테스트 자리 — `ir_compact.rs` 의 tests mod (line 2200+ 자리). 두 자리 추가:

```rust
#[test]
fn doc_meta_total_pages_falls_back_to_one_for_empty_paginator() {
    // 빈 코어 (paginator 미실행) — total_pages 가 0 이 아닌 1 fallback 인지
    let core = DocumentCore::new_empty();
    let opts = BuildOptions::default();
    let slice = build_compact_ir_slice(&core, &opts);
    assert_eq!(slice.doc_meta.total_pages, 1);
    assert_eq!(slice.doc_meta.page, 1);
}

#[test]
fn doc_meta_page_one_based_when_opts_page_zero() {
    // opts.page = Some(0) (내부 0-based 첫 페이지) → doc_meta.page = 1 (1-based 표시)
    let core = core_with_text("x");  // helper 가 있다면
    let opts = BuildOptions { page: Some(0), ..Default::default() };
    let slice = build_compact_ir_slice(&core, &opts);
    assert_eq!(slice.doc_meta.page, 1);
}
```

(★ 실제 단위 테스트 작성 시 *기존 tests mod 의 helper* / *expected total_pages 가 1 인 자리* 정합. 다중 페이지 코어 만드는 자리는 별개 사이클.)

## Stage 3 — 회귀

기존 `total_pages: 1` expected 테스트들 (line 2216, 2568, 2715, 2741, 2761, 2787 등):
- 모두 *single page core* — 새 fallback 으로 total_pages=1 그대로 정합
- expected page 값들도 1 — 새 page_display=1 (opts.page=None) 정합

## 검증

```bash
~/.cargo/bin/cargo test --workspace --lib 2>&1 | tail -3
~/.cargo/bin/cargo clippy --workspace --lib -- -D warnings 2>&1 | tail -3
```

## 비목표

- *paginator 가 미실행* 일 때 *get-ir-slice 응답 시점에 강제 실행* — 별개 사이클 (m600?)
- *page 0 (외부 0-based) 호출 자리에서 doc_meta.page = 0 표시* — opts.page=None 과 같은 의미 (전체) 로 그대로 1 fallback
- 노트북 시뮬 자동 재현 — VM 재기동 후 자리
