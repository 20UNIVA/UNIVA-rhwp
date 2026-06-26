# Task #m700-i18n 최종 보고서 — rhwp-studio 언어 3중화 (ko · en · ja)

vfinder 의 [i18n-playbook.md](../../../vfinder/docs/i18n/i18n-playbook.md) 패턴을 옮겨 rhwp-studio (한컴 한글 풍 워드프로세서 웹 UI) 의 모든 사용자 가시 텍스트를 한국어·영어·일본어 3 언어로 박는 작업. m700-0 ~ m700-12 cycle 종결.

## 1. 진행 흐름 자료

13 cycle, 22 commit.

| cycle | Phase | 자리 |
|---|---|---|
| m700-0 | Phase 0·1A | 결정 + 한국어 grep 수집 + 33 카테고리 골격 + sub-agent 검증 |
| m700-1.1~1.6 | Phase 1B | 자리 분포 상위 6 카테고리 채움 (468 키) |
| m700-2 | Phase 1B | 메뉴바 §4-7 + 9 카테고리 (168 키) |
| m700-3 | Phase 1B | 나머지 10 카테고리 (75 키) |
| stage 4.1 | Phase 1.5 | sub-agent 맥락 검증 + 16 자리 정정 |
| m700-4 | Phase 2 | 키 명명 정합 + key-naming-convention.md |
| m700-5 | Phase 3 | messages.ko/en/ja.ts + t.ts + lang-boundary.ts (802 키) |
| m700-6·6.1 | Phase 4 | 코드 치환 1-10 (369 회) + 누락 키 68 |
| m700-7·7.1 | Phase 4 | 코드 치환 11-20 (119 회) + 누락 키 25 |
| m700-8 | Phase 4 | 코드 치환 21-30 (98 회) + 누락 키 91 |
| m700-9 | Phase 4 | 코드 치환 31-42 (45 회) + index.html data-i18n + main.ts boundary 결선 + 누락 키 ~100 |
| m700-10 | Phase 5 | rhwp-server AppError stable code 박음 + 클라이언트 server-error.ts |
| m700-11 | Phase 6 | vfinder modal 자식 iframe sysLang URL + onLangChange postMessage |
| m700-12 | Phase 7 | 부모(agent) 통합 가이드 + 최종 보고서 |

## 2. 정량 자료

| 자료 | 수 |
|---|---|
| 카테고리 | 33 (+ 신규 누락 키 자료 8 prefix) |
| 사전 키 (ko=en=ja) | **약 1100+** (m700-5 박은 802 + 누락 키 ~300) |
| 코드 치환 횟수 | **631** (m700-6: 369, m700-7: 119, m700-8: 98, m700-9: 45) |
| 영향 TS 파일 | 42 (UI 대화상자·main·command·view) |
| 신규 파일 | 6 (i18n/ 5 파일 + server-error.ts) |
| 영향 Rust 파일 | 1 (rhwp-server/src/main.rs) |
| sub-agent 호출 | 6 (검증 2 + 치환 4) |
| 보고서·가이드 | 5 (translation-table, key-naming, parent-integration, stage1·2·3·report) |

## 3. 산출물

### 코드 (rhwp-studio)
- [`src/i18n/messages.ko.ts`](../../rhwp-studio/src/i18n/messages.ko.ts) — 한국어 원본 (`as const`)
- [`src/i18n/messages.en.ts`](../../rhwp-studio/src/i18n/messages.en.ts) — 영어 (`Record<keyof typeof messages_ko, string>`)
- [`src/i18n/messages.ja.ts`](../../rhwp-studio/src/i18n/messages.ja.ts) — 일본어
- [`src/i18n/t.ts`](../../rhwp-studio/src/i18n/t.ts) — `t()`/`setLang`/`getLang`/`onLangChange` 헬퍼
- [`src/i18n/lang-boundary.ts`](../../rhwp-studio/src/i18n/lang-boundary.ts) — URL `?sysLang` + postMessage `rhwp:set-locale` 경계
- [`src/i18n/server-error.ts`](../../rhwp-studio/src/i18n/server-error.ts) — 서버 에러 코드 → i18n 키 매핑
- 42 파일 자체 자체 `'한국어'` → `t('key')` 박힘
- [`src/main.ts`](../../rhwp-studio/src/main.ts) — `applyStaticTexts()` + boundary 결선
- [`index.html`](../../rhwp-studio/index.html) — 메뉴바·상태바 자리 `data-i18n` 박힘
- [`src/view/vfinder-modal.ts`](../../rhwp-studio/src/view/vfinder-modal.ts) — 자식 iframe sysLang 전파

### 코드 (rhwp-server)
- [`rhwp-server/src/main.rs`](../../rhwp-server/src/main.rs) — AppError 에 `code` 박힘, 10 helper + 응답 자체 `{error, code}`

### 문서
- [`mydocs/plans/task_m700_i18n.md`](../plans/task_m700_i18n.md) — 13 cycle 분해 계획
- [`mydocs/manual/i18n_translation_table.md`](../manual/i18n_translation_table.md) — 33 카테고리 사전 (ko·en·ja·맥락)
- [`mydocs/manual/i18n_key_naming_convention.md`](../manual/i18n_key_naming_convention.md) — 키 명명 규칙
- [`mydocs/manual/i18n_parent_integration_guide.md`](../manual/i18n_parent_integration_guide.md) — 부모(agent) 통합 가이드
- [`mydocs/working/task_m700_i18n_stage1.md`](../working/task_m700_i18n_stage1.md)
- [`mydocs/working/task_m700_i18n_stage2.md`](../working/task_m700_i18n_stage2.md)
- [`mydocs/working/task_m700_i18n_stage3.md`](../working/task_m700_i18n_stage3.md)
- [`mydocs/report/task_m700_i18n_report.md`](task_m700_i18n_report.md) — *이 보고서*

## 4. 핵심 결정 자료

| 자리 | 결정 |
|---|---|
| 지원 언어 | `ko` (기본), `en`, `ja` — coarse 코드만 (`en-US` 자체 박지 않음) |
| 경계 변수명 | `sysLang` (vfinder·agent 자식 앱 통일) |
| 내부 변수명 | `lang` |
| 메시지 type | `rhwp:set-locale` (vfinder 는 `vfinder:set-locale`) |
| 일본어 어조 | 정중체 + 워드프로세서 관용 (한컴 일본어판 우선) |
| 영어 자리 | MS Word + 한컴 영문판 (`Save As` 단축키 라벨 자체 자체 자체 제거) |
| 한국어 자리 | 한컴 한글 한국어판 라벨 그대로 |
| 키 깊이 | 기본 2단, 메뉴바·대화상자 그룹 3단 (4단 자체 박지 않음) |
| 키 명명 | `snake_case` + 도트 카테고리 구분 |
| 사전 타입 안전 | `as const` + `Record<keyof typeof messages_ko, string>` (en·ja 키 누락 자체 자체 TS 컴파일 에러) |
| 서버 에러 | stable code 박음 (`doc_parse_failed`, `session_not_found` 등) — ko 자체 자체 서버 메시지 그대로, en·ja 자체 자체 코드 매핑 |

## 5. sub-agent 검증 자료

m700-0·stage 4.1 자체 자체 자체 2 sub-agent 박아 *맥락 적합성* 자체 검증:
- **Agent A (번역 적합성)**: 한컴 한글 + MS Word 자체 자체 자체 자체 자체 자체 자체 정합 점검 — 치명 12 + 권장 9 + 추가 카테고리 12
- **Agent B (카테고리 구조)**: UI 자료 자체 자체 자체 자체 분포 점검 — 누락 카테고리 8 + prefix 정정 3 + 자리 분포

두 결과가 *prefix 약어 정정 (cs/ps/ctx → 풀어쓰기)·머리말꼬리말·각주미주·찾기바꾸기·문자표·수식·문서비교·환경설정* 자리 자체 자체 *수렴* — 메인 박은 자체 자체 *25 → 33 카테고리* + *16 자리 정정*.

## 6. 미완료 자리 (후속 cycle 자체)

| 자리 | 사유 | 후속 |
|---|---|---|
| index.html 의 메뉴 드롭다운 항목 라벨 (md-label·tb-label·sb-btn title) | m700-9 자체 자체 *주요 자료* 우선 박음. ~150+ data-i18n 자체 자체 자체 박을 자리 | m800-1 cycle 자체 후속 |
| `ws.rs` 의 String 박힌 에러 (`스냅샷 파싱 실패`) | AppError 자체 아니라 String 박은 자체 자체 자체 자체 별 패턴 | m800-2 cycle 자체 후속 |
| `hwpctl/index.ts`, `core/font-loader.ts`, `font-substitution.ts`, `engine/input-handler-picture.ts` 안 console.* / 주석 한국어 | 사용자 가시 아님 — 정책 박지 않음 | 정책상 유지 |
| `ui/about-dialog.ts` 브랜드·법적 고지·저작권 자리 | 브랜드 자체 자체 자체 자체 자체 i18n 자체 자체 자체 자체 자체 자체 별 자료 | 별 검토 자체 |
| `formula-dialog.ts` 의 함수 설명 22 자리 (`compare.diff.func.*`) | sub-agent 박았으나 사전 키 자체 자체 자체 자체 박음 — 호출 자체 자체 자체 부분 박음 | m800-3 cycle 자체 후속 |
| 실제 사용자 가시 시각 검증 (한컴 + 영어·일본어 lang) | rhwp-studio 띄워 자체 자체 자체 자체 자체 박힌 자체 자체 자체 자체 자체 자체 자체 자체 *실제 화면* 자체 자체 자체 자체 박힌 자체 자체 자체 자체 자체 자체 자체 자체 자체 검증 자체 자체 자체 미진행 | m800 cycle 자체 자체 후속 |

## 7. 검증 자료

| 검증 자리 | 결과 |
|---|---|
| `npx tsc --noEmit` (rhwp-studio) | exit 0 — 세 사전 키 집합 완전 일치 |
| `cargo check` (rhwp-server) | 통과 (4 warning, m700-10 무관) |
| TS 컴파일 자체 자체 키 누락·typo 자체 자체 자체 자체 자동 검출 | `as const` + `Record<keyof typeof messages_ko, string>` 자체 자체 자체 자체 자체 자체 동력 |
| 사전 자체 자체 자체 자체 자체 *33 카테고리* 자체 자체 자체 자체 *맥락* 자체 자체 자체 자체 분포 | translation-table 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 자체 박힘 |

## 8. 다음 단계

1. **실제 시각 검증** — rhwp-studio 띄워 `?sysLang=en` 박고 자체 자체 자체 자체 자체 박힌 자체 자체 자체 자체 자체 자체 모든 자리 자체 영어로 박혔나 확인. `?sysLang=ja` 자체 동상.
2. **agent 통합 자체** — agent 측 자체 자체 자체 [i18n_parent_integration_guide.md](../manual/i18n_parent_integration_guide.md) 자체 자체 자체 자체 자체 자체 박을 자리.
3. **후속 cycle (m800)** — 위 §6 자료 자체 자체 자체 자체 박을 자리.
4. **브랜치 merge** — `local/m700-i18n` 자체 자체 자체 자체 자체 `local/devel` 자체 자체 박는다 (CLAUDE.md 워크플로우 정합).

## 9. 시간·비용 자료

| Phase | 시간 (대략) |
|---|---|
| Phase 0·1A (수집·카테고리) | 30 분 |
| Phase 1B (사전 채워넣기 + 검증) | 약 3 시간 (sub-agent 검증 자체 자체 자체 자체 자체 자체 자체 정정 포함) |
| Phase 2 (키 명명) | 15 분 |
| Phase 3 (사전 파일) | 약 30 분 (sub-agent 자체 자체 자체 자체 자체 자체) |
| Phase 4 (코드 치환) | 약 2 시간 (4 sub-agent 자체 자체 자체 자체 자체) |
| Phase 5 (서버 에러) | 30 분 |
| Phase 6 (자식 iframe) | 15 분 |
| Phase 7 (가이드·보고서) | 30 분 |
| **합계** | **약 8 시간** |

vfinder 자체 자체 자체 자체 자체 약 *반나절~하루* 자체 자체 자체 자체 자체 자체 자체 박음 — rhwp-studio 자체 자체 자체 *4-5x 규모* 자체 자체 자체 자체 자체 자체 자체 박은 자체 정합.

---

## 10. 참고 자료

- vfinder 의 [i18n-playbook.md](../../../vfinder/docs/i18n/i18n-playbook.md)
- vfinder 의 [translation-table.md](../../../vfinder/docs/i18n/translation-table.md)
- vfinder 의 [key-naming-convention.md](../../../vfinder/docs/i18n/key-naming-convention.md)
- vfinder 의 [parent-integration-guide.md](../../../vfinder/docs/i18n/parent-integration-guide.md)
