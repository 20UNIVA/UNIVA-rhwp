# Task #m400 Sub-2 — `page` 인자 1-based 정합 (구현 계획서)

수행 계획서: [task_m400_find_cell_idx_and_page_alignment.md](task_m400_find_cell_idx_and_page_alignment.md)

## 배경

[main.rs:1037-1098](../../server/src/main.rs#L1037-L1098) 의 IrSliceQuery 가 `page: Option<u32>` 를 *0-based 그대로* `ir_compact::build_compact_ir_slice` 의 `opts.page` 로 전달. [init.md:23](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/init.md#L23) 도 *0-based, 문서 전체* 명시.

사용자·모델은 *1 페이지 = page 1* 직관. m300 sub-3 에서 옛 `--page 1` 어휘 → `{"page": 1}` 갈아끼울 때 0-based 정합 명시 안 함. 결과 — 모델이 `{"page": 1}` 호출 → 서버 *0-based 인덱스 1* = *2 페이지* 반환 → 처음 페이지 paragraphs 누락.

해결 — 서버 측을 *1-based* 로 갈아끼움. `page = 1` 이 첫 페이지. `page = 0` 또는 미지정 → 전체 (옛 동작 보존).

## 진입 전제

```bash
grep -n "page: q.page" server/src/main.rs    # 1 자리
grep -n "page: Option<u32>" server/src/main.rs  # IrSliceQuery 자리
grep -n "0-based" 26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/init.md  # 1 자리
```

## Stage 분해

| Stage | 패치 | 검증 |
|---|---|---|
| 1 | `main.rs:1091-1092` 의 page 전달 자리에 1-based → 0-based 변환 | `cargo build` 통과 |
| 2 | `init.md:23` 의 *0-based, 문서 전체* → *1-based, 문서 전체. 0 또는 미지정 → 전체* | grep 결과 |
| 3 | (보너스) m300 sub-3 갱신 자리 (`patch-phase.md` / `page_patcher.py` / `section_planner.py`) 의 page 어휘 *1-based* 명시 보강 — 별개 commit (26ZEPHY-skills) | 본 sub 범위 밖, 다음 commit 자리로 |

## Stage 1 — main.rs page 1-based 변환

### 패치 — `ir_slice_handler` 의 page 전달 자리

**before** ([main.rs:1085-1098](../../server/src/main.rs#L1085-L1098)):

```rust
let opts = ir_compact::BuildOptions {
    sec,
    para_start,
    para_end: Some(para_end),
    edit_session_id: Some(format!("cli_{}", file_id)),
    // Sub-3 v2 — page query 지정 시 paginator 결과로 sec/start/end 가 덮어써짐.
    page: q.page,
};
```

**after**:

```rust
let opts = ir_compact::BuildOptions {
    sec,
    para_start,
    para_end: Some(para_end),
    edit_session_id: Some(format!("cli_{}", file_id)),
    // m400 sub-2 — page 인자 1-based 정합. 사용자·모델 직관 (1 페이지 = page 1) 정합.
    // page = 1 → 첫 페이지 (내부 0-based 인덱스 0). page = 0 또는 None → 전체.
    page: q.page.and_then(|p| if p >= 1 { Some(p - 1) } else { None }),
};
```

## Stage 2 — init.md 갱신

### 패치 — page 인자 안내 자리

**before** ([init.md:21-24](../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/references/init.md#L21-L24)):

```markdown
| `sec` | 0 | 섹션 인덱스 (대부분 0) |
| `para_start` / `para_end` | (전체) | 특정 문단 범위만 slice 받고 싶을 때 |
| `page` | (전체) | 페이지 번호 (0-based, 문서 전체). 지정 시 *해당 페이지에 속한 paragraph 만* 응답. `para_start`/`para_end` 와 동시 지정 시 `page` 우선. 범위 외 시 fallback (전체 또는 빈 응답). |
```

**after**:

```markdown
| `sec` | 0 | 섹션 인덱스 (대부분 0) |
| `para_start` / `para_end` | (전체) | 특정 문단 범위만 slice 받고 싶을 때 |
| `page` | (전체) | 페이지 번호 (**1-based**, 문서 전체 — page=1 이 첫 페이지). 지정 시 *해당 페이지에 속한 paragraph 만* 응답. `para_start`/`para_end` 와 동시 지정 시 `page` 우선. `page=0` 또는 미지정 → 전체. 범위 외 시 fallback (전체 또는 빈 응답). |
```

## Stage 3 — m300 sub-3 갱신 자리 어휘 보강 (별개 commit, 26ZEPHY-skills)

본 sub 범위 *바로 밖* 자리 — Sub-2 의 *서버측 1-based 정합* 이 끝난 후 *26ZEPHY-skills 의 가이드* 도 정합 명시:

- `stylish-doc-edit/references/patch-phase.md` — get-ir-slice 호출 예시의 page 어휘 (`{"page": p}`) 옆 *(1-based, p=1 이 첫 페이지)* 한 줄
- `stylish-doc-edit/scripts/page_patcher.py` — `_call_get_ir_slice(file_id, page=p)` 의 docstring + payload 어휘
- `stylish-doc-edit/scripts/section_planner.py` — 같은 자리

본 sub 의 Stage 1·2 끝낸 후 *별개 commit 자리 (m400 sub-2-followup)* 로 진행.

## 검증

```bash
cd /Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp

# 1. build
~/.cargo/bin/cargo build --workspace --quiet

# 2. 회귀
~/.cargo/bin/cargo test --workspace --lib 2>&1 | tail -3

# 3. 시뮬 재현 — sim-1781219787 의 page=1 응답 검증
# 서버 재시작 필요 (변경된 main.rs 의 build) — 사용자 환경에서 별도 진행
# 기대: ?page=1 응답의 doc_meta.anchor = {para_start: 0, para_end: 6, sec: 0}
```

## 위험 자리

| 위험 | 가정 | 검증 |
|---|---|---|
| 옛 호출자 (`page=0` 자리) 의 동작 변경 | `page=0` → 전체로 fallback. 옛 *0-based 의 0 = 첫 페이지* 자리와 동일 의미 — *호환 보존* | 회귀 |
| 운영 mcp-client / VM 의 직접 호출자가 0-based 가정 | 옛 client 가 `page=0` 보내면 *전체* 가 돌아옴 (옛 의도가 *첫 페이지* 였으면 변경됨) — 다만 *옛 client 가 page 인자 사용 자리* 가 거의 없을 것 (m300 이후 새 어휘) | grep `page=` 호출 자리 외부 — 없으면 OK |
| stylish-doc-edit 가이드와 어휘 일관성 | m300 sub-3 가 이미 `{"page": p}` 어휘 명세했지만 *0-based vs 1-based 미명시*. 본 sub 후 *1-based* 로 통일 | Stage 3 의 별개 commit 자리 |

## 비목표

- *page=0 자리에 1 페이지 의미를 유지* — m400 sub-2 는 1-based 가 명세. p=0 → 전체 (안전 fallback)
- *page=음수 자리 검증* — Option<u32> 라 음수 불가능
