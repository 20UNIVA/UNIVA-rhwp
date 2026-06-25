# rhwp-studio UI 문자열 매핑 테이블 (ko · en · ja)

이 문서는 *rhwp-studio* (워드프로세서 UI) 가 화면에 띄우는 *모든 사용자 가시 문자열*을 한국어·영어·일본어 3개 언어로 매핑한 *원본 사전*이다. *키 명명 규칙*은 vfinder 의 [key-naming-convention.md](../../../vfinder/docs/i18n/key-naming-convention.md) 두 단(`카테고리.역할`) 규칙 그대로 따른다.

**범위** — 사용자가 *눈으로 보는* 텍스트만. 코드 주석·`console.log`·assertion·테스트 fixture 자리는 제외.

**일본어 표기 원칙** — 한컴 한글·MS Word 일본어판 관용. 정중체(~です/~ます) 기본. 워드프로세서 어휘 (`削除`, `太字`, `斜体`, `下線`, `フォント`, `段落`, `表`, `セル`, `章`) 표준 채택.

**자료 수집 자리** — `rhwp-studio/src/**/*.ts(x)` 의 UI 가시 자리 (`textContent`/`setAttribute`/`placeholder`/`aria-label`/`showToast`/`.title`) + `index.html` 정적 텍스트. 합계 385개 unique 한국어 자리.

**카테고리 정합 흐름** — 1단계 cycle (m700-0) 에선 *골격*만 박고 자리별 채움은 m700-1~3 cycle 자리에서 카테고리별로 박는다.

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
| 11 | 글자 모양 대화상자 | `cs.*` | 글꼴·기준 크기·언어별·속성·확장·테두리/배경 |
| 12 | 문단 모양 대화상자 | `ps.*` | 정렬·간격·여백·줄간격·탭 |
| 13 | 표·셀 대화상자 | `table.*` | 표 만들기·셀 합치기/나누기·줄 추가/삭제 |
| 14 | 편집 용지·구역·격자 | `page.*` | 편집 용지·구역 설정·격자 설정 |
| 15 | 그림·도형 | `shape.*` | 도형 삽입·그림 자리 |
| 16 | 인쇄·내보내기 | `print.*` | 인쇄 대화상자·내보내기 |
| 17 | 우클릭 컨텍스트 메뉴 | `ctx.*` | 우클릭 자리 |
| 18 | 토스트·진행 표시 | `toast.*` | 토스트·progress |
| 19 | 공통 버튼 | `button.*` | 확인·취소·닫기·적용·기본값 |
| 20 | 빈 상태·안내 | `empty.*` | 데이터 없음 자리 |
| 21 | 시간 표기 | `time.*` | 상대 시간 (`방금 전`, `n분 전`) |
| 22 | 클라이언트 에러 | `error.client.*` | 클라이언트 측 에러 |
| 23 | 서버 에러 | `error.server.*` | rhwp-server (Rust) 에러 매핑 |
| 24 | 확인 다이얼로그 | `confirm.*` | `confirm()` 자리 |
| 25 | 글꼴·언어 자리 | `font.*` | 글꼴 이름·언어 라벨 |

---

## 1. 메뉴바 — 파일

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `menu.file.label` | 파일 | File | ファイル | 메뉴바 최상위 라벨 |
| `menu.file.new` | 새로 만들기 | New | 新規作成 | `file:new-doc` |
| `menu.file.open` | 열기 | Open | 開く | `file:open` |
| `menu.file.save` | 저장 | Save | 保存 | `file:save` |
| `menu.file.save_as` | 다른 이름으로 저장(A)... | Save As(A)... | 名前を付けて保存(A)... | `file:save-as` |
| `menu.file.page_setup` | 편집 용지 | Page Setup | 用紙設定 | `file:page-setup` |
| `menu.file.print` | 인쇄 | Print | 印刷 | `file:print` |
| `menu.file.about` | 제품 정보 | About | 製品情報 | `file:about` |

## 2. 메뉴바 — 편집

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `menu.edit.label` | 편집 | Edit | 編集 | 메뉴바 최상위 라벨 |
| `menu.edit.undo` | 되돌리기 | Undo | 元に戻す | `edit:undo` |
| `menu.edit.redo` | 다시 실행 | Redo | やり直し | `edit:redo` |
| `menu.edit.cut` | 오려 두기 | Cut | 切り取り | `edit:cut` |
| `menu.edit.copy` | 복사 | Copy | コピー | `edit:copy` |
| `menu.edit.paste` | 붙이기 | Paste | 貼り付け | `edit:paste` |
| `menu.edit.delete` | 지우기 | Delete | 削除 | `edit:delete` |
| `menu.edit.select_all` | 모두(A) | Select All(A) | すべて選択(A) | `edit:select-all` |
| `menu.edit.find` | 찾기 | Find | 検索 | `edit:find` |
| `menu.edit.replace` | 고치기(D) | Replace(D) | 置換(D) | `edit:replace` |

## 3. 메뉴바 — 보기

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `menu.view.label` | 보기 | View | 表示 | 메뉴바 최상위 라벨 |
| `menu.view.show_grid` | 격자 | Grid | グリッド | 격자 표시 토글 |
| `menu.view.show_paragraph_marks` | 문단 부호 | Paragraph Marks | 段落記号 | 조판부호 표시 |
| `menu.view.show_control_codes` | 조판부호 | Control Codes | 制御記号 | 조판부호 표시 |

## 4. 메뉴바 — 입력

*다음 cycle (m700-1) 자리에서 박는다.*

## 5. 메뉴바 — 서식

*다음 cycle 자리에서 박는다.*

## 6. 메뉴바 — 쪽

*다음 cycle 자리에서 박는다.*

## 7. 메뉴바 — 표

*다음 cycle 자리에서 박는다.*

## 8. 도구 상자

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `toolbar.cut` | 오려 두기 | Cut | 切り取り | 도구 상자 버튼 |
| `toolbar.copy` | 복사 | Copy | コピー | 도구 상자 버튼 |
| `toolbar.paste` | 붙이기 | Paste | 貼り付け | 도구 상자 버튼 |
| `toolbar.undo` | 되돌리기 | Undo | 元に戻す | 도구 상자 버튼 |
| `toolbar.redo` | 다시 실행 | Redo | やり直し | 도구 상자 버튼 |
| `toolbar.char_shape` | 글자 모양 | Character | 文字書式 | 도구 상자 버튼 (cs 다이얼로그) |
| `toolbar.para_shape` | 문단 모양 | Paragraph | 段落書式 | 도구 상자 버튼 (ps 다이얼로그) |

## 9. 서식 도구 모음

*카테고리 11(글자 모양 대화상자)·12(문단 모양 대화상자)와 자리 일부 공유.*

## 10. 상태 표시줄

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `statusbar.page` | {current} / {total} 쪽 | {current} / {total} pages | {current} / {total} ページ | 쪽 표시 |
| `statusbar.section` | 구역: {current} / {total} | Section: {current} / {total} | 区分: {current} / {total} | 구역 표시 |
| `statusbar.insert_mode` | 삽입 | Insert | 挿入 | 입력 모드 |
| `statusbar.overwrite_mode` | 수정 | Overwrite | 上書き | 입력 모드 |

## 11~25. 후속 카테고리

*m700-1, m700-2, m700-3 cycle 자리에서 박는다.* 카테고리 골격·자리는 위 목록 자리에서 정해 두었다.

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

---

## 자료 수집 자리

자료 자리 (`/tmp/m700-i18n/`):
- `ts_literals.txt` — 모든 한국어 리터럴 (1321) — *console.log·assertion 자리 포함*
- `visible_ts.txt` — UI 가시 자리 (201) — `textContent`/`setAttribute`/`placeholder`/`aria-label`/`showToast`/`.title` 자리
- `visible_html.txt` — index.html 정적 텍스트 (198)
- `visible_all.txt` — unique 합계 (385)

---

## 다음 cycle (m700-1) 진입 자리

1. 카테고리 4-7 (메뉴바 입력·서식·쪽·표) 채우기
2. 카테고리 11 (글자 모양 대화상자) 채우기
3. 카테고리 12 (문단 모양 대화상자) 채우기
4. 카테고리 13 (표·셀) 채우기
