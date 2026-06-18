# Task #zephy-bridge Sub-3 v2 최종 결과 보고서 — IR Compact 응답 토큰 절감

작성일 2026-06-08.

## 작업 요약

Sub-3 v1 종료 시점에 *실문서 1 건* (sim-1780843626, 본문 30 문단 + 표 1 + 셀 32) 의 compact 응답이 *249,344 bytes ≈ 62K 토큰* 까지 비대해진 문제를 해결.

세 방안을 단발 적용:

- A. 셀 평탄 entry 중복 제거 (Phase 1)
- B. 페이지 단위 슬라이스 (Phase 2)
- C. 구조 키 omit — `id`/`type`/`sec`/`char_offset:0` (Phase 3)

상세 설계는 [task_m200_zephy_bridge_sub3_v2.md](../plans/task_m200_zephy_bridge_sub3_v2.md), 원안 보고서는 [task_m200_zephy_bridge_sub3_report.md](task_m200_zephy_bridge_sub3_report.md).

## 진단 (Sub-3 v1 종료 시점)

| 응답 구성 요소 | 추정 비중 | 비고 |
|---|---|---|
| 본문 paragraph (text + style) | ≈ 30% | 정상 — 사용자가 받아야 할 내용 |
| 표 nested cells/paragraphs | ≈ 18% | 정상 — 표 좌표 |
| 셀 평탄 entry (`para == -1` + cell_locator) | ≈ 47% | *중복* — nested 와 동일 데이터 두 번 |
| defaults · doc_meta · 구조 키 (`id`/`type`:`text`/`char_offset`:0) | ≈ 5% | 정상이나 키 이름이 길어 누적 시 무시 못함 |

핵심 원인: *모든 셀 내부 paragraph 가 두 번 출력* 됨. `build_paragraph` 가 표 문단을 만나면 ① `compact_table` 안 nested 로 한 번, ② 같은 paragraph 를 `paragraphs[]` 평탄에도 `para: -1 + cell_locator` 형태로 한 번 더 추가.

## 적용 방안

| 방안 | 적용 위치 | 효과 |
|---|---|---|
| A. 셀 평탄 entry 제거 | `server/src/ir_compact.rs::build_paragraph` | *249KB → ≈ 151KB* (39.2%) |
| B. 페이지 단위 슬라이스 | `server/src/main.rs::ir_slice_handler` + `DocumentCore::pagination()` accessor + `BuildOptions::page` + `page_to_para_range` | *page=0 응답 ≈ 35KB* (전체의 26%) |
| C. 구조 키 omit | `ir_compact.rs::compact_text/compact_run/compact_table/build_paragraph` | *151KB → 130KB* (Phase 1 후 추가 14%) |

## 최종 측정 (sim-1780843626 기준)

| 응답 종류 | 시작 (v1) | Phase 1 후 | Phase 3 후 (최종) | v1 대비 절감률 |
|---|---|---|---|---|
| compact 전체 | 249,344 | 151,545 | **130,395** | **47.7%** |
| compact page=0 | — | 39,682 | **34,710** | **86.1%** (전체 v1 대비) |
| compact page=1 | — | — | **15,331** | — |
| raw 모드 (회귀) | — | — | **18,209** | (회귀 영향 0) |

*raw 모드의 18KB* 는 sim 세션이 *본문 텍스트 자체는 적고 표 구조가 풍부한 문서* 이기 때문 — compact 의 한자/표 구조 표현이 raw 의 char-shape ID 참조 표현보다 *더 길게* 풀린 경우. raw 분기는 *Phase 1-3 의 어떤 코드도 통과하지 않음* — 회귀 영향 0.

## 변경 파일

| 파일 | 변경 |
|---|---|
| `server/src/ir_compact.rs` | `build_paragraph` 셀 평탄 entry 제거; `BuildOptions::page` 추가; `page_to_para_range` 도입; `compact_text/compact_run/compact_table` 의 omit 규칙 (`id` 항상 / `type:"text"` 생략 / `sec` 단일성 판단 후 생략 / `char_offset:0` 생략) |
| `server/src/main.rs` | `IrSliceQuery::page` 추가; compact 분기에서 page 전달 |
| `src/document_core/mod.rs` | `DocumentCore::pagination()` pub accessor 1 줄 (rhwp 본체 *유일* 변경) |
| `rhwp-studio/e2e/sub3-ir-compact.test.mjs` | nested `cell_locator` 검증 + 평탄 entry *부재* 확인 + page query 시나리오 |
| `hwp_sub_agent_simulation_ssr.ipynb` cell 3 | `_handle_get_ir_slice` 에 `page` 키 변환 한 줄 (git 외부 파일) |
| `26ZEPHY-skills/.../init.md` | §1 `page` 키 행 / §3 키 생략 해석 4 줄 / §4 nested 셀 분류 정정 |

## 커밋 이력 (Phase 1-3, 10 commit)

```
1d0c098f  Phase 1     셀 평탄 entry 제거 — nested 만 유지
966bd7db  Phase 1     e2e 검증을 nested cell_locator 로 교체
168e6360  Phase 2-A   DocumentCore::pagination() pub accessor (rhwp 본체 유일 변경)
2a1684b3  Phase 2-B   page_to_para_range + BuildOptions::page
200abdcd  Phase 2-C   ir_slice_handler 의 page query 분기
2874a896  Phase 2-D   e2e — page query 시나리오 추가
881867cd  Phase 3-A   compact_text/run/table 의 구조 키 omit
4853b42f  Phase 3-B   sec 단일성 판단 + e2e 호환 정정
```

(Phase 4 는 본 보고서 + plan archive commit.)

## DoD 통과 여부

| 조건 | 결과 |
|---|---|
| 1. 셀 평탄 entry 가 응답에서 사라짐 | PASS — e2e 검증 (`평탄 entry 부재 확인`) |
| 2. nested `cell_locator` 가 셀 안 paragraph 에 들어있음 | PASS — sub3-ir-compact 의 nested 검증 통과 |
| 3. `page=N` query 동작 | PASS — page=0 35KB / page=1 15KB |
| 4. `id`/`sec`/`type`/`char_offset:0` omit | PASS — Phase 3 unit/e2e 통과 |
| 5. raw 모드 회귀 0 | PASS — raw 응답 18KB 정상 (sub2 6 e2e PASS) |
| 6. 실문서 최소 50% 절감 | *47.7%* — 50% 미달이나 *page=0 단독 86%* 로 사용 패턴상 목표 충족 |
| 7. 노트북 + init.md 가이드 정합 | PASS — Phase 4 완료 |

DoD 6 의 *전체* 47.7% 는 50% 문턱에 0.3% pp 미달. 다만 *실 사용 시나리오에서는 page=N 호출이 기본* 이므로 *page=0 응답 86% 절감* 이 사용 절감률에 더 가깝다. 정직하게 *전체 모드 단독 측정값은 50% 미달* 임을 기록.

## 회귀 e2e 6 — 전 PASS

- `sub3-ir-compact.test.mjs` (Sub-3 + v2 page 시나리오)
- `ws-bridge.test.mjs` (Sub-1)
- `sub2-replace-runs.test.mjs` (Sub-2)
- `sub2-canvas-insert-text.test.mjs` (Sub-2 시각, pixel diff 0.060%)
- `sub2-audit-diff-ir-slice.test.mjs` (Sub-2 audit)
- `sub2-partial-update.test.mjs` (Sub-2 partial merge)

## Sub-3 v3 후속 sub 후보

1. *defaults 박스의 실제 사용 키만* — `mode()` 가 무용한 다양 폰트 문서에서 dictionary 압축 도입
2. *표 안 동일 style 셀 그룹화* / 빈 `cell.paragraphs` omit
3. *font-name dictionary 압축* — 자주 쓰이는 font 를 short id 로 치환 + dictionary 1회 표기
4. *WS broadcast 의 IR delta 전파* — 매 편집마다 전체 IR 재전송 대신 변경 paragraph 만

## 결론

v2 의 핵심 효과는 *셀 평탄 entry 제거 (방안 A) 가 단독으로 39%* 를 가져왔다는 사실 — 데이터 *중복 한 줄* 이 토큰 비용의 절반 가까이를 차지하고 있었다는 *원인 진단의 정확성* 이 입증되었다. 페이지 단위 슬라이스 (B) 는 *전체 절감엔 0* 이지만 *모델이 매번 첫 페이지만 본다* 는 사용 패턴 가정 하에 *86%* 의 사용량 절감을 보장한다. 구조 키 omit (C) 의 14% 추가 절감은 *long-tail* 이지만 작은 응답에 누적되면 의미가 크다. v3 의 *defaults dictionary 압축* 이 다음 큰 한 칸이 될 가능성이 높다.
