# rhwp-studio UI 문자열 매핑 테이블 (ko · en · ja)

이 문서는 *rhwp-studio* (한컴 한글 풍 워드프로세서 UI) 가 화면에 띄우는 *모든 사용자 가시 문자열*을 한국어·영어·일본어 3개 언어로 매핑한 *원본 사전*이다. *키 명명 규칙*은 vfinder 의 [key-naming-convention.md](../../../vfinder/docs/i18n/key-naming-convention.md) 두 단(`카테고리.역할`) 규칙 그대로 따른다 — 메뉴바 자리만 *3 단* (`menu.file.save`) 예외 자리.

**범위** — 사용자가 *눈으로 보는* 텍스트만. 코드 주석·`console.log`·assertion·테스트 fixture 자리는 제외.

**일본어 표기 원칙** — 한컴 한글 일본어판 + MS Word 일본어판 관용. 정중체(~です/~ます) 기본. 워드프로세서 어휘 (`削除`, `太字`, `斜体`, `下線`, `フォント`, `段落`, `表`, `セル`) 채택. 한컴 한글 풍이라 *バージョン情報*·*組版記号* 같은 한컴 일본어판 표기를 MS Word 보다 우선.

**한국어 표기 원칙** — *한컴 한글 한국어판* 라벨 자리 그대로. *되돌리기*(undo), *오려 두기*(cut), *복사하기*(메뉴) / *복사*(도구상자), *모두 선택*(select all), *찾기*·*찾아 바꾸기*·*다시 찾기*, *수정*(overwrite mode 자리), *제품 정보*(about) 자리.

**영어 표기 원칙** — 단축키 라벨 `(A)`·`(D)` 자리는 *영어판에서는 자리 제거* (Alt-mnemonic 자체로 박힘). 한국어·일본어판 한컴 자리는 `(A)` 자리 유지.

**자료 수집 자리** — `rhwp-studio/src/**/*.ts(x)` 의 UI 가시 자리 (`textContent`/`setAttribute`/`placeholder`/`aria-label`/`showToast`/`.title`) + `index.html` 정적 텍스트. 합계 385개 unique 한국어 자리.

---

## 카테고리 목록

| § | 카테고리 | prefix | UI 자리 |
|---|---|---|---|
| 1 | 메뉴바 — 파일 | `menu.file.*` | `#menu-bar` 파일 드롭다운 |
| 2 | 메뉴바 — 편집 | `menu.edit.*` | `#menu-bar` 편집 드롭다운 |
| 3 | 메뉴바 — 보기 | `menu.view.*` | `#menu-bar` 보기 드롭다운 |
| 4 | 메뉴바 — 입력 | `menu.insert.*` | `#menu-bar` 입력 드롭다운 |
| 5 | 메뉴바 — 서식 | `menu.format.*` | `#menu-bar` 서식 드롭다운 |
| 6 | 메뉴바 — 쪽 | `menu.page.*` | `#menu-bar` 쪽 드롭다운 |
| 7 | 메뉴바 — 표 | `menu.table.*` | `#menu-bar` 표 드롭다운 |
| 8 | 도구 상자 | `toolbar.*` | `#icon-toolbar` 아이콘+라벨 |
| 9 | 서식 도구 모음 | `stylebar.*` | `#style-bar` 스타일·글꼴·정렬 |
| 10 | 상태 표시줄 | `statusbar.*` | `#status-bar` 쪽·구역·줌 |
| 11 | 글자 모양 대화상자 | `char_shape.*` | 글꼴·기준 크기·언어별·속성·확장·테두리/배경 |
| 12 | 문단 모양 대화상자 | `para_shape.*` | 정렬·간격·여백·줄간격·탭 |
| 13 | 표·셀 대화상자 | `table.*` | 표 만들기·셀 합치기/나누기·줄 추가/삭제 |
| 14 | 편집 용지·구역·격자 | `page.*` | 편집 용지·구역 설정·격자 설정 |
| 15 | 그림·도형 | `shape.*` | 도형 삽입·그림 자리 |
| 16 | 인쇄·내보내기 | `print.*` | 인쇄 대화상자·내보내기·진행 토스트 |
| 17 | 우클릭 컨텍스트 메뉴 | `context_menu.*` | 우클릭 자리 |
| 18 | 토스트·진행 표시 | `toast.*` | 토스트·progress |
| 19 | 공통 버튼 | `button.*` | 확인·취소·닫기·적용·기본값 |
| 20 | 빈 상태·안내 | `empty.*` | 데이터 없음 자리 |
| 21 | 시간 표기 | `time.*` | 상대 시간 (`방금 전`, `n분 전`) |
| 22 | 클라이언트 에러 | `error.client.*` | 클라이언트 측 에러 |
| 23 | 서버 에러 | `error.server.*` | rhwp-server (Rust) 에러 매핑 |
| 24 | 확인 다이얼로그 | `confirm.*` | `confirm()` 자리 |
| 25 | 글꼴·언어 자리 | `font.*` | 글꼴 이름·언어 라벨 |
| **26** | **머리말·꼬리말** | `header_footer.*` | *신규* — 머리말·꼬리말 편집·쪽 번호 템플릿 11종 |
| **27** | **각주·미주** | `footnote.*` | *신규* — 각주·미주 대화상자 |
| **28** | **책갈피** | `bookmark.*` | *신규* — 책갈피 목록·이름·이동 |
| **29** | **찾기·바꾸기·찾아가기** | `find.*` | *신규* — 찾기·찾아 바꾸기·다시 찾기·찾아가기 대화상자 |
| **30** | **문자표·기호** | `charmap.*` | *신규* — 문자표·유니코드·최근 사용한 문자 |
| **31** | **수식·계산** | `equation.*` | *신규* — 수식 편집·블록 계산식·자릿점 |
| **32** | **문서 비교·이력** | `compare.*` / `history.*` | *신규 — 가장 큰 자리* — 왼쪽/오른쪽 문서·스냅샷·비교 결과 |
| **33** | **환경 설정** | `prefs.*` | *신규* — 환경 설정 대화상자 |

추가로 카테고리에 *흡수*되는 자리:
- 하이퍼링크 (`menu.insert.hyperlink.*` 자리, 단일 자리라 §4 흡수)
- 모양 복사 (`menu.edit.format_copy.*`, §2 흡수)
- 글머리표·문단 번호 (`menu.format.numbering.*`, §5 흡수)
- 캡션 (`menu.insert.caption.*`, §4 흡수)

---

## 1. 메뉴바 — 파일

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `menu.file.label` | 파일 | File | ファイル | 메뉴바 최상위 라벨 |
| `menu.file.new` | 새로 만들기 | New | 新規作成 | `file:new-doc` |
| `menu.file.open` | 열기 | Open | 開く | `file:open` |
| `menu.file.save` | 저장 | Save | 保存 | `file:save` |
| `menu.file.save_as` | 다른 이름으로 저장(A)... | Save As... | 名前を付けて保存(A)... | `file:save-as` — 영어판 단축키 라벨 자리 제거 |
| `menu.file.page_setup` | 편집 용지 | Page Setup | 用紙設定 | `file:page-setup` |
| `menu.file.print` | 인쇄 | Print | 印刷 | `file:print` |
| `menu.file.about` | 제품 정보 | About | バージョン情報 | `file:about` — 일본어판 한컴 자리 *バージョン情報* |

## 2. 메뉴바 — 편집

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `menu.edit.label` | 편집 | Edit | 編集 | 메뉴바 최상위 라벨 |
| `menu.edit.undo` | 되돌리기 | Undo | 元に戻す | `edit:undo` — 한컴 한국어판 *되돌리기* |
| `menu.edit.redo` | 다시 실행 | Redo | やり直し | `edit:redo` |
| `menu.edit.cut` | 오려 두기 | Cut | 切り取り | `edit:cut` — 한컴 한국어판 *오려 두기* |
| `menu.edit.copy` | 복사하기 | Copy | コピー | `edit:copy` — *메뉴* 자리 (도구상자는 *복사* §8) |
| `menu.edit.paste` | 붙이기 | Paste | 貼り付け | `edit:paste` |
| `menu.edit.delete` | 지우기 | Delete | 削除 | `edit:delete` |
| `menu.edit.select_all` | 모두 선택 | Select All | すべて選択 | `edit:select-all` — 한국어 *"모두"* 단독 자리 정정 |
| `menu.edit.find` | 찾기 | Find | 検索 | `edit:find` |
| `menu.edit.find_replace` | 찾아 바꾸기(E) | Find & Replace | 検索/置換 | `edit:replace` — 키 자체 `replace` → `find_replace` 자리 갈람 |
| `menu.edit.find_again` | 다시 찾기(X) | Find Next | 次を検索 | `edit:find-again` *(신규)* |
| `menu.edit.format_copy` | 모양 복사 | Format Painter | 書式コピー | `edit:format-copy` *(신규)* |

## 3. 메뉴바 — 보기

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `menu.view.label` | 보기 | View | 表示 | 메뉴바 최상위 라벨 |
| `menu.view.show_grid` | 격자 | Grid | グリッド | 격자 표시 토글 |
| `menu.view.show_paragraph_marks` | 문단 부호 | Paragraph Marks | 段落記号 | 조판부호 표시 — ↵/↓ 마커 자리 |
| `menu.view.show_control_codes` | 조판 부호 | Control Codes | 組版記号 | 모든 컨트롤 마커 자리 — 일본어 *組版記号* 정정 |

## 4. 메뉴바 — 입력

*다음 cycle (m700-1) 자리에서 박는다.* 자리 후보: 그림·도형·표·각주·미주·하이퍼링크·수식·문자표·캡션.

## 5. 메뉴바 — 서식

*다음 cycle 자리에서 박는다.* 자리 후보: 스타일·글자 모양·문단 모양·글머리표·문단 번호.

## 6. 메뉴바 — 쪽

*다음 cycle 자리에서 박는다.* 자리 후보: 머리말·꼬리말·구역·쪽 번호·여백.

## 7. 메뉴바 — 표

*다음 cycle 자리에서 박는다.*

## 8. 도구 상자

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `toolbar.cut` | 오려 두기 | Cut | 切り取り | 도구 상자 버튼 |
| `toolbar.copy` | 복사 | Copy | コピー | 도구 상자 버튼 — *짧은 표기*. 메뉴 자리는 *복사하기* |
| `toolbar.paste` | 붙이기 | Paste | 貼り付け | 도구 상자 버튼 |
| `toolbar.undo` | 되돌리기 | Undo | 元に戻す | 도구 상자 버튼 |
| `toolbar.redo` | 다시 실행 | Redo | やり直し | 도구 상자 버튼 |
| `toolbar.char_shape` | 글자 모양 | Font | フォント | 도구 상자 버튼 — *Font* 자리. MS Word 관용 |
| `toolbar.para_shape` | 문단 모양 | Paragraph | 段落 | 도구 상자 버튼 |

## 9. 서식 도구 모음

*다음 cycle 자리에서 박는다.* §11 글자 모양 대화상자 자리와 *별 키*로 분리 (vfinder playbook "같은 한국어 맥락 다르면 키 분리" 정합).

## 10. 상태 표시줄

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `statusbar.page` | {current} / {total} 쪽 | {current} / {total} pages | {current} / {total} ページ | 쪽 표시 |
| `statusbar.section` | 구역: {current} / {total} | Section: {current} / {total} | 区分: {current} / {total} | 구역 표시 |
| `statusbar.insert_mode` | 삽입 | Insert | 挿入 | 입력 모드 |
| `statusbar.overwrite_mode` | 수정 | Overwrite | 上書き | 입력 모드 — 한컴 한국어판 *수정* |

## 11~33. 후속 카테고리

*m700-1, m700-2, m700-3 cycle 자리에서 박는다.*

m700-1 cycle 진입 시 박을 자리 (자리 분포 상위 + 자리 큰 덩어리 우선):

| 카테고리 | 예상 자리 수 | 우선 자리 |
|---|---|---|
| §32 `compare.*` / `history.*` (문서 비교·이력) | ~30 | *가장 큰 자리* |
| §11 `char_shape.*` (글자 모양) | ~25 | 진하게·기울임·밑줄·양각·음각·외곽선·첨자 |
| §13 `table.*` (표·셀) | ~22 | 셀 합치기/나누기·줄/칸·캡션·외곽선 |
| §15 `shape.*` (그림·도형) | ~18 | 도형·그림·글상자·회전·대칭 |
| §26 `header_footer.*` (머리말·꼬리말) | ~15 | 쪽 번호 템플릿 11종 |
| §12 `para_shape.*` (문단 모양) | ~15 | 정렬·줄간격·여백·탭 |
| §29 `find.*` (찾기·바꾸기) | ~12 | 찾기·바꾸기·찾아가기 |
| §25 `font.*` (글꼴·언어) | ~12 | 한글·영문·일어·한자 |
| §28 `bookmark.*` (책갈피) | ~10 | 책갈피 목록·이름 |
| §30 `charmap.*` (문자표) | ~8 | 문자표·유니코드 |

---

## placeholder 카탈로그

| placeholder | 뜻 | 예 |
|---|---|---|
| `{count}` | 항목 개수 | `{count}개 항목` |
| `{current}` | 현재 번호 | `{current} / {total} 쪽` |
| `{total}` | 전체 번호 | 동상 |
| `{name}` | 단일 이름 | 글꼴 이름 등 |
| `{path}` | 파일 경로 | 파일 다이얼로그 |
| `{value}` | 일반 값 | 옵션 값 |
| `{message}` | 에러 원문 | 서버 에러 |
| `{error}` | OS·시스템 에러 | rhwp-server 에러 |
| `{i}` | 진행 번호 | `인쇄 준비 중... ({i}/{total})` |

---

## 자료 수집 자리

자료 자리 (`/tmp/m700-i18n/`):
- `ts_literals.txt` — 모든 한국어 리터럴 (1321) — *console.log·assertion 자리 포함*
- `visible_ts.txt` — UI 가시 자리 (201) — `textContent`/`setAttribute`/`placeholder`/`aria-label`/`showToast`/`.title` 자리
- `visible_html.txt` — index.html 정적 텍스트 (198)
- `visible_all.txt` — unique 합계 (385)

---

## 검증 사이클 (m700-0 stage1 검증 자리)

sub-agent 두 자리로 *번역 적합성 + 카테고리 구조* 자리 자체를 검증 (m700-0 cycle 안). 결과:

| 자리 | 검증 결과 자리 | 처리 |
|---|---|---|
| 누락 카테고리 자리 | 8 자리 발견 (`header_footer`/`footnote`/`bookmark`/`find`/`charmap`/`equation`/`compare`+`history`/`prefs`) | ✅ 25 → 33 자리 확장 |
| prefix 약어 자리 | `cs.*`/`ps.*`/`ctx.*` 3 자리 자리 깨짐 | ✅ `char_shape.*`/`para_shape.*`/`context_menu.*` 풀어쓰기 |
| 한국어 *모두(A)* 잘못 | `모두 선택` 정합 | ✅ 자리 정정 |
| 한국어 *복사* vs *복사하기* 자리 | 메뉴(`복사하기`) / 도구상자(`복사`) 자리 분리 | ✅ 자리 분리 |
| `find` + `replace` 키 자리 | `고치기(D)` 잘못 → `찾아 바꾸기` + `다시 찾기` 분리 | ✅ 자리 정정 + 키 자체 갈람 |
| 일본어 *制御記号* 자리 | 한컴 일본어판 *組版記号* 정합 | ✅ 자리 정정 |
| 영어 *Character* / *Paragraph* 자리 부족 | MS Word 관용 *Font* / *Paragraph* | ✅ 자리 정정 |
| 일본어 *製品情報* 자리 | 한컴 일본어판 *バージョン情報* | ✅ 자리 정정 |
| 영어판 단축키 `(A)` 자리 | 영어판 자리 제거 (mnemonic 자체) | ✅ 자리 정정 |

---

## 다음 cycle (m700-1) 진입 자리

1. §32 `compare.*` / `history.*` (가장 큰 자리, ~30)
2. §11 `char_shape.*` (~25)
3. §13 `table.*` (~22)
4. §15 `shape.*` (~18)
5. §26 `header_footer.*` (~15)
6. §12 `para_shape.*` (~15)

예상 자리 채움: 약 *125 자리*. m700-1 cycle 종결 시 33 + 125 = 약 158 자리. 전체 385 자리 중 약 41%.
