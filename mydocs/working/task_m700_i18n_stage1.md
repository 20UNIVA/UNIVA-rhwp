# Task #m700-i18n Stage 1 보고서 — Phase 0 결정 + Phase 1A 수집·분류

## 자리 — m700-0 cycle 종결 보고

vfinder 의 [i18n-playbook.md](../../../vfinder/docs/i18n/i18n-playbook.md) 자료를 rhwp-studio 에 옮기는 작업의 *첫 cycle*. Phase 0 결정과 Phase 1A 한국어 수집·카테고리 분류 자리.

## 산출물

### 문서
- `mydocs/plans/task_m700_i18n.md` — 전체 cycle 분해 계획서
- `mydocs/manual/i18n_translation_table.md` — UI 문자열 매핑 테이블 (카테고리 골격 + 자리 일부 채움)
- `mydocs/working/task_m700_i18n_stage1.md` (*이 보고서*)

### 자료 (커밋 안 함, 작업용)
- `/tmp/m700-i18n/ts_literals.txt` — 1321 한국어 리터럴 (console.log 포함)
- `/tmp/m700-i18n/visible_ts.txt` — 201 UI 가시 ts 자리
- `/tmp/m700-i18n/visible_html.txt` — 198 index.html 자리
- `/tmp/m700-i18n/visible_all.txt` — 385 unique 합계

## 결정 자리 (Phase 0)

| 결정 | 값 |
|---|---|
| 지원 언어 | `ko`(기본), `en`, `ja` |
| 기본값 | `ko` |
| 경계 변수명 | `sysLang` |
| 내부 변수명 | `lang` |
| 메시지 type | `rhwp:set-locale` |
| 일본어 어조 | 정중체 + 워드프로세서 관용 |
| 동음이의 점검 자리 | `구역`(section/zone), `스타일`(text style/CSS), `서식`(format/style), `표`(table/mark) |
| 서버 에러 | rhwp-server stable code 박을 자리 (Phase 5) |

## 작업 규모 자리

| 항목 | 수 |
|---|---|
| TS 한국어 리터럴 (전체) | 1321 |
| TS 안 console.log 자리 (제외) | 235 |
| TS UI 가시 자리 (textContent·setAttribute·placeholder·aria-label·showToast·title) | 201 |
| index.html 한국어 자리 | 198 |
| **UI 가시 unique 합계** | **385** |

vfinder 의 173 키 대비 약 *2.2배 규모* — 카테고리별 cycle 분리 정합.

## 카테고리 골격 (translation-table.md §1-25)

25개 카테고리로 분류. 골격 박고 자리 일부 채움 (§1, 2, 3, 8, 10).

| § | 카테고리 | prefix | 자리 채움 |
|---|---|---|---|
| 1 | 메뉴바 — 파일 | `menu.file.*` | ✅ 8 자리 |
| 2 | 메뉴바 — 편집 | `menu.edit.*` | ✅ 10 자리 |
| 3 | 메뉴바 — 보기 | `menu.view.*` | ✅ 4 자리 |
| 4-7 | 메뉴바 (입력·서식·쪽·표) | `menu.*.*` | m700-1 cycle |
| 8 | 도구 상자 | `toolbar.*` | ✅ 7 자리 |
| 9 | 서식 도구 모음 | `stylebar.*` | m700-1 |
| 10 | 상태 표시줄 | `statusbar.*` | ✅ 4 자리 |
| 11 | 글자 모양 대화상자 | `cs.*` | m700-1 |
| 12 | 문단 모양 대화상자 | `ps.*` | m700-1 |
| 13 | 표·셀 대화상자 | `table.*` | m700-1 |
| 14 | 편집 용지·구역·격자 | `page.*` | m700-2 |
| 15 | 그림·도형 | `shape.*` | m700-2 |
| 16 | 인쇄·내보내기 | `print.*` | m700-2 |
| 17 | 우클릭 컨텍스트 메뉴 | `ctx.*` | m700-2 |
| 18 | 토스트·진행 표시 | `toast.*` | m700-2 |
| 19 | 공통 버튼 | `button.*` | m700-3 |
| 20 | 빈 상태·안내 | `empty.*` | m700-3 |
| 21 | 시간 표기 | `time.*` | m700-3 |
| 22 | 클라이언트 에러 | `error.client.*` | m700-3 |
| 23 | 서버 에러 | `error.server.*` | m700-3 + Phase 5 |
| 24 | 확인 다이얼로그 | `confirm.*` | m700-3 |
| 25 | 글꼴·언어 | `font.*` | m700-3 |

## 다음 cycle (m700-1) 진입 자리

1. 카테고리 4-7 (메뉴바 입력·서식·쪽·표 드롭다운) — 약 50 자리 박을 자리
2. 카테고리 9 (서식 도구 모음) — 약 30 자리
3. 카테고리 11 (글자 모양 대화상자) — 약 40 자리
4. 카테고리 12 (문단 모양 대화상자) — 약 35 자리
5. 카테고리 13 (표·셀) — 약 25 자리

m700-1 cycle 종결 시 약 *180 자리* 채움 예상. 전체 385 자리 중 약 47% 진입.

## 사용자 검토 자리

승인 받을 자리:
1. Phase 0 결정 자료 (지원 언어·기본값·prefix·일본어 어조) 가 자리에 맞나
2. 25개 카테고리 골격이 rhwp-studio 자리에 맞나
3. m700-1 cycle 분량 (180 자리) 이 적당한가
4. 다음 cycle 진입 시 sub-agent 위탁 vs 메인 직접 — 사전 채워넣기는 *맥락 적합성* 자리라 메인 직접 권장
