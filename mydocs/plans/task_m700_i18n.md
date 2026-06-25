# Task #m700-i18n — rhwp-studio 언어 3중화 (ko · en · ja)

vfinder 의 [i18n-playbook.md](../../../vfinder/docs/i18n/i18n-playbook.md) 자료를 그대로 옮겨 *rhwp-studio* (TypeScript + Vite + iframe 자식 앱) 의 한국어 리터럴을 ko·en·ja 3중화 한다.

## 1. 작업 규모

| 자리 | 수 |
|---|---|
| TS 안 한국어 unique 리터럴 | 약 1321 |
| 한국어 박힌 TS 파일 | 82 |
| index.html 정적 텍스트 | 메뉴바·도구상자·서식바·상태바 |
| 자식 iframe (rhwp-studio 안에서 띄우는 iframe) | 후속 확인 |
| rhwp-server 한국어 에러 메시지 | 후속 확인 |

vfinder 의 약 152~173 키 대비 *대규모 앱* — playbook §12 권장: *카테고리별 PR 분리, Phase 1 채워넣기 하루 이상*.

## 2. Phase 0 — 결정 자리

vfinder 자료 차용 + rhwp 도메인 자리 조정.

| 결정 | 값 | 사유 |
|---|---|---|
| 지원 언어 | `ko`(기본), `en`, `ja` | playbook §0 권장. agent 측 자식 앱 통일. |
| 기본값 | `ko` | sysLang 없거나 무효일 때. |
| 경계 변수명 | `sysLang` | vfinder·agent 의 자식 앱 통일. URL 파라미터 + postMessage 필드. |
| 내부 변수명 | `lang` | 자식 앱 안쪽. |
| 메시지 type | `rhwp:set-locale` | rhwp prefix. |
| 일본어 어조 | 정중체 (~です/~ます) + 워드프로세서 관용 (`削除`, `太字`, `斜体`, `フォント`, `段落`) | 사용자층 = 일반 사용자. |
| 동음이의 점검 자리 | `구역`(section/zone), `스타일`(text style/CSS), `서식`(format/style), `표`(table/mark) | 문서 편집 도메인 자리. |
| 서버 에러 | rhwp-server (Rust) 가 stable code 박을 수 있는 자리 | Phase 5 자리에서 상세 정리. |

## 3. Cycle 분해

| Cycle | 자리 | 분량 | sub-agent |
|---|---|---|---|
| m700-0 | Phase 0 결정 + Phase 1A 한국어 grep 수집·카테고리 분류 + translation-table 골격 박기 | 메인 직접 | X |
| m700-1 | Phase 1B 사전 채워넣기 (카테고리 1-7) | 메인 직접 | X |
| m700-2 | Phase 1B 후속 (카테고리 8-14) | 메인 직접 | X |
| m700-3 | Phase 1B 후속 (카테고리 15-끝) + Phase 1.5 맥락 적합성 검증 | 메인 직접 | X |
| m700-4 | Phase 2 키 명명 + 사용자 검토 | 메인 직접 | X |
| m700-5 | Phase 3 사전 파일 (messages.ko/en/ja.ts) + t.ts 헬퍼 + lang-boundary.ts + main.ts 결선 | 메인 직접 | X |
| m700-6 | Phase 4 코드 치환 (파일 1-20) | sub-agent | O |
| m700-7 | Phase 4 코드 치환 (파일 21-40) | sub-agent | O |
| m700-8 | Phase 4 코드 치환 (파일 41-60) | sub-agent | O |
| m700-9 | Phase 4 코드 치환 (파일 61-82) + index.html 정적 텍스트 박기 | sub-agent | O |
| m700-10 | Phase 5 서버 에러 매핑 (rhwp-server 한국어 자료 → stable code 박기) | 메인 직접 | X |
| m700-11 | Phase 6 자식 iframe 처리 (rhwp-studio 안 iframe 자리 점검) | 메인 직접 | X |
| m700-12 | Phase 7 부모(agent) 통합 가이드 문서 + 마무리 정리 | 메인 직접 | X |

## 4. 산출물

### 코드
- `rhwp-studio/src/i18n/messages.ko.ts`
- `rhwp-studio/src/i18n/messages.en.ts`
- `rhwp-studio/src/i18n/messages.ja.ts`
- `rhwp-studio/src/i18n/t.ts`
- `rhwp-studio/src/i18n/lang-boundary.ts`
- `rhwp-studio/src/main.ts` 자리에 boundary 결선 + onLangChange 등록
- 치환된 82개 TS 파일

### 문서
- `mydocs/plans/task_m700_i18n.md` (*이 문서*)
- `mydocs/working/task_m700_i18n_stageN.md` (cycle 별 자료 보고)
- `mydocs/manual/i18n_translation_table.md` (5열 사전 + 맥락)
- `mydocs/manual/i18n_key_naming_convention.md` (키 규칙)
- `mydocs/manual/i18n_parent_integration_guide.md` (부모(agent) 측 가이드)
- `mydocs/report/task_m700_i18n_report.md` (최종 보고서)

## 5. 검증 자리

각 cycle 끝마다:
- `cd rhwp-studio && npx tsc --noEmit` (사전 키 누락·typo 잡힘)
- `npx vite build` (런타임 placeholder 매핑 잘못 박힌 자리 잡힘)
- 시각 검증 — dev 서버 띄워 sysLang=en, sysLang=ja 둘 다 시각 확인

## 6. 진행 흐름

CLAUDE.md 의 *하이퍼-워터폴* 룰:
- 각 cycle 끝에 단계별 완료보고서 (`_stageN.md`) 박기
- 사용자 승인 받고 다음 cycle 진입
- 모든 cycle 종결 후 최종 보고서 (`_report.md`) 박기

## 7. 참고 자료

- vfinder 의 [i18n-playbook.md](../../../vfinder/docs/i18n/i18n-playbook.md)
- vfinder 의 [translation-table.md](../../../vfinder/docs/i18n/translation-table.md) — 공통 카테고리 자료 차용 가능
- vfinder 의 [key-naming-convention.md](../../../vfinder/docs/i18n/key-naming-convention.md)
- vfinder 의 [parent-integration-guide.md](../../../vfinder/docs/i18n/parent-integration-guide.md)
- vfinder 의 `vfinder-studio/src/i18n/` 파일 5개 — 직접 옮겨 갈음 가능
