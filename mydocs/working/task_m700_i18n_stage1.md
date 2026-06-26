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

## sub-agent 검증 (사용자 명시 지시 자리)

stage1 종결 직전 사용자가 *각 언어별 맥락 적합성 + 카테고리 정합* 자리를 sub-agent 자리로 검증하라 명시. 두 자리 병렬 호출:

| Agent | 자리 | 결과 자리 |
|---|---|---|
| A | 번역 적합성 (한·영·일 라벨 자리·일본어 어조·동음이의·한컴 vs MS Word 관용) | 치명 4 + 권장 9 + 추가 카테고리 12 |
| B | 카테고리 구조 (385 자리 자료 자체 + UI·코드 정합) | 누락 8 + prefix 정정 3 + 자리 분포 |

두 결과가 *prefix 약어 정정 (cs/ps/ctx)·머리말꼬리말·각주미주·찾기바꾸기·문자표·수식·하이퍼링크·문서비교* 자리에서 *수렴*. 메인이 자리 반영해 *25 → 33 카테고리*로 확장 + 이미 박힌 33 자리 중 *9 자리 정정*:

| 정정 자리 | 자리 |
|---|---|
| `menu.edit.select_all` 한국어 | `모두(A)` → `모두 선택` |
| `menu.edit.copy` 한국어 | `복사` → `복사하기` (메뉴 자리), `복사`는 도구상자(`toolbar.copy`) 한정 |
| `menu.edit.find_replace` 키 자체 | `replace` → `find_replace` 자리 갈람, 한국어 `고치기(D)` → `찾아 바꾸기(E)` |
| `menu.edit.find_again` (신규) | `다시 찾기(X)` |
| `menu.edit.format_copy` (신규) | `모양 복사` |
| `menu.view.show_control_codes` | 한국어 `조판부호` → `조판 부호`, 일본어 `制御記号` → `組版記号` |
| `menu.file.save_as` 영어 | `Save As(A)...` → `Save As...` (영어판 단축키 자리 제거) |
| `menu.file.about` 일본어 | `製品情報` → `バージョン情報` |
| `toolbar.char_shape` 영어 | `Character` → `Font`, 일본어 `文字書式` → `フォント` |
| prefix 약어 자리 | `cs.*`/`ps.*`/`ctx.*` → `char_shape.*`/`para_shape.*`/`context_menu.*` |

## 카테고리 골격 (translation-table.md §1-33)

*25 → 33 카테고리로 확장* (sub-agent 검증 후). 골격 박고 자리 일부 채움 (§1, 2, 3, 8, 10 — 총 33 자리).

| § | 카테고리 | prefix | 자리 채움 |
|---|---|---|---|
| 1 | 메뉴바 — 파일 | `menu.file.*` | ✅ 8 자리 |
| 2 | 메뉴바 — 편집 | `menu.edit.*` | ✅ 12 자리 (find_again·format_copy 추가) |
| 3 | 메뉴바 — 보기 | `menu.view.*` | ✅ 4 자리 |
| 4-7 | 메뉴바 (입력·서식·쪽·표) | `menu.*.*` | m700-1·2 cycle |
| 8 | 도구 상자 | `toolbar.*` | ✅ 7 자리 |
| 9 | 서식 도구 모음 | `stylebar.*` | m700-1 |
| 10 | 상태 표시줄 | `statusbar.*` | ✅ 4 자리 |
| 11 | 글자 모양 대화상자 | `char_shape.*` *(풀어쓰기)* | m700-1 |
| 12 | 문단 모양 대화상자 | `para_shape.*` *(풀어쓰기)* | m700-1 |
| 13 | 표·셀 대화상자 | `table.*` | m700-1 |
| 14 | 편집 용지·구역·격자 | `page.*` | m700-2 |
| 15 | 그림·도형 | `shape.*` | m700-1 |
| 16 | 인쇄·내보내기 | `print.*` | m700-2 |
| 17 | 우클릭 컨텍스트 메뉴 | `context_menu.*` *(풀어쓰기)* | m700-2 |
| 18 | 토스트·진행 표시 | `toast.*` | m700-2 |
| 19 | 공통 버튼 | `button.*` | m700-3 |
| 20 | 빈 상태·안내 | `empty.*` | m700-3 |
| 21 | 시간 표기 | `time.*` | m700-3 |
| 22 | 클라이언트 에러 | `error.client.*` | m700-3 |
| 23 | 서버 에러 | `error.server.*` | m700-3 + Phase 5 |
| 24 | 확인 다이얼로그 | `confirm.*` | m700-3 |
| 25 | 글꼴·언어 | `font.*` | m700-2 |
| **26** | **머리말·꼬리말** | `header_footer.*` | *신규* — m700-1 |
| **27** | **각주·미주** | `footnote.*` | *신규* — m700-2 |
| **28** | **책갈피** | `bookmark.*` | *신규* — m700-2 |
| **29** | **찾기·바꾸기·찾아가기** | `find.*` | *신규* — m700-2 |
| **30** | **문자표·기호** | `charmap.*` | *신규* — m700-2 |
| **31** | **수식·계산** | `equation.*` | *신규* — m700-3 |
| **32** | **문서 비교·이력** | `compare.*` / `history.*` | *신규 — 가장 큰 자리* — m700-1 |
| **33** | **환경 설정** | `prefs.*` | *신규* — m700-3 |

## 다음 cycle (m700-1) 진입 자리

자리 분포 상위 6 카테고리 (Agent B 가 분석한 자리 분포 자료 자체):

| § | 카테고리 | 예상 자리 |
|---|---|---|
| 32 | `compare.*` / `history.*` (문서 비교·이력) | ~30 *(가장 큰 자리)* |
| 11 | `char_shape.*` (글자 모양) | ~25 |
| 13 | `table.*` (표·셀) | ~22 |
| 15 | `shape.*` (그림·도형) | ~18 |
| 26 | `header_footer.*` (머리말·꼬리말) | ~15 |
| 12 | `para_shape.*` (문단 모양) | ~15 |
| **합계** | | **~125 자리** |

m700-1 cycle 종결 시 약 *158 자리* 채움 예상 (33 + 125). 전체 385 자리 중 약 41% 진입.

## 사용자 결정 자리

- 자리 1, 2 (Phase 0 결정 + 카테고리 골격) — sub-agent 자리 검증 + 메인 자리 정렬 *완료*
- 자리 3 (m700-1 cycle 분량) — *승인*. 사용자 명시 자리.

m700-1 cycle 진입 자체는 *별도 cycle commit* 자리에서 박는다.
