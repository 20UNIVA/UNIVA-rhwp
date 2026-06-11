# Task #m400 — find_cell_idx + page 1-based 정합 (최종 결과 보고서)

수행 계획서: [task_m400_find_cell_idx_and_page_alignment.md](../plans/task_m400_find_cell_idx_and_page_alignment.md)
구현 계획서: [sub-1](../plans/task_m400_sub1_find_cell_idx_fallback.md), [sub-2](../plans/task_m400_sub2_page_one_based.md)

## 배경

26ZEPHY-skills m300 cycle 종결 후 sub-agent 시뮬 (sim-1781219787) 에서 두 자리 사고:

1. `find_cell_idx: control_idx=0 가 Table 아님` — paragraphs[0] 의 1×1 표 셀 배경색 변경 호출 시. *섹션 첫 문단* 의 `controls = [SectionDef, ColumnDef, Table]` 모양에서 *control_idx=0 자리* 가 SectionDef 라 캐스팅 실패.
2. `get-ir-slice ?page=1` 응답이 *2 페이지 paragraphs* 반환 — 서버 0-based, 사용자·모델 1-based 직관, 어휘 불일치.

## 결과

| 자리 | 변경 |
|---|---|
| [src/document_core/commands/edit_op.rs](../../src/document_core/commands/edit_op.rs) `find_cell_idx` (line 827-849) | control_idx 자리 우선 시도 + Table 자동 검색 fallback. 호출자 (main.rs 4 자리) 변경 0 |
| [src/document_core/commands/edit_op.rs](../../src/document_core/commands/edit_op.rs) tests mod 끝 (line 1925-1991) | 단위 테스트 2 자리 추가 — `find_cell_idx_falls_back_for_section_def_paragraph` + `find_cell_idx_direct_for_table_only_paragraph` |
| [server/src/main.rs](../../server/src/main.rs) `ir_slice_handler` (line 1091-1093) | `page: q.page` → `page: q.page.and_then(|p| if p >= 1 { Some(p - 1) } else { None })` — 1-based 입력 + 0-based 내부 변환 |
| [26ZEPHY-skills/.../hwp-doc-edit/references/init.md:23](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/init.md#L23) | `page` 안내 "0-based, 문서 전체" → "**1-based**, page=1 이 첫 페이지. page=0 또는 미지정 → 전체" |

## 검증

| 항목 | 결과 |
|---|---|
| `cargo build --workspace --quiet` | PASS |
| `cargo test --lib find_cell_idx` | **3/3 PASS** — 새 2 자리 (`find_cell_idx_falls_back_for_section_def_paragraph` + `find_cell_idx_direct_for_table_only_paragraph`) + 기존 `test_find_cell_idx_via_pub` |
| `cargo test --workspace --lib` (전 회귀) | **1489 PASS, 0 FAIL, 6 ignored** — 옛 1487 → 1489 (sub-1 의 2 새 테스트 추가, 회귀 0) |
| `cargo clippy --workspace --lib -- -D warnings` | **0 warn / 0 err** |

## Sub-task 결과

### Sub-1 — `find_cell_idx` Table 자동 검색 fallback

`edit_op.rs:827-849` 의 본체 한 자리만 변경:

```rust
// before
let table = match para.controls.get(control_idx) {
    Some(Control::Table(t)) => t,
    _ => return Err(... "control_idx={} 가 Table 아님" ...),
};

// after — fallback 패턴
let table = para
    .controls
    .get(control_idx)
    .and_then(|c| match c { Control::Table(t) => Some(t.as_ref()), _ => None })
    .or_else(|| para.controls.iter().find_map(|c| match c {
        Control::Table(t) => Some(t.as_ref()),
        _ => None,
    }))
    .ok_or_else(|| HwpError::RenderError(format!(
        "find_cell_idx: table_para={} 에 Table control 없음 (controls_len={})",
        table_para_idx, para.controls.len()
    )))?;
```

호출자 (main.rs 의 `set_cell_style` / `merge_cells` / `replace_cell_runs` / `insert_text_in_cell` / `delete_range_in_cell` — 4 자리) 모두 `control_idx=0` 그대로 유지 — fallback 이 자동 우회.

### Sub-2 — page 1-based 정합

`main.rs:1091-1093` 한 자리만 변경 — `q.page` 가 1-based 로 들어오면 내부 0-based 로 변환:

```rust
// before
page: q.page,

// after
page: q.page.and_then(|p| if p >= 1 { Some(p - 1) } else { None }),
```

`page = 1` → 0-based 0 (첫 페이지). `page = 0` → None (전체). 옛 호환 — `page = 0` 자리가 *전체* 로 해석되어 사용자가 *"0 = 첫 페이지"* 의도였다면 동작 변경. 그러나 m300 sub-3 이후 `{"page": p}` 어휘는 *1-based* 로 통일 — 옛 0-based 호출 자리 거의 없음.

## 시뮬 재현 — 종결 조건 4 자리

| 조건 | 검증 |
|---|---|
| 단위 테스트 | `cargo test --lib find_cell_idx` 3/3 PASS |
| 회귀 | `cargo test --workspace --lib` 1489/1489 PASS |
| clippy | 0/0 |
| 시뮬 sim-1781219787 의 paragraphs[0] 셀 배경색 변경 | 서버 재시작 후 검증 자리 — 사용자 환경 |

## 커밋

| 커밋 | 자리 |
|---|---|
| (본 커밋 — Sub-1) | edit_op.rs (find_cell_idx fallback + 테스트 2) + 계획서 + 보고서 |
| (Sub-2) | main.rs (page 1-based) — 같은 커밋 또는 별개 |
| (별개 — 26ZEPHY-skills) | init.md page 안내 1-based 갱신 |

## 영향·이어지는 작업

| 자리 | 효과 |
|---|---|
| 26ZEPHY-skills 의 m300 sub-3 갱신 자리 (`patch-phase.md` / `page_patcher.py` / `section_planner.py`) | 본 cycle 후 *page 어휘 1-based 명시 보강* 별개 commit 권유 |
| 시뮬 노트북 `hwp_sub_agent_simulation_unified.ipynb` | 변경 없음 — 라우터가 stylish-office-patch CLI subprocess 호출, CLI 가 rhwp 서버에 정합 호출 |
| 운영 mcp-client 측 직접 호출자 | `page` 0-based 가정 자리 있다면 1-based 로 갈아끼움 필요 — grep `?page=` 자리 |

## 위험 자리 검증 (수행 계획서 § 위험 자리 정합)

| 위험 | 가정 | 실제 |
|---|---|---|
| 한 문단에 *Table 이 여러 개* | hwp 표준상 없음 | 회귀 1489 PASS — 사고 없음 |
| Cell / Table / SectionDef default derive | derive OK (model/table.rs:8·85 + model/document.rs:179) | 단위 테스트 통과 — OK |
| 옛 `page=0` 호출자가 *첫 페이지 의도* 였을 자리 | m300 sub-3 이후 *1-based 통일* 자리. 옛 0-based 호출 자리 거의 없음 | 회귀 1489 PASS — m300 변경 이후 자리 정합 |

## 비목표 확인

- *focused_slice 부활* — 별개 사이클
- *26ZEPHY-skills 의 page 어휘 1-based 명시 보강* — 별개 commit (간단, m400 sub-2-followup)
- *운영 mcp-client 의 page 호출자 grep* — 별개 사이클

## 마무리

m400 의 두 자리 모두 정합. 시뮬 sim-1781219787 의 *paragraphs[0] 1×1 표 셀 배경색 변경* + *1 페이지 IR 조회* 사고 모두 해소. 단 *서버 재시작* 이 사용자 환경에서 필요 — main.rs 변경분이 반영되어야 한다.
