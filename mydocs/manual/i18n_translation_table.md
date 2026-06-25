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

## 32. 문서 비교·이력 — `compare.*` / `history.*`

m700-1.1 sub-cycle 자리에서 박은 자리. compare-dialog·compare-result-window·history-dialog UI 가시 자리 자체.

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `compare.dialog_title` | 문서 비교 | Compare Documents | 文書比較 | dialog 타이틀 |
| `compare.detail_title` | 문서 비교 상세 | Compare Details | 文書比較の詳細 | 상세 창 라벨 |
| `compare.detail_title_pair` | 문서 비교 상세 · {left} ↔ {right} | Compare Details · {left} ↔ {right} | 文書比較の詳細 · {left} ↔ {right} | 상세 창 타이틀 (placeholder) |
| `compare.run` | 문서 비교 실행 | Run Compare | 比較を実行 | 실행 버튼 |
| `compare.run_with_current` | 선택과 현재 문서 비교 | Compare with Current | 選択と現在の文書を比較 | 현재 문서 비교 버튼 |
| `compare.in_progress` | 비교 중... | Comparing... | 比較中... | 진행 토스트 |
| `compare.calculating` | 비교 계산 중... | Calculating... | 計算中... | 진행 |
| `compare.no_run_yet` | 비교 실행 전 | Not yet run | 比較未実行 | 빈 상태 |
| `compare.result` | 비교 결과 | Compare Result | 比較結果 | 결과 영역 |
| `compare.failed` | 비교 실패: {message} | Compare failed: {message} | 比較失敗: {message} | 에러 |
| `compare.run_first` | 먼저 문서 비교를 실행해 결과를 생성하세요. | Run a compare first to generate results. | まず文書比較を実行して結果を生成してください。 | 빈 상태 안내 |
| `compare.select_both` | 왼쪽/오른쪽 문서를 모두 선택하세요. | Select both documents. | 左右の文書を選択してください。 | 검증 |
| `compare.select_snapshot` | 비교할 스냅샷을 목록에서 선택하세요. | Select a snapshot from the list. | リストから比較するスナップショットを選択してください。 | 검증 |
| `compare.next_diff` | 다음 차이 | Next Diff | 次の差分 | 네비 버튼 |
| `compare.prev_diff` | 이전 차이 | Previous Diff | 前の差分 | 네비 버튼 |
| `compare.case_sensitive` | 영문 대소문자 구분 | Case-sensitive | 大文字小文字を区別 | 옵션 체크박스 |
| `compare.left_doc` | 왼쪽 문서 | Left document | 左の文書 | 라벨 |
| `compare.right_doc` | 오른쪽 문서 | Right document | 右の文書 | 라벨 |
| `compare.left_name` | 왼쪽 문서: {name} | Left: {name} | 左: {name} | 라벨 (placeholder) |
| `compare.right_name` | 오른쪽 문서: {name} | Right: {name} | 右: {name} | 라벨 (placeholder) |
| `compare.left_loading` | 왼쪽 문서 로딩 중... | Loading left document... | 左の文書を読み込み中... | 진행 |
| `compare.right_loading` | 오른쪽 문서 로딩 중... | Loading right document... | 右の文書を読み込み中... | 진행 |
| `compare.left_ready` | 왼쪽 문서 페이지 준비 완료 | Left document ready | 左の文書ページ準備完了 | 완료 |
| `compare.right_ready` | 오른쪽 문서 페이지 준비 완료 | Right document ready | 右の文書ページ準備完了 | 완료 |
| `compare.no_doc_loaded` | 문서가 아직 로드되지 않았습니다. | No document is loaded yet. | 文書がまだ読み込まれていません。 | 에러 |
| `compare.no_current_doc` | 현재 문서가 없습니다. 문서를 연 뒤 다시 시도하세요. | No current document. Open a document and try again. | 現在の文書がありません。文書を開いてからもう一度お試しください。 | 에러 |
| `compare.page_render_failed` | 페이지 렌더 실패: {message} | Failed to render page: {message} | ページの描画に失敗しました: {message} | 에러 |
| `compare.page_load_failed` | 페이지 로드 실패: {message} | Failed to load page: {message} | ページの読み込みに失敗しました: {message} | 에러 |
| `compare.page_preparing` | 페이지 준비 중... | Preparing page... | ページを準備中... | 진행 |
| `compare.text_change` | 텍스트 변경 | Text change | テキスト変更 | diff 라벨 |
| `compare.property_change` | 속성 변경 | Property change | プロパティ変更 | diff 라벨 |
| `history.snapshot` | 스냅샷 | Snapshot | スナップショット | 라벨 |
| `history.saved` | 스냅샷을 저장했습니다. | Snapshot saved. | スナップショットを保存しました。 | 토스트 |
| `history.save_failed` | 저장 실패: {message} | Save failed: {message} | 保存失敗: {message} | 에러 |
| `history.note_placeholder` | 메모 (비우면 시각 기본값) | Note (default if empty) | メモ (空欄なら既定) | input placeholder |
| `history.saved_list` | 저장된 이력 (클릭하여 선택) | Saved history (click to select) | 保存履歴 (クリックで選択) | 라벨 |
| `history.save_current` | 현재 문서 저장 | Save current | 現在の文書を保存 | 버튼 |
| `history.delete_selected` | 선택 삭제 | Delete selected | 選択を削除 | 버튼 |
| `history.clear_all` | 전체 비우기 | Clear all | すべて削除 | 버튼 |
| `history.confirm_clear` | 저장된 문서 이력을 모두 지울까요? | Clear all saved history? | 保存された文書履歴をすべて削除しますか? | confirm |
| `history.cleared` | 이력을 비웠습니다. | History cleared. | 履歴を削除しました。 | 토스트 |
| `history.deleted` | 삭제했습니다. | Deleted. | 削除しました。 | 토스트 |
| `history.select_to_delete` | 삭제할 항목을 목록에서 먼저 선택하세요. | Select an item to delete first. | まず削除する項目を選択してください。 | 검증 |
| `history.read_failed` | 스냅샷 데이터를 읽을 수 없습니다. | Cannot read snapshot data. | スナップショットデータを読み込めません。 | 에러 |

## 11. 글자 모양 대화상자 — `char_shape.*`

m700-1.2 sub-cycle 자리. char-shape-dialog.ts 의 *기본·확장·테두리/배경* 탭 자리 자체.

### 기본 탭

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `char_shape.dialog_title` | 글자 모양 | Character | 文字書式 | dialog 타이틀 |
| `char_shape.tab_basic` | 기본 | Basic | 基本 | 탭 |
| `char_shape.tab_extension` | 확장 | Extension | 拡張 | 탭 |
| `char_shape.tab_border` | 테두리/배경 | Border/Fill | 罫線/塗り | 탭 |
| `char_shape.language` | 언어(L): | Language(L): | 言語(L): | 라벨 |
| `char_shape.language_settings` | 언어별 설정 | Language-specific | 言語別設定 | 그룹 |
| `char_shape.font` | 글꼴(T): | Font(T): | フォント(T): | 라벨 |
| `char_shape.base_size` | 기준 크기(Z): | Base size(Z): | 基準サイズ(Z): | 라벨 |
| `char_shape.relative_size` | 상대 크기(B): | Relative size(B): | 相対サイズ(B): | 라벨 |
| `char_shape.width` | 장평(W): | Width(W): | 文字幅(W): | 라벨 |
| `char_shape.spacing` | 자간(P): | Spacing(P): | 字間(P): | 라벨 |
| `char_shape.position` | 글자 위치(E): | Position(E): | 文字位置(E): | 라벨 |
| `char_shape.preview_sample` | 한글Eng123漢字あいう※○ | Korean한글English123漢字あいう※○ | 한글Eng123漢字あいう※○ | 미리 보기 샘플 |
| `char_shape.text_color` | 글자 색(C): | Text color(C): | 文字色(C): | 라벨 |
| `char_shape.shade_color` | 음영 색(G): | Shade(G): | 文字の網かけ(G): | 라벨 |

### 언어 옵션

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `char_shape.lang.representative` | 대표 | Representative | 代表 | 라벨 |
| `char_shape.lang.korean` | 한글 | Korean | 韓国語 | 옵션 |
| `char_shape.lang.english` | 영문 | English | 英語 | 옵션 |
| `char_shape.lang.hanja` | 한자 | Hanja | 漢字 | 옵션 |
| `char_shape.lang.japanese` | 일어 | Japanese | 日本語 | 옵션 |
| `char_shape.lang.foreign` | 외국어 | Other | その他外国語 | 옵션 |
| `char_shape.lang.symbol` | 기호 | Symbol | 記号 | 옵션 |
| `char_shape.lang.user` | 사용자 | User-defined | ユーザー | 옵션 |
| `char_shape.font_source.local` | 로컬 글꼴 | Local fonts | ローカルフォント | 그룹 |
| `char_shape.font_source.web` | 웹 글꼴 | Web fonts | Web フォント | 그룹 |

### 속성

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `char_shape.attr.bold` | 굵게 | Bold | 太字 | 토글 |
| `char_shape.attr.italic` | 기울임 | Italic | 斜体 | 토글 |
| `char_shape.attr.underline` | 밑줄 | Underline | 下線 | 토글 |
| `char_shape.attr.strikethrough` | 취소선 | Strikethrough | 取り消し線 | 토글 |
| `char_shape.attr.outline` | 외곽선 | Outline | 縁取り | 토글 |
| `char_shape.attr.shadow` | 그림자 | Shadow | 影 | 토글 |
| `char_shape.attr.emboss` | 양각 | Emboss | 浮き出し | 토글 |
| `char_shape.attr.engrave` | 음각 | Engrave | 浮き彫り | 토글 |
| `char_shape.attr.superscript` | 위 첨자 | Superscript | 上付き | 토글 |
| `char_shape.attr.subscript` | 아래 첨자 | Subscript | 下付き | 토글 |
| `char_shape.attr.kerning` | 커닝(K) | Kerning(K) | カーニング(K) | 토글 |
| `char_shape.attr.fit_space` | 글꼴에 어울리는 빈칸(F) | Font-aware spaces(F) | フォントに合わせた空白(F) | 토글 |
| `char_shape.attr.emphasis_dot` | 강조점(E): | Emphasis mark(E): | 強調点(E): | 라벨 |

### 외곽선·그림자·밑줄 옵션

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `char_shape.outline.kind` | 종류(Y): | Type(Y): | 種類(Y): | 라벨 |
| `char_shape.underline.location` | 위치(L): | Location(L): | 位置(L): | 라벨 |
| `char_shape.underline.location_above` | 위 | Above | 上 | 옵션 |
| `char_shape.underline.location_below` | 아래 | Below | 下 | 옵션 |
| `char_shape.underline.shape` | 모양(S): | Shape(S): | 形状(S): | 라벨 |
| `char_shape.underline.color` | 색(C): | Color(C): | 色(C): | 라벨 |

### 테두리/배경 탭

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `char_shape.border.group` | 테두리 | Border | 罫線 | 그룹 |
| `char_shape.border.none` | 테두리 없음 | None | 罫線なし | 옵션 |
| `char_shape.border.kind` | 종류(Y): | Type(Y): | 種類(Y): | 라벨 |
| `char_shape.border.thickness` | 굵기(I): | Thickness(I): | 太さ(I): | 라벨 |
| `char_shape.border.color` | 색(B): | Color(B): | 色(B): | 라벨 |
| `char_shape.bg.group` | 배경 | Background | 背景 | 그룹 |
| `char_shape.bg.shape` | 모양(M): | Pattern shape(M): | 形状(M): | 라벨 |
| `char_shape.bg.face_color` | 면 색(Q): | Fill color(Q): | 塗りつぶしの色(Q): | 라벨 |
| `char_shape.bg.pattern_shape` | 무늬 모양(L): | Pattern(L): | 模様(L): | 라벨 |
| `char_shape.bg.pattern_color` | 무늬 색(P): | Pattern color(P): | 模様の色(P): | 라벨 |
| `char_shape.bg.color_none` | 색 없음 | No color | 色なし | 라디오 |
| `char_shape.bg.color_set` | 색 지정 | Set color | 色指定 | 라디오 |

### 선 종류 옵션 (공통)

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `char_shape.line.solid` | 실선 | Solid | 実線 | 선 종류 |
| `char_shape.line.dash` | 파선 | Dash | 破線 | 선 종류 |
| `char_shape.line.dot` | 점선 | Dot | 点線 | 선 종류 |
| `char_shape.line.dash_dot` | 일점쇄선 | Dash-dot | 一点鎖線 | 선 종류 |
| `char_shape.line.dash_dot_dot` | 이점쇄선 | Dash-dot-dot | 二点鎖線 | 선 종류 |
| `char_shape.line.double` | 이중선 | Double | 二重線 | 선 종류 |
| `char_shape.line.thick` | 굵은 선 | Thick | 太い線 | 선 종류 |
| `char_shape.line.continuous` | 연속(T) | Continuous(T) | 連続(T) | 옵션 |
| `char_shape.line.discontinuous` | 비연속(U) | Discontinuous(U) | 不連続(U) | 옵션 |

### 미리보기·기타

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `char_shape.preview.box` | 상자형 | Box | ボックス型 | 미리보기 자리 종류 |
| `char_shape.preview.grid` | 격자형 | Grid | グリッド型 | 미리보기 자리 종류 |
| `char_shape.preview.custom` | 사용자 정의 | Custom | カスタム | 미리보기 자리 종류 |
| `char_shape.misc.none` | 없음 | None | なし | 일반 옵션 |
| `char_shape.misc.misc` | 기타 | Other | その他 | 일반 옵션 |
| `char_shape.misc.basic` | 기본 | Default | 既定 | 일반 옵션 |
| `char_shape.misc.preview_letter` | 가 | Aa | あ | 미리보기 글자 (한국어 *가*, 영어 *Aa*, 일본어 *あ*) |
| `char_shape.misc.attribute` | 속성 | Attributes | 属性 | 그룹 |

## 12. 문단 모양 대화상자 — `para_shape.*`

m700-1.3 sub-cycle 자리. para-shape-dialog + para-shape-tab-builders 의 *기본·확장·탭·테두리/배경* 탭 자리 자체.

### 기본 탭

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `para_shape.dialog_title` | 문단 모양 | Paragraph | 段落 | dialog 타이틀 |
| `para_shape.alignment` | 정렬 방식 | Alignment | 配置 | 그룹 |
| `para_shape.align.left` | 왼쪽 정렬 | Left align | 左揃え | 옵션 |
| `para_shape.align.center` | 가운데 정렬 | Center align | 中央揃え | 옵션 |
| `para_shape.align.right` | 오른쪽 정렬 | Right align | 右揃え | 옵션 |
| `para_shape.align.justify` | 양쪽 정렬 | Justify | 両端揃え | 옵션 |
| `para_shape.align.distribute` | 배분 정렬 | Distribute | 均等割り付け | 옵션 |
| `para_shape.align.divide` | 나눔 정렬 | Divide | 分割 | 옵션 |
| `para_shape.vertical_align` | 세로 정렬(S): | Vertical align(S): | 縦の整列(S): | 라벨 |
| `para_shape.vertical_align.font_based` | 글꼴 기준 | Font-based | フォント基準 | 옵션 |
| `para_shape.vertical_align.top` | 위쪽 | Top | 上 | 옵션 |
| `para_shape.vertical_align.center` | 가운데 | Center | 中央 | 옵션 |
| `para_shape.vertical_align.bottom` | 아래쪽 | Bottom | 下 | 옵션 |

### 여백·간격

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `para_shape.margin.group` | 여백 | Margin | 余白 | 그룹 |
| `para_shape.margin.left` | 왼쪽(E): | Left(E): | 左(E): | 라벨 |
| `para_shape.margin.right` | 오른쪽(O): | Right(O): | 右(O): | 라벨 |
| `para_shape.indent.first_line` | 첫 줄 | First line | 1行目 | 들여쓰기 자리 |
| `para_shape.indent.indent` | 들여쓰기(A) | Indent(A) | 字下げ(A) | 옵션 |
| `para_shape.indent.hanging` | 내어쓰기(B) | Hanging(B) | ぶら下げ(B) | 옵션 |
| `para_shape.indent.margin_only` | 여백만 지정 | Margin only | 余白のみ指定 | 옵션 |
| `para_shape.spacing.group` | 간격 | Spacing | 間隔 | 그룹 |
| `para_shape.spacing.before` | 문단 위(U): | Before(U): | 段落前(U): | 라벨 |
| `para_shape.spacing.after` | 문단 아래(V): | After(V): | 段落後(V): | 라벨 |
| `para_shape.spacing.line_spacing` | 줄 간격(S): | Line spacing(S): | 行間(S): | 라벨 |
| `para_shape.spacing.font_aware_line` | 글꼴에 어울리는 줄 높이(H) | Font-aware line height(H) | フォントに合わせた行高(H) | 토글 |

### 줄 간격 종류

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `para_shape.line_spacing.percent` | 글자에 따라 | By character | 文字に合わせる | 줄 간격 종류 |
| `para_shape.line_spacing.fixed` | 고정 값 | Fixed | 固定値 | 줄 간격 종류 |
| `para_shape.line_spacing.minimum` | 최소 | Minimum | 最小 | 줄 간격 종류 |
| `para_shape.line_spacing.spacing` | 간격 | Spacing | 間隔 | 줄 간격 종류 |

### 미리보기

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `para_shape.preview.title` | 미리보기 | Preview | プレビュー | 라벨 |
| `para_shape.preview.sample` | 이것은 문단 미리보기입니다. 이렇게 문단의 정렬과 여백, 들여쓰기가 적용된 모습을 확인할 수 있습니다. | This is a paragraph preview showing alignment, margins, and indentation. | これは段落のプレビューです。配置·余白·字下げが反映された姿を確認できます。 | 미리보기 본문 |
| `para_shape.preview.hint_second_line` | 두 번째 줄은 보통 여백만 적용됩니다. | The second line typically applies only margins. | 2行目は通常、余白のみ適用されます。 | 미리보기 부연 |

### 확장 탭

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `para_shape.ext.group_type` | 문단 종류 | Paragraph type | 段落の種類 | 그룹 |
| `para_shape.ext.outline` | 개요 문단(U) | Outline paragraph(U) | アウトライン段落(U) | 옵션 |
| `para_shape.ext.bullet` | 글머리표 문단(B) | Bullet paragraph(B) | 箇条書き段落(B) | 옵션 |
| `para_shape.ext.numbering` | 번호 문단(M) | Numbered paragraph(M) | 番号付き段落(M) | 옵션 |
| `para_shape.ext.normal` | 보통(N) | Normal(N) | 通常(N) | 옵션 |
| `para_shape.ext.none` | 없음(O) | None(O) | なし(O) | 옵션 |
| `para_shape.ext.level` | 수준(L): | Level(L): | レベル(L): | 라벨 |
| `para_shape.ext.keep_with_next` | 다음 문단과 함께(N) | Keep with next(N) | 次の段落と一緒(N) | 토글 |
| `para_shape.ext.page_break_before` | 문단 앞에서 항상 쪽 나눔(E) | Page break before(E) | 段落前で常に改ページ(E) | 토글 |
| `para_shape.ext.widow_orphan` | 외톨이줄 보호(K) | Widow/Orphan control(K) | 行末·行頭の禁則(K) | 토글 |
| `para_shape.ext.protect` | 문단 보호(P) | Paragraph protect(P) | 段落保護(P) | 토글 |
| `para_shape.ext.single_line` | 한 줄로 입력(W) | Single line(W) | 1行で入力(W) | 토글 |

### 줄 나눔 기준

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `para_shape.line_break.group` | 줄 나눔 기준 | Line break basis | 改行基準 | 그룹 |
| `para_shape.line_break.korean` | 한글(K): | Korean(K): | 韓国語(K): | 라벨 |
| `para_shape.line_break.english` | 영어(E): | English(E): | 英語(E): | 라벨 |
| `para_shape.line_break.character` | 글자 | Character | 文字 | 옵션 |
| `para_shape.line_break.word` | 단어 | Word | 単語 | 옵션 |
| `para_shape.line_break.eojeol` | 어절 | Eojeol | 文節 | 옵션 |
| `para_shape.line_break.hyphen` | 하이픈 | Hyphen | ハイフン | 옵션 |
| `para_shape.line_break.auto_korean_number` | 한글과 숫자 간격을 자동 조절(R) | Auto-adjust Korean & numbers(R) | 韓国語と数字の間隔を自動調整(R) | 토글 |
| `para_shape.line_break.auto_korean_english` | 한글과 영어 간격을 자동 조절(G) | Auto-adjust Korean & English(G) | 韓国語と英語の間隔を自動調整(G) | 토글 |

### 탭 설정

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `para_shape.tab.title` | 탭 설정 | Tab settings | タブ設定 | 라벨 |
| `para_shape.tab.position` | 탭 위치(P): | Tab position(P): | タブ位置(P): | 라벨 |
| `para_shape.tab.kind` | 탭 종류 | Tab kind | タブ種類 | 그룹 |
| `para_shape.tab.kind_left` | 왼쪽(L) | Left(L) | 左(L) | 옵션 |
| `para_shape.tab.kind_center` | 가운데(C) | Center(C) | 中央(C) | 옵션 |
| `para_shape.tab.kind_right` | 오른쪽(R) | Right(R) | 右(R) | 옵션 |
| `para_shape.tab.kind_decimal` | 소수점(M) | Decimal(M) | 小数点(M) | 옵션 |
| `para_shape.tab.fill` | 채움 모양(F): | Fill(F): | 区切り線(F): | 라벨 |
| `para_shape.tab.fill_none` | 없음 | None | なし | 옵션 |
| `para_shape.tab.fill_solid` | 실선 ───── | Solid ───── | 実線 ───── | 옵션 |
| `para_shape.tab.fill_dot` | 점선 ········· | Dot ········· | 点線 ········· | 옵션 |
| `para_shape.tab.fill_dash` | 긴 파선 ── ── ── | Long dash ── ── ── | 長破線 ── ── ── | 옵션 |
| `para_shape.tab.fill_circle` | 큰 동그라미 ○○○ | Large circles ○○○ | 大きい丸 ○○○ | 옵션 |
| `para_shape.tab.list_title` | 탭 목록 | Tab list | タブ一覧 | 라벨 |
| `para_shape.tab.removed_list` | 지운 탭 목록 | Removed tabs | 削除済みタブ | 라벨 |
| `para_shape.tab.add` | 추가(S) | Add(S) | 追加(S) | 버튼 |
| `para_shape.tab.delete_selected` | 선택 삭제 | Delete selected | 選択を削除 | 버튼 |
| `para_shape.tab.delete_all` | 전체 삭제 | Delete all | すべて削除 | 버튼 |
| `para_shape.tab.toggle_all` | 모두 적용/해제 | Apply/Reset all | すべて適用/解除 | 버튼 |
| `para_shape.tab.double_click_restore` | 더블클릭하여 복원 | Double-click to restore | ダブルクリックで復元 | 안내 |
| `para_shape.tab.section_default` | 구역 기본 탭 간격: | Section default tab: | 区分の既定タブ間隔: | 라벨 |
| `para_shape.tab.default_tab` | 기본 탭 | Default tab | 既定タブ | 라벨 |
| `para_shape.tab.auto_tab` | 자동 탭 | Auto tab | 自動タブ | 그룹 |
| `para_shape.tab.auto_hanging` | 내어 쓰기용 자동 탭(E) | Auto hanging tab(E) | ぶら下げ用自動タブ(E) | 토글 |
| `para_shape.tab.auto_right_edge` | 문단 오른쪽 끝 자동 탭(I) | Auto right-edge tab(I) | 段落右端自動タブ(I) | 토글 |
| `para_shape.tab.change` | 변경(H)... | Change(H)... | 変更(H)... | 버튼 |

### 테두리/배경 탭 (공통 자리 — para_shape 자리 한정)

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `para_shape.border.all_apply` | 모두(A): | All(A): | すべて(A): | 라벨 |
| `para_shape.border.top` | 위쪽(U): | Top(U): | 上(U): | 라벨 |
| `para_shape.border.bottom` | 아래쪽(V): | Bottom(V): | 下(V): | 라벨 |
| `para_shape.border.left` | 왼쪽(E): | Left(E): | 左(E): | 라벨 |
| `para_shape.border.right` | 오른쪽(B): | Right(B): | 右(B): | 라벨 |
| `para_shape.line.long_dash_label` | 긴 파선 | Long dash | 長破線 | 선 종류 (구체) |
| `para_shape.line.long_dot_label` | 긴 점선 - - - - | Long dot - - - - | 長点線 - - - - | 선 종류 |
| `para_shape.line.thin_thick` | 가는선+굵은선 | Thin+Thick | 細+太 | 선 종류 |
| `para_shape.line.thick_thin` | 굵은선+가는선 | Thick+Thin | 太+細 | 선 종류 |
| `para_shape.line.triple` | 삼중선 | Triple | 三重線 | 선 종류 |
| `para_shape.line.wave` | 물결 | Wave | 波線 | 선 종류 |
| `para_shape.line.double_wave` | 이중 물결 | Double wave | 二重波線 | 선 종류 |
| `para_shape.line.circle_dot` | 동그라미 | Circle dot | 丸 | 선 종류 |
| `para_shape.line.thick_3d` | 두꺼운 3D | Thick 3D | 太い3D | 선 종류 |
| `para_shape.line.thick_3d_inv` | 두꺼운 3D(반대) | Thick 3D (inverted) | 太い3D(反転) | 선 종류 |

## 13. 표·셀 — `table.*`

m700-1.4 sub-cycle 자리. table-create-dialog + table-cell-props-dialog 자리.

### 표 만들기

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.create.title` | 표 만들기 | Insert Table | 表の挿入 | dialog 타이틀 |
| `table.create.title_with_ellipsis` | 표 만들기... | Insert Table... | 表の挿入... | 메뉴 라벨 |
| `table.create.lines_cols` | 줄/칸 | Rows/Columns | 行/列 | 그룹 |
| `table.create.lines` | 줄 개수 | Row count | 行数 | 라벨 |
| `table.create.cols` | 칸 개수 | Column count | 列数 | 라벨 |
| `table.create.size_spec` | 크기 지정 | Size | サイズ | 그룹 |
| `table.create.width` | 너비 | Width | 幅 | 라벨 |
| `table.create.height` | 높이 | Height | 高さ | 라벨 |
| `table.create.auto` | 자동 | Auto | 自動 | 옵션 |
| `table.create.column_fit` | 단에 맞춤 | Fit to column | 段に合わせる | 옵션 |
| `table.create.direct` | 직접 지정 | Custom | 直接指定 | 옵션 |
| `table.create.create_btn` | 만들기 | Create | 作成 | 버튼 |

### 셀 속성 — 공통

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.props.title` | 표/셀 속성 | Table/Cell properties | 表/セルのプロパティ | dialog 타이틀 |
| `table.props.cell_border_fill` | 셀 테두리/배경 | Cell border/fill | セルの罫線/塗り | 탭 |
| `table.tab.basic` | 기본 | Basic | 基本 | 탭 |
| `table.tab.border_fill` | 테두리/배경 | Border/Fill | 罫線/塗り | 탭 |
| `table.tab.margin_caption` | 여백/캡션 | Margin/Caption | 余白/キャプション | 탭 |

### 본문과의 배치 (Object placement)

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.placement.title` | 본문과의 배치 | Text wrapping | 本文との配置 | 그룹 |
| `table.placement.behind_text` | 글 뒤로 | Behind text | テキストの背面 | 옵션 |
| `table.placement.in_front` | 글 앞으로 | In front of text | テキストの前面 | 옵션 |
| `table.placement.wrap` | 어울림 | Square wrap | 折り返し | 옵션 |
| `table.placement.take_space` | 자리 차지 | Take space | 場所を取る | 옵션 |
| `table.placement.like_char` | 글자처럼 취급 | Treat as character | 文字として扱う | 옵션 |
| `table.placement.allow_overlap` | 서로 겹침 허용 | Allow overlap | 重なりを許可 | 토글 |
| `table.placement.keep_with_anchor` | 개체와 조판부호를 항상 같은 쪽에 놓기 | Keep object and anchor on same page | 開封符号と同じページに置く | 토글 |
| `table.placement.restrict_to_page` | 쪽 영역 안으로 제한 | Restrict to page area | ページ領域内に制限 | 토글 |
| `table.placement.expand_to_margin` | 여백 부분까지 너비 확대(W) | Expand to margin width(W) | 余白まで幅拡張(W) | 토글 |

### 위치·기준

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.pos.title` | 위치 | Position | 位置 | 그룹 |
| `table.pos.basis` | 기준 | Basis | 基準 | 그룹 |
| `table.pos.horizontal` | 가로 | Horizontal | 水平 | 라벨 |
| `table.pos.vertical` | 세로 | Vertical | 垂直 | 라벨 |
| `table.pos.paragraph` | 문단 | Paragraph | 段落 | 옵션 |
| `table.pos.column` | 단 | Column | 段 | 옵션 |
| `table.pos.page` | 쪽 | Page | ページ | 옵션 |
| `table.pos.paper` | 종이 | Paper | 用紙 | 옵션 |
| `table.pos.of` | 의 | of | の | 연결사 |
| `table.pos.left` | 왼쪽 | Left | 左 | 옵션 |
| `table.pos.right` | 오른쪽 | Right | 右 | 옵션 |
| `table.pos.top` | 위쪽 | Top | 上 | 옵션 |
| `table.pos.bottom` | 아래쪽 | Bottom | 下 | 옵션 |
| `table.pos.center` | 가운데 | Center | 中央 | 옵션 |
| `table.pos.outer` | 바깥쪽 | Outer | 外 | 옵션 |
| `table.pos.inner` | 안쪽 | Inner | 内 | 옵션 |
| `table.pos.up` | 위 | Up | 上 | 옵션 |
| `table.pos.down` | 아래 | Down | 下 | 옵션 |

### 셀 크기·여백

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.size.group` | 셀 크기 | Cell size | セルサイズ | 그룹 |
| `table.size.apply` | 셀 크기 적용 | Apply cell size | セルサイズを適用 | 토글 |
| `table.margin.outer` | 바깥 여백 | Outer margin | 外側の余白 | 그룹 |
| `table.margin.inner` | 안 여백 | Inner margin | 内側の余白 | 그룹 |
| `table.margin.inner_specify` | 안 여백 지정 | Specify inner margin | 内側の余白を指定 | 토글 |
| `table.margin.all_cells` | 모든 셀의 안 여백 | All cells' inner margin | すべてのセルの内側余白 | 토글 |
| `table.cell_spacing` | 셀 간격 | Cell spacing | セル間隔 | 라벨 |

### 셀 동작·보호

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.cell.protect` | 셀 보호 | Protect cell | セル保護 | 토글 |
| `table.cell.header_cell` | 제목 셀 | Header cell | 見出しセル | 토글 |
| `table.cell.title_repeat` | 제목 줄 자동 반복 | Auto-repeat header row | 見出し行を自動繰り返し | 토글 |
| `table.cell.form_editable` | 양식 모드에서 편집 가능 | Editable in form mode | フォームモードで編集可能 | 토글 |
| `table.cell.single_line` | 한 줄로 입력(S) | Single line(S) | 1行で入力(S) | 토글 |
| `table.cell.field_name` | 필드 이름 | Field name | フィールド名 | 라벨 |
| `table.cell.field` | 필드 | Field | フィールド | 라벨 |
| `table.cell.attribute` | 속성 | Attribute | プロパティ | 그룹 |

### 페이지 분할

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.split.multi_page` | 여러 쪽 지원 | Multi-page support | 複数ページ対応 | 토글 |
| `table.split.do_not_split` | 나누지 않음 | Do not split | 分割しない | 옵션 |
| `table.split.split` | 나눔 | Split | 分割 | 옵션 |
| `table.split.by_cell` | 셀 단위로 나눔 | Split by cell | セル単位で分割 | 옵션 |
| `table.split.at_page_boundary` | 쪽 경계에서(Q) | At page boundary(Q) | ページ境界で(Q) | 옵션 |
| `table.split.auto_border` | 자동 경계선 | Auto border | 自動境界線 | 토글 |
| `table.split.auto_border_split` | 자동으로 나뉜 표의 경계선 설정(J) | Auto-split border settings(J) | 自動分割表の境界線設定(J) | 토글 |

### 텍스트 방향 (셀 안)

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.text_dir.horizontal` | 가로쓰기 | Horizontal | 横書き | 옵션 |
| `table.text_dir.vertical` | 세로쓰기 | Vertical | 縦書き | 옵션 |
| `table.text_dir.rotate_text` | 문 눕힘(Q) | Lay text(Q) | 文字を寝かせる(Q) | 옵션 |
| `table.text_dir.upright_text` | 문 세움(U) | Upright text(U) | 文字を立てる(U) | 옵션 |
| `table.text_dir.vertical_align` | 세로 정렬 | Vertical align | 縦の整列 | 라벨 |
| `table.text_dir.rotate_angle` | 회전각 | Rotation | 回転角 | 라벨 |
| `table.text_dir.skew` | 기울이기 | Skew | 傾斜 | 라벨 |

### 테두리·배경

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.border.kind` | 종류(N) | Type(N) | 種類(N) | 라벨 |
| `table.border.line_kind` | 선 종류(Y) | Line type(Y) | 線種(Y) | 라벨 |
| `table.border.line_attr` | 선 속성 | Line attribute | 線の属性 | 그룹 |
| `table.border.thickness` | 굵기(H) | Thickness(H) | 太さ(H) | 라벨 |
| `table.border.apply_immediately` | 선 모양 바로 적용(I) | Apply immediately(I) | 線種をすぐ適用(I) | 토글 |
| `table.border.color` | 색(S) | Color(S) | 色(S) | 라벨 |
| `table.border.horizontal_line` | 가로줄 | Horizontal line | 横の罫線 | 그룹 |
| `table.border.vertical_line` | 세로줄 | Vertical line | 縦の罫線 | 그룹 |
| `table.border.slash` | 슬래시 | Slash | スラッシュ | 옵션 |
| `table.border.backslash` | 역슬래시 | Backslash | 円記号 | 옵션 |
| `table.border.cross` | 십자 | Cross | 十字 | 옵션 |
| `table.border.double_line` | 이중 | Double | 二重 | 옵션 |
| `table.border.double_solid` | 이중 실선 | Double solid | 二重実線 | 옵션 |

### 채우기

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.fill.title` | 채우기 | Fill | 塗りつぶし | 그룹 |
| `table.fill.background` | 배경 | Background | 背景 | 그룹 |
| `table.fill.gradient` | 그러데이션 | Gradient | グラデーション | 옵션 |
| `table.fill.face_color` | 면색(C) | Face color(C) | 面の色(C) | 라벨 |
| `table.fill.pattern_shape` | 무늬모양(L) | Pattern shape(L) | 模様(L) | 라벨 |
| `table.fill.pattern_color` | 무늬색(K) | Pattern color(K) | 模様の色(K) | 라벨 |
| `table.fill.image` | 그림 | Image | 画像 | 그룹 |
| `table.fill.image_file` | 그림 파일 | Image file | 画像ファイル | 라벨 |
| `table.fill.open_file` | 열기... | Open... | 開く... | 버튼 |

### 캡션·번호

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.caption.title` | 캡션 | Caption | キャプション | 그룹 |
| `table.caption.size` | 캡션 크기(S) | Caption size(S) | キャプションのサイズ(S) | 라벨 |
| `table.caption.number_kind` | 번호 종류 | Number type | 番号種類 | 라벨 |

### 그라데이션·그러데이션 형태

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `table.gradient.linear` | 선형 | Linear | 線形 | 옵션 |
| `table.gradient.radial` | 방사형 | Radial | 放射 | 옵션 |
| `table.gradient.conic` | 원뿔형 | Conic | 円錐 | 옵션 |
| `table.gradient.rectangular` | 사각형 | Rectangular | 矩形 | 옵션 |

## 15. 그림·도형 — `shape.*`

m700-1.5 sub-cycle 자리. picture-props-dialog + shape-picker 의 *개체 속성*·*도형 선택* 자리. *공통 자리* (위치·기준·테두리·채우기) 는 §13 `table.*` 자리 재사용 권장. *shape 특수 자리*만 박는다.

### 개체 속성 dialog 자리

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.dialog_title` | 개체 속성 | Object properties | オブジェクトのプロパティ | dialog 타이틀 |
| `shape.tab.basic` | 기본 | Basic | 基本 | 탭 |
| `shape.tab.picture` | 그림 | Image | 画像 | 탭 |
| `shape.tab.effects` | 그림 효과 | Effects | 効果 | 탭 |
| `shape.tab.fill` | 채우기 | Fill | 塗りつぶし | 탭 |
| `shape.tab.line` | 선 | Line | 線 | 탭 |
| `shape.tab.shadow` | 그림자 | Shadow | 影 | 탭 |
| `shape.tab.rotate_flip` | 개체 회전/대칭 | Rotate/Flip | 回転/反転 | 그룹 |

### 도형 선택 (shape-picker)

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.picker.title` | 도형 | Shapes | 図形 | 라벨 |
| `shape.picker.drawing` | 그리기 개체 | Drawing object | 描画オブジェクト | 그룹 |
| `shape.picker.line` | 직선 | Line | 直線 | 도형 |
| `shape.picker.curve` | 곡선 | Curve | 曲線 | 도형 |
| `shape.picker.polyline` | 꺾인 | Polyline | 折れ線 | 도형 |
| `shape.picker.polygon` | 다각형 | Polygon | 多角形 | 도형 |
| `shape.picker.rectangle` | 사각형 | Rectangle | 四角形 | 도형 |
| `shape.picker.ellipse` | 타원 | Ellipse | 楕円 | 도형 |
| `shape.picker.arc` | 호 | Arc | 円弧 | 도형 |
| `shape.picker.connector` | 연결선 | Connector | コネクタ | 도형 |
| `shape.picker.line_arrow` | 직선 화살표 | Straight arrow | 直線矢印 | 도형 |
| `shape.picker.curve_arrow` | 곡선 화살표 | Curved arrow | 曲線矢印 | 도형 |
| `shape.picker.polyline_arrow` | 꺾인 화살표 | Polyline arrow | 折れ線矢印 | 도형 |

### 크기·위치 (shape 자리 한정)

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.size.title` | 크기 | Size | サイズ | 그룹 |
| `shape.size.width` | 너비(W) | Width(W) | 幅(W) | 라벨 |
| `shape.size.height` | 높이(H) | Height(H) | 高さ(H) | 라벨 |
| `shape.size.locked` | 크기 고정(S) | Lock size(S) | サイズ固定(S) | 토글 |
| `shape.size.aspect_locked` | 가로 세로 같은 비율 유지 | Keep aspect ratio | 縦横比を維持 | 토글 |
| `shape.pos.horizontal_offset` | 가로 방향 이동(H): | Horizontal offset(H): | 水平移動(H): | 라벨 |
| `shape.pos.vertical_offset` | 세로 방향 이동(V): | Vertical offset(V): | 垂直移動(V): | 라벨 |
| `shape.pos.both` | 양쪽 | Both | 両方 | 옵션 |

### 회전·대칭

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.rotate.angle` | 회전각(E): | Rotation angle(E): | 回転角度(E): | 라벨 |
| `shape.rotate.flip_horizontal` | 좌우 대칭 | Flip horizontal | 左右反転 | 버튼 |
| `shape.rotate.flip_vertical` | 상하 대칭 | Flip vertical | 上下反転 | 버튼 |
| `shape.rotate.center_horizontal` | 가로 중심(W): | H-center(W): | 水平中心(W): | 라벨 |
| `shape.rotate.center_vertical` | 세로 중심(X): | V-center(X): | 垂直中心(X): | 라벨 |
| `shape.rotate.center_of` | 가운데에서 | From center | 中心から | 옵션 |

### 그림 특수 자리

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.picture.title` | 그림 | Image | 画像 | 그룹 |
| `shape.picture.file` | 그림 파일(I): | Image file(I): | 画像ファイル(I): | 라벨 |
| `shape.picture.filename` | 파일 이름 | File name | ファイル名 | 라벨 |
| `shape.picture.embed` | 문서에 포함 | Embed in document | 文書に埋め込む | 토글 |
| `shape.picture.embed_paren` | 문서에 포함(J) | Embed(J) | 文書に埋め込む(J) | 토글 |
| `shape.picture.crop` | 그림 자르기 | Crop | トリミング | 그룹 |
| `shape.picture.crop_margin` | 그림 여백 | Image margin | 画像の余白 | 그룹 |
| `shape.picture.reverse` | 그림 반전 | Reverse | 反転 | 토글 |
| `shape.picture.brightness` | 밝기 | Brightness | 明るさ | 라벨 |
| `shape.picture.brightness_label` | 밝기(H): | Brightness(H): | 明るさ(H): | 라벨 |
| `shape.picture.contrast` | 대비 | Contrast | コントラスト | 라벨 |
| `shape.picture.contrast_label` | 대비(I): | Contrast(I): | コントラスト(I): | 라벨 |
| `shape.picture.transparency` | 투명도 | Transparency | 透明度 | 라벨 |
| `shape.picture.transparency_label` | 투명도(I): | Transparency(I): | 透明度(I): | 라벨 |
| `shape.picture.transparency_set` | 투명도 설정 | Set transparency | 透明度を設定 | 토글 |
| `shape.picture.zoom_ratio` | 확대/축소 비율 | Zoom ratio | 拡大/縮小率 | 라벨 |
| `shape.picture.fit_size` | 크기에 맞추어 | Fit to size | サイズに合わせる | 옵션 |
| `shape.picture.tile_all` | 바둑판식으로-모두 | Tile - all | タイル状 - すべて | 옵션 |
| `shape.picture.from_original` | 원래 그림에서 | From original | 元の画像から | 옵션 |
| `shape.picture.to_original` | 원래 그림으로 | To original | 元の画像へ | 옵션 |
| `shape.picture.original_size` | 원래 크기로 | Original size | 元のサイズ | 옵션 |
| `shape.picture.grayscale` | 회색조 | Grayscale | グレースケール | 옵션 |
| `shape.picture.black_white` | 흑백 | Black & white | モノクロ | 옵션 |
| `shape.picture.watermark` | 워터마크 효과 | Watermark effect | 透かし効果 | 옵션 |
| `shape.picture.watermark_paren` | 워터마크 효과(M) | Watermark(M) | 透かし(M) | 옵션 |

### 그림 효과 (네온·반사·열은 테두리)

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.effects.title` | 그림 효과(E): | Effects(E): | 効果(E): | 라벨 |
| `shape.effects.glow` | 네온 | Glow | 光彩 | 효과 |
| `shape.effects.glow_none` | 네온 없음 | No glow | 光彩なし | 효과 |
| `shape.effects.glow_effect` | 네온 효과 | Glow effect | 光彩効果 | 효과 |
| `shape.effects.reflection` | 반사 | Reflection | 反射 | 효과 |
| `shape.effects.reflection_none` | 반사 없음 | No reflection | 反射なし | 효과 |
| `shape.effects.reflection_effect` | 반사 효과 | Reflection effect | 反射効果 | 효과 |
| `shape.effects.soft_edge` | 열은 테두리 | Soft edge | ぼかし | 효과 |
| `shape.effects.soft_edge_none` | 열은 테두리 없음 | No soft edge | ぼかしなし | 효과 |
| `shape.effects.soft_edge_effect` | 열은 테두리 효과 | Soft edge effect | ぼかし効果 | 효과 |
| `shape.effects.no_effect` | 효과 없음 | No effect | 効果なし | 효과 |
| `shape.effects.shadow_color` | 그림자 색(C): | Shadow color(C): | 影の色(C): | 라벨 |
| `shape.effects.blur_distance` | 번짐 정도(Z): | Blur(Z): | ぼかし(Z): | 라벨 |
| `shape.effects.distance` | 거리 | Distance | 距離 | 라벨 |

### 선 끝 모양·시작 모양 (화살표 등)

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.line.thickness` | 굵기(T): | Thickness(T): | 太さ(T): | 라벨 |
| `shape.line.kind` | 종류(L): | Type(L): | 種類(L): | 라벨 |
| `shape.line.start_shape` | 시작 모양(S): | Start shape(S): | 始点形状(S): | 라벨 |
| `shape.line.start_color` | 시작 색(G): | Start color(G): | 始点の色(G): | 라벨 |
| `shape.line.start_size` | 시작 크기(Z): | Start size(Z): | 始点サイズ(Z): | 라벨 |
| `shape.line.end_shape` | 끝 모양(Y): | End shape(Y): | 終点形状(Y): | 라벨 |
| `shape.line.end_shape_e` | 끝 모양(E): | End shape(E): | 終点形状(E): | 라벨 |
| `shape.line.end_color` | 끝 색(E): | End color(E): | 終点の色(E): | 라벨 |
| `shape.line.end_size` | 끝 크기(N): | End size(N): | 終点サイズ(N): | 라벨 |
| `shape.line.arrow.tail` | 꼬리 화살표 | Tail arrow | 尾矢印 | 옵션 |
| `shape.line.arrow.open` | 열린 화살표 | Open arrow | 開放矢印 | 옵션 |
| `shape.line.arrow.arrow` | 화살표 | Arrow | 矢印 | 옵션 |
| `shape.line.thickness_inside` | 선 굵기 내부 적용(K) | Apply thickness inside(K) | 線の太さを内側に適用(K) | 토글 |
| `shape.line.cap.round` | 둥근 | Round | 丸 | 옵션 |
| `shape.line.cap.flat` | 평면 | Flat | 平面 | 옵션 |
| `shape.line.long_dash` | 긴 파선 | Long dash | 長破線 | 선 종류 |
| `shape.line.solid_simple` | 실선 | Solid | 実線 | 선 종류 |

### 사각형 모서리·도형 모양

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.rect.corner_radius` | 사각형 모서리 곡률 | Rectangle corner radius | 角丸の半径 | 라벨 |
| `shape.rect.corner_round` | 둥근 모양(O) | Rounded(O) | 角丸(O) | 옵션 |
| `shape.rect.corner_right_angle` | 직각(G) | Right angle(G) | 直角(G) | 옵션 |
| `shape.arc.shape` | 호(A) | Arc(A) | 円弧(A) | 옵션 |
| `shape.arc.fan` | 부채꼴(B) | Pie(B) | 扇形(B) | 옵션 |
| `shape.arc.half_circle` | 반원(M) | Half circle(M) | 半円(M) | 옵션 |
| `shape.arc.bow` | 활 모양(I) | Bow(I) | 弓型(I) | 옵션 |
| `shape.arc.border` | 호 테두리 | Arc border | 弧の罫線 | 옵션 |

### 그림자

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.shadow.title` | 그림자 | Shadow | 影 | 그룹 |
| `shape.shadow.none` | 효과 없음 | No effect | 効果なし | 옵션 |
| `shape.shadow.narcis` | 나르시스 | Narcissus | ナルキッソス | 옵션 |
| `shape.shadow.snail` | 소라 | Conch | サザエ | 옵션 |
| `shape.shadow.classic` | 클래식 | Classic | クラシック | 옵션 |
| `shape.shadow.rhombus` | 마름모 | Rhombus | ひし形 | 옵션 |
| `shape.shadow.diagonal_left_top` | 왼쪽 위 | Left-top | 左上 | 옵션 |
| `shape.shadow.diagonal_right_top` | 오른쪽 위 | Right-top | 右上 | 옵션 |
| `shape.shadow.diagonal_left_bottom` | 왼쪽 아래 | Left-bottom | 左下 | 옵션 |
| `shape.shadow.diagonal_right_bottom` | 오른쪽 아래 | Right-bottom | 右下 | 옵션 |
| `shape.shadow.diagonal_left` | 왼쪽 대각선 | Left diagonal | 左斜め | 옵션 |
| `shape.shadow.diagonal_right` | 오른쪽 대각선 | Right diagonal | 右斜め | 옵션 |
| `shape.shadow.diagonal_1` | 대각선1 | Diagonal 1 | 斜め1 | 옵션 |
| `shape.shadow.diagonal_2` | 대각선2 | Diagonal 2 | 斜め2 | 옵션 |
| `shape.shadow.horizontal_line` | 수평선 | Horizontal line | 水平線 | 옵션 |
| `shape.shadow.vertical_line` | 수직선 | Vertical line | 垂直線 | 옵션 |
| `shape.shadow.horizontal` | 수평 | Horizontal | 水平 | 옵션 |

### 글상자·문 자리

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.textbox.title` | 글상자 | Text box | テキストボックス | 그룹 |
| `shape.textbox.margin` | 글상자 여백 | Text box margin | テキストボックスの余白 | 그룹 |
| `shape.textbox.body_position` | 본문 위치(P): | Body position(P): | 本文位置(P): | 라벨 |
| `shape.textbox.fit_text_with_box` | 글에 어울리는 줄 자리 | (legacy) | (legacy) | 임시 자리 |
| `shape.textbox.vertical_writing` | 세로쓰기(E): | Vertical writing(E): | 縦書き(E): | 라벨 |
| `shape.textbox.text_horizontal` | 영문 눕힘(O) | Lay English(O) | 英文寝かせ(O) | 옵션 |
| `shape.textbox.text_vertical` | 영문 세움(U) | Upright English(U) | 英文立て(U) | 옵션 |
| `shape.textbox.body_align_top` | 위 | Top | 上 | 옵션 |
| `shape.textbox.body_align_bottom` | 아래 | Bottom | 下 | 옵션 |

### 도형 채우기 종류

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.fill.type` | 채우기 유형(S): | Fill type(S): | 塗りつぶしの種類(S): | 라벨 |
| `shape.fill.spread` | 빈 공간 채움 | Fill empty space | 空欄を埋める | 토글 |
| `shape.fill.reverse_center` | 반전 중심(N): | Reverse center(N): | 反転中心(N): | 라벨 |
| `shape.fill.transparency_increase_all` | 모두 증가 | Increase all | すべて増加 | 버튼 |
| `shape.fill.transparency_decrease_all` | 모두 감소 | Decrease all | すべて減少 | 버튼 |
| `shape.fill.small_side` | 작은 쪽 | Small side | 小さい側 | 옵션 |
| `shape.fill.big_side` | 큰 쪽 | Big side | 大きい側 | 옵션 |
| `shape.fill.small_x_small` | 작은×작은 | Small×Small | 小×小 | 옵션 |
| `shape.fill.small_x_medium` | 작은×중간 | Small×Medium | 小×中 | 옵션 |
| `shape.fill.small_x_big` | 작은×큰 | Small×Big | 小×大 | 옵션 |
| `shape.fill.medium_x_small` | 중간×작은 | Medium×Small | 中×小 | 옵션 |
| `shape.fill.medium_x_medium` | 중간×중간 | Medium×Medium | 中×中 | 옵션 |
| `shape.fill.medium_x_big` | 중간×큰 | Medium×Big | 中×大 | 옵션 |
| `shape.fill.big_x_small` | 큰×작은 | Big×Small | 大×小 | 옵션 |
| `shape.fill.big_x_medium` | 큰×중간 | Big×Medium | 大×中 | 옵션 |
| `shape.fill.big_x_big` | 큰×큰 | Big×Big | 大×大 | 옵션 |

### 개체 설명문·일반

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.alt_text.title` | 개체 설명문 | Alt text | 代替テキスト | 그룹 |
| `shape.alt_text.btn` | 개체 설명문(X)... | Alt text(X)... | 代替テキスト(X)... | 버튼 |
| `shape.protect` | 개체 보호하기(K) | Protect object(K) | オブジェクトを保護(K) | 토글 |
| `shape.gap_between_objects` | 개체와의 간격(G): | Object spacing(G): | オブジェクトとの間隔(G): | 라벨 |
| `shape.preview.right_column` | 그림(B) | Image(B) | 画像(B) | 라벨 |

### 회전 자리·이동·정렬 미리보기

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.preview.preview_basic` | 가\nA B | (preview) | (preview) | 미리보기 글자 |
| `shape.preview.preview_stack` | 가\nA\nB | (preview) | (preview) | 미리보기 글자 |
| `shape.preview.preview_underline_h1` | 가1─ | (preview) | (preview) | 미리보기 글자 |
| `shape.preview.preview_underline_v1` | 가1│ | (preview) | (preview) | 미리보기 글자 |
| `shape.preview.preview_corner_topright` | 가1┐ | (preview) | (preview) | 미리보기 글자 |
| `shape.preview.preview_corner_bottomright` | 가1┘ | (preview) | (preview) | 미리보기 글자 |

### shape 자리 confirm 버튼

| 키 | 한국어 | English | 日本語 | 맥락 |
|---|---|---|---|---|
| `shape.confirm` | 확인(D) | OK(D) | OK(D) | 버튼 |

## 11~33. 나머지 후속 카테고리

*m700-1.6 sub-cycle 자리에서 박는다.*

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
