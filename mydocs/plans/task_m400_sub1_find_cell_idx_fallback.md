# Task #m400 Sub-1 — `find_cell_idx` Table 자동 검색 fallback (구현 계획서)

수행 계획서: [task_m400_find_cell_idx_and_page_alignment.md](task_m400_find_cell_idx_and_page_alignment.md)

## 배경

[edit_op.rs:808-843](../../src/document_core/commands/edit_op.rs#L808-L843) 의 `find_cell_idx` 가 `para.controls.get(control_idx)` 로 *정확한 자리 control* 만 받음. [main.rs:780·843·882·922](../../server/src/main.rs#L780) 의 4 호출자 모두 `control_idx=0` 하드코딩.

paragraphs[0] 처럼 *섹션 첫 문단* 은 `controls = [SectionDef, ColumnDef, Table]` 모양 — control_idx=0 자리가 SectionDef. 사고 발생. paragraphs[1]/[3] 같은 *섹션 내부 문단* 은 `controls = [Table]` 한 자리 — control_idx=0 자리가 Table. OK.

## 진입 전제

```bash
~/.cargo/bin/cargo build --workspace --quiet  # 베이스 통과
grep -nE "control_idx=0|find_cell_idx" src/document_core/commands/edit_op.rs server/src/main.rs | head -8
```

## Stage 분해

| Stage | 패치 | 검증 |
|---|---|---|
| 1 | `find_cell_idx` 본체에 *Table 자동 검색 fallback* 추가 | `cargo build --workspace` 통과 |
| 2 | 단위 테스트 2 자리 추가 — *섹션 첫 표 (controls=[SectionDef, Table])* + *섹션 내부 표 (controls=[Table])* | `cargo test --lib find_cell_idx` PASS |
| 3 | main.rs 의 4 호출자 (set_cell_style·merge_cells·replace_cell_runs·insert_text_in_cell·delete_range_in_cell — 5 자리) — *변경 없음* 확인. 호환 보존 | grep 결과 |

## Stage 1 — `find_cell_idx` fallback 패치

### 패치

**before** ([edit_op.rs:827-835](../../src/document_core/commands/edit_op.rs#L827-L835)):

```rust
let table = match para.controls.get(control_idx) {
    Some(crate::model::control::Control::Table(t)) => t,
    _ => {
        return Err(HwpError::RenderError(format!(
            "find_cell_idx: control_idx={} 가 Table 아님",
            control_idx
        )))
    }
};
```

**after**:

```rust
// control_idx 자리 우선 시도. 실패하면 paragraph 안의 *첫 Table control* 자동 검색.
// 호출자가 control_idx=0 하드코딩 자리에서 paragraph 가 SectionDef/ColumnDef 같은 다른
// control 을 먼저 가질 때 (섹션의 첫 문단 자리) 자동 우회.
let table = para
    .controls
    .get(control_idx)
    .and_then(|c| match c {
        crate::model::control::Control::Table(t) => Some(t.as_ref()),
        _ => None,
    })
    .or_else(|| {
        para.controls.iter().find_map(|c| match c {
            crate::model::control::Control::Table(t) => Some(t.as_ref()),
            _ => None,
        })
    })
    .ok_or_else(|| {
        HwpError::RenderError(format!(
            "find_cell_idx: table_para={} 에 Table control 없음 (controls_len={})",
            table_para_idx,
            para.controls.len()
        ))
    })?;
```

## Stage 2 — 단위 테스트 2 자리

### 패치 1 — *섹션 첫 표* (controls = [SectionDef, Table]) 정상 동작 확인

`#[cfg(test)] mod tests` (edit_op.rs 끝 자리) 안에 추가:

```rust
#[test]
fn find_cell_idx_falls_back_for_section_def_paragraph() {
    use crate::model::{
        control::Control,
        section_def::SectionDef,
        table::{Cell, Table},
    };
    // 섹션 첫 문단 모양 — controls 가 [SectionDef, Table] 순서
    let mut core = DocumentCore::new_empty();
    core.document.sections.push(Default::default());
    let mut para = crate::model::paragraph::Paragraph::default();
    para.controls.push(Control::SectionDef(Box::new(SectionDef::default())));
    let table = Table {
        rows: 1,
        cols: 1,
        cells: vec![Cell { row: 0, col: 0, ..Default::default() }],
        ..Default::default()
    };
    para.controls.push(Control::Table(Box::new(table)));
    core.document.sections[0].paragraphs.push(para);

    // 옛 동작: control_idx=0 자리 = SectionDef → 사고
    // 새 동작: fallback 으로 controls 안 Table 자동 검색 → 셀 (0,0) 자리 발견
    let cell_idx = core.find_cell_idx(0, 0, 0, 0, 0).unwrap();
    assert_eq!(cell_idx, 0);
}

#[test]
fn find_cell_idx_direct_for_table_only_paragraph() {
    use crate::model::{
        control::Control,
        table::{Cell, Table},
    };
    // 섹션 내부 문단 모양 — controls 가 [Table] 한 자리
    let mut core = DocumentCore::new_empty();
    core.document.sections.push(Default::default());
    let mut para = crate::model::paragraph::Paragraph::default();
    let table = Table {
        rows: 2,
        cols: 3,
        cells: vec![
            Cell { row: 0, col: 0, ..Default::default() },
            Cell { row: 0, col: 1, ..Default::default() },
            Cell { row: 0, col: 2, ..Default::default() },
            Cell { row: 1, col: 0, ..Default::default() },
            Cell { row: 1, col: 1, ..Default::default() },
            Cell { row: 1, col: 2, ..Default::default() },
        ],
        ..Default::default()
    };
    para.controls.push(Control::Table(Box::new(table)));
    core.document.sections[0].paragraphs.push(para);

    // control_idx=0 자리가 Table — 직접 매핑
    let cell_idx_00 = core.find_cell_idx(0, 0, 0, 0, 0).unwrap();
    let cell_idx_12 = core.find_cell_idx(0, 0, 0, 1, 2).unwrap();
    assert_eq!(cell_idx_00, 0);
    assert_eq!(cell_idx_12, 5);
}
```

(★ 만약 `DocumentCore::new_empty()` 가 *섹션 없이* 만들어지면, `core.document.sections.push(Default::default())` 자리에서 빈 Section default 가 paragraphs 빈 자리로 시작 — paragraphs.push 정합.)

(★ Cell / Table / SectionDef 의 `Default::default()` 가 *모두 derive* 되어 있는지 미확인. 빌드 실패 시 *직접 필드 초기화* 로 fallback.)

## Stage 3 — main.rs 4 호출자 변경 없음 확인

```bash
grep -nE "find_cell_idx" server/src/main.rs
# 4 자리 모두 control_idx=0 그대로
# (Sub-1 의 fallback 이 자동 우회 — 호출자 변경 0)
```

## 검증

```bash
cd /Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp

# 1. 새 단위 테스트 2 자리
~/.cargo/bin/cargo test --lib find_cell_idx 2>&1 | tail -10
# 2 PASS 기대

# 2. 회귀
~/.cargo/bin/cargo test --workspace --lib 2>&1 | tail -5
# 1487+ PASS, 0 FAIL

# 3. clippy
~/.cargo/bin/cargo clippy --workspace --lib -- -D warnings 2>&1 | tail -5
# 0 warn / 0 err
```

## 위험 자리

| 위험 | 가정 | 검증 |
|---|---|---|
| 한 문단에 *Table 이 여러 개* 인 자리 | hwp 표준상 *없음* — 표 둘이면 각자 다른 문단 | 회귀 테스트가 잡음 |
| Cell / Table / SectionDef default derive 미정합 | derive 되어 있어야 — 빌드 안 되면 직접 필드 초기화 fallback | Stage 1 빌드 확인 |
| fallback 동작이 *원래 control_idx 자리에 Table 있는데 다른 자리 Table 도 있는 모순 자리* 에서 *첫 Table 만 반환* | hwp 표준상 표 1 자리 가정 — 모순 자리 없음 | 회귀 |
