# Sub-8 — InsertPageBreak EditOperation 광고 (workbench REST 노출)

## 배경

stylish-doc-edit (26ZEPHY-skills) 의 *blank_build 흐름 — 빈 새 문서 N 페이지 만들기* 를 rhwp 12 명령 어휘로 갈아끼우는 작업 중, *page break 동작이 외부 REST 에 광고되지 않은 자리* 가 발견됨.

함수 자체는 이미 구현됨:

- [text_editing.rs:1071-1131](../../src/document_core/commands/text_editing.rs#L1071-L1131) `insert_page_break_native(section, para, offset)` — 커서 위치에서 문단 분할 + 새 문단에 `ColumnBreakType::Page` 설정 + `recompose_section` + `paginate_if_needed` + `invalidate_page_tree_cache`. *Ctrl+Enter 동등 동작*.

외부 REST 진입은 막힘:

- [edit_op.rs:219-362](../../src/document_core/commands/edit_op.rs#L219-L362) 의 `EditOperation` enum 에 `InsertPageBreak` variant *없음*.
- [main.rs `workbench` handler](../../server/src/main.rs) 의 `req.action.as_str()` 매칭 12 갈래에 `"insert_page_break"` *없음*.

audit 결과 *PartialParagraphStyle 의 7 광고 키 (align, line_height, margin_left, margin_right, indent, spacing_before, spacing_after)* 어디에도 *page_break_before / column_type* 자리가 없어 *`set_paragraph_style` 두 호출 우회* 가 불가능 (`deny_unknown_fields` 가 막음).

따라서 본 sub 는 *광고 연결 한 자리만* 정합 — 함수·자료가 모두 이미 있어 변경 자리가 *3 파일* 로 제한된다.

## 목표

1. *신규 EditOperation variant* `InsertPageBreak { section, para, offset }` 등록.
2. *workbench REST* 의 `"insert_page_break"` action arm 추가 — payload `{section, para, offset}` 받음.
3. 기존 12 명령과 *동일 보장* — `op_stash` 영속, broadcast (`origin_client_id=None`), `apply_op_with_stash` 경유.
4. *최소 E2E 검증* — `POST /hwp/sessions/:id/workbench` 한 줄로 page_break 적용·다음 page 시작 idx 확인.

## 비목표

- *단 나누기* (`insert_column_break_native`) 같은 형제 자리 동시 광고 — 별도 sub (Sub-9 검토).
- hwp-doc-edit / stylish-doc-edit 의 모델 가이드 (SKILL.md / references) 갱신 — 본 sub 의 *결과 광고가 안정* 된 다음 별개 사이클.
- rhwp-studio (브라우저 측) WASM API 의 새 `wasm.insertPageBreak()` 노출 — *서버 REST broadcast 만 추가*, WS 진입 op 처리는 *기존 EditOperation::SplitParagraph + 자체 column_type 분기로 충분* (단 ws.rs 의 ops 처리 자리가 새 variant 도 deserialize 하는지는 Phase 0 에서 확인).
- *Partial*Style 의 새 키 추가* — 본 sub 는 EditOperation 한 자리만 늘림.

## 변경 자리 매트릭스

| 자리 | 변경 |
|---|---|
| [edit_op.rs](../../src/document_core/commands/edit_op.rs) `EditOperation` enum | 신규 variant `InsertPageBreak { section, para, offset }` |
| [edit_op.rs](../../src/document_core/commands/edit_op.rs) `affected_range` | 신규 variant 의 영향 범위 — `AffectedRange { section, para_start: para, para_end: para+1 }` (split 결과 새 문단이 para+1 자리에 들어가므로) |
| [edit_op.rs](../../src/document_core/commands/edit_op.rs) `apply` (또는 `apply_to`) | 매칭 arm 추가 — `core.insert_page_break_native(section, para, offset)` 호출 |
| [main.rs](../../server/src/main.rs) `workbench` handler | `"insert_page_break" =>` arm — payload 에서 section / para / offset 추출, `EditOperation::InsertPageBreak {...}` 만들어 `apply_op_with_stash` |
| [main.rs](../../server/src/main.rs) `apply_ops` (`POST /sessions/:id/ops`) | *자동 정합* — `EditOperation` enum 의 `#[serde(tag = "op", rename_all = "snake_case")]` 가 새 variant 도 그대로 받음. 추가 작업 없음. |
| [ws.rs](../../server/src/ws.rs) `handle_client_text` | *자동 정합* — `ClientMessage::Ops` 가 `EditOperation` 그대로 deserialize, broadcast 도 같은 경로. 추가 작업 없음. |
| (선택) e2e test | `POST /sessions/:id/workbench` `{action: "insert_page_break", ...}` 한 줄 시나리오 — base 문서 1 페이지 → page_break 후 2 페이지 확인. |

*핵심: 코드 변경은 3 자리 (enum variant + affected_range + apply arm) + workbench handler arm. ops·ws 진입은 enum tag 정합 덕에 자동 동작*.

## 설계 — Phase 분할

### Phase 0 — 영향 자리 정확히 파악 (코드 변경 0)

1. `EditOperation::apply` (또는 `apply_to`) 함수의 매칭 자리 확인 — 12 variants 각자의 `core.X_native(...)` 호출 모양 한 번 통일된 형식 따라 새 arm 작성.
2. `affected_range` 의 `InsertParagraph` arm 모양 참고 — `InsertPageBreak` 의 영향 범위 정의.
3. `op_stash` 에 저장되는 `op_json_str` 모양 확인 — `apply_op_with_stash` 가 자동 직렬화 ✓.
4. `ws.rs` 의 `ClientMessage::Ops` deserialize 가 새 variant 자동 인식하는지 확인 (`from_value` 한 줄로 처리되는지).

### Phase 1 — EditOperation 추가 + apply wire

1. `edit_op.rs` 의 `EditOperation` enum 에 `InsertPageBreak { section, para, offset }` variant 추가 (12 → 13 variants).
2. `affected_range` 매칭에 새 arm.
3. `apply` 매칭에 새 arm — `core.insert_page_break_native(section, para, offset)`.
4. *cargo build* 통과 — apply 매칭 누락 시 컴파일 에러로 정합 강제.

### Phase 2 — workbench arm + e2e test

1. `main.rs` 의 `workbench` handler 에 `"insert_page_break"` arm — payload deserialize → `EditOperation::InsertPageBreak` op 만들기 → `apply_op_with_stash`.
2. 기존 12 arm 모양 그대로 따라가는 작은 자리.
3. *e2e test 한 자리* — `POST /sessions/:id/workbench` 한 줄 검증. tests 디렉토리의 기존 시나리오 한 자리 옆에 박음.

### Phase 3 — 정합 검증 + 커밋

1. `cargo fmt --all` (포맷은 본 sub 의 새 파일·자리만).
2. `cargo clippy --workspace -- -D warnings` — warning 0.
3. `cargo test` — 전 테스트 통과.
4. 커밋 분할: Phase 1 한 커밋, Phase 2 한 커밋. 두 자리에 `Task #zephy-bridge Sub-8: …` 접두.

## 검증 시나리오 (Phase 2 의 e2e 모양)

```bash
# 1) 빈 1-paragraph 문서 세션 생성
curl -X POST http://127.0.0.1:7710/hwp/documents \
  -H 'Content-Type: application/json' \
  -d '{"filename":"test.hwpx","file_base64":"<빈 hwpx>"}'

# 2) page_break 적용 — section 0, para 0, offset 0
curl -X POST http://127.0.0.1:7710/hwp/sessions/<file_id>/workbench \
  -H 'Content-Type: application/json' \
  -d '{"action":"insert_page_break","payload":{"section":0,"para":0,"offset":0}}'

# 3) IR slice 로 페이지 분할 확인
curl http://127.0.0.1:7710/hwp/sessions/<file_id>/ir-slice?mode=compact
# → sections[0].paragraphs[1].column_type == "Page" 확인
```

## 위험 자리

- `insert_page_break_native` 가 *반환하는 JSON* (`{paraIdx, charOffset}`) 의 모양이 *기존 12 EditOperation apply 반환 모양* 과 다를 수 있음. `apply_op_with_stash` 가 반환값을 어떻게 처리하는지 Phase 0 에서 확인. 정합 깨지면 *apply arm 에서 빈 String 반환* 으로 우회 가능.
- `recompose_section` + `paginate_if_needed` 가 *큰 문서에서 비용* 일 수 있지만 동기 호출 — server 측 lock hold time 확인. 본 sub 의 비목표 (성능) 라 *측정만*.
- 옛 `op_stash` 에 *없는 op 종류* 가 새로 박히면 *기존 sqlite 의 op 재생이 깨질* 수 있음. `op_stash` 가 *opaque JSON 그대로 저장* 이라 새 op 도 자연 정합 — Phase 0 에서 확인.

## 의존·이어지는 작업

- *선행*: 없음. 함수·자료가 모두 이미 있음.
- *후속*:
  - hwp-doc-edit 의 명령 카탈로그 12 → 13 (`SKILL.md` 한 자리 + `actions/body.py` 한 자리). *별개 작업 사이클*.
  - stylish-doc-edit (`feature/jerry-rhwp`) 의 단계 3 patch-phase.md 재작성. *본 sub 광고 안정 후 재개*.

## 승인 요청

위 설계 방향대로 진행해도 OK 한지 작업지시자 확인 부탁. 승인 후 *구현 계획서* (`task_m200_zephy_bridge_sub8_impl.md`) 작성으로 진입.
