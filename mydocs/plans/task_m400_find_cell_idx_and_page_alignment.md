# Task #m400 — find_cell_idx + page 1-based 정합 (수행 계획서)

## 배경

2026-06-12 26ZEPHY-skills 의 *m300 cycle 종결* 후 sub-agent 시뮬 (sim-1781219787) 에서 두 자리 사고:

| 자리 | 증상 |
|---|---|
| `find_cell_idx: control_idx=0 가 Table 아님` | paragraphs[0] 의 1×1 표 셀 배경색 변경 호출 시. paragraphs[1], [3] 의 표는 정상 동작 |
| `get-ir-slice ?page=1` 응답이 *2 페이지 paragraphs* 반환 | 사용자/모델이 1-based 의도로 호출, 서버는 0-based 해석 |

### Sub-1 — `find_cell_idx` Table 자동 검색

[edit_op.rs:808-843](../../src/document_core/commands/edit_op.rs#L808-L843) 의 `find_cell_idx` 가 `para.controls.get(control_idx)` 로 *정확한 자리 control* 만 받음. [main.rs:780·843·882·922](../../server/src/main.rs#L780) 의 4 자리 모두 `control_idx=0` *하드코딩*.

paragraphs[0] 같은 *섹션 첫 문단* 은 `controls = [SectionDef, ColumnDef, Table]` 모양 — control_idx=0 자리가 *SectionDef*. paragraphs[1]/[3] 같은 *섹션 내부 문단* 은 `controls = [Table]` — control_idx=0 자리가 *Table*.

해결 — `find_cell_idx` 안에서 *control_idx 자리 우선 시도, 실패하면 Table 자동 검색* (fallback). 호출자 (main.rs 의 4 자리) 변경 0.

### Sub-2 — `page` 인자 1-based 정합

[main.rs:1037-1098](../../server/src/main.rs#L1037-L1098) 의 IrSliceQuery 가 *0-based* (init.md:23 명시). 사용자 직관·옛 어휘 (`--page 1`) 모두 1-based. m300 sub-3 에서 `--page 1` → `{"page": 1}` 갈아끼울 때 0-based 명시 안 했어 사고 잔존.

두 갈래:
- **A (권유)**: 서버를 1-based 로 갈아끼움 — `page = 1` 이 첫 페이지. p=0 (또는 None) → 전체.
- **B**: 가이드 5 자리에 0-based 명시 — 모델/사용자 직관과 충돌, 학습 잔존 사고 가능

→ **A 선택**. 한 줄 변경 + init.md 갱신.

## Sub-task 분해

```
Sub-1 (find_cell_idx fallback) ──┐
                                  ├─→ 회귀 게이트 → 최종 보고서
Sub-2 (page 1-based 정합)        ─┘
```

두 자리 독립 — 병렬 진행 OK. 한 사이클 안에 통합.

| Sub | 자리 | 산출 |
|---|---|---|
| 1 | `find_cell_idx` fallback + 단위 테스트 2 자리 (섹션 첫 표 + 섹션 내부 표) | `edit_op.rs` 한 자리 + 테스트 2 |
| 2 | IrSliceQuery 의 page 1-based 매핑 + init.md 갱신 + 회귀 테스트 자리 | `main.rs` 한 자리 + `init.md` 자리 |

## 진입 전제

```bash
cd /Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp
git branch --show-current  # feature/jerry-command-expansion 정합
git status --short         # dirty 0
~/.cargo/bin/cargo build --workspace --quiet  # 베이스 통과
```

## Stage 분해 (전체)

```
Stage 1 — Sub-1 + Sub-2 병렬 진행
   ↓
Stage 2 — 단위 테스트 + 회귀 (cargo test --workspace --lib)
   ↓
Stage 3 — clippy + fmt 정합
   ↓
Stage 4 — 시뮬 sim-1781219787 재현 (paragraphs[0] 셀 배경색 변경 성공)
   ↓
Stage 5 — 최종 보고서 + 커밋 묶음
```

## 종결 조건

| 항목 | 명령 | 기대 |
|---|---|---|
| 단위 테스트 | `cargo test --workspace --lib find_cell_idx` | 새 2 자리 PASS |
| 회귀 | `cargo test --workspace --lib` | 1487+ PASS, 0 FAIL |
| clippy | `cargo clippy --workspace --lib -- -D warnings` | 0 warn / 0 err |
| 시뮬 재현 | `set-cell-style ... table_para=0 ...` | sky blue 적용 (paragraphs[0]) |
| page 1-based | `?page=1` 응답 anchor `{para_start: 0, ...}` | 첫 페이지 |

## 비목표

- m300 sub-3 의 가이드 5 자리에 *page 1-based 정합 어휘* 갱신 — Sub-2 끝난 후 별개 commit (간단)
- *focused_slice 자리 부활* — 별개 사이클
- 시뮬 노트북 라우터에서 *page 자동 변환* 도입 — Sub-2 가 서버 측 정합하므로 불필요
