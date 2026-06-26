# rhwp-studio i18n 키 명명 규칙

vfinder 의 [key-naming-convention.md](../../../vfinder/docs/i18n/key-naming-convention.md) 자료를 옮긴 자리. rhwp-studio 도메인에 맞게 조정·확장.

## 1. 키 구조 — `카테고리.역할`

**기본 — 두 단**:
```
toolbar.copy
button.cancel
bookmark.dialog_title
charmap.dialog_title
```

**예외 — 세 단** (자리 자체에 명확한 *상위 묶음*이 있는 자리):
```
menu.file.save          ← 메뉴바 드롭다운 7 자리 (file/edit/view/insert/format/page/table)
char_shape.attr.bold    ← 글자 모양 안 *속성* 그룹
char_shape.line.solid   ← 글자 모양 안 *선 종류* 그룹
error.client.network    ← 클라이언트 vs 서버 분리
error.server.timeout
```

**금지 — 네 단 이상**: 깊이만 늘리고 검색·자동완성·읽기 자체 자체 어렵게 박힘. 4단 자리 자체 자체 자체 *재구성*해 3단으로 박는다 (`page.section.obj_picture` 자리 자체 자체 자체 *4단* `page.section.obj.picture` 자리 자체 자체 자체 박지 *않는다*).

## 2. 작성 규칙

| 규칙 | 예시 |
|---|---|
| **소문자 `snake_case`** | `find_replace`, `not_yet_run`, `find_target` |
| **도트는 카테고리 구분에만** | `bookmark.list` (도트) / `find_again` (의미 구분 `_`) |
| **카멜·케밥 금지** | `bookmark.dialogTitle` (X) / `bookmark.dialog_title` (O) |
| **단·복수 별도 키 금지** | `delete.one`/`delete.multi` (X) / `delete.confirm` + `{count}` placeholder (O) |
| **placeholder 있는 자리 / 없는 자리 충돌 시 suffix** | `picker.save_here` / `picker.save_here_with_path` |
| **버튼 단축키 라벨 `(A)`·`(D)` 자리** | 한국어 라벨 안에 박는다. 키 이름엔 박지 않는다 |

## 3. 카테고리 prefix 카탈로그

rhwp-studio 자리 33 카테고리:

### 메뉴바 (3단)
| prefix | 자리 |
|---|---|
| `menu.file.*` | 파일 |
| `menu.edit.*` | 편집 |
| `menu.view.*` | 보기 |
| `menu.insert.*` | 입력 |
| `menu.format.*` | 서식 |
| `menu.page.*` | 쪽 |
| `menu.table.*` | 표 |

### 도구바·서식바·상태바 (2단)
| prefix | 자리 |
|---|---|
| `toolbar.*` | 도구 상자 |
| `stylebar.*` | 서식 도구 모음 |
| `statusbar.*` | 상태 표시줄 |

### 대화상자 (2단, 안에 3단 그룹 자리)
| prefix | 자리 | 안의 3단 그룹 |
|---|---|---|
| `char_shape.*` | 글자 모양 | `.attr.*`, `.lang.*`, `.font_source.*`, `.outline.*`, `.underline.*`, `.border.*`, `.bg.*`, `.line.*`, `.preview.*`, `.misc.*` |
| `para_shape.*` | 문단 모양 | `.align.*`, `.vertical_align.*`, `.margin.*`, `.indent.*`, `.spacing.*`, `.line_spacing.*`, `.preview.*`, `.ext.*`, `.line_break.*`, `.tab.*`, `.border.*`, `.line.*` |
| `table.*` | 표·셀 | `.create.*`, `.tab.*`, `.placement.*`, `.pos.*`, `.size.*`, `.margin.*`, `.cell.*`, `.split.*`, `.text_dir.*`, `.border.*`, `.fill.*`, `.caption.*`, `.gradient.*`, `.props.*` |
| `shape.*` | 그림·도형 | `.dialog_title`, `.tab.*`, `.picker.*`, `.size.*`, `.pos.*`, `.rotate.*`, `.picture.*`, `.effects.*`, `.line.*`, `.rect.*`, `.arc.*`, `.shadow.*`, `.textbox.*`, `.fill.*`, `.alt_text.*`, `.preview.*` |
| `page.*` | 편집 용지·구역·격자 | `.setup.*`, `.section.*`, `.grid.*` |
| `bookmark.*` | 책갈피 | `.sort.*` |
| `find.*` | 찾기·바꾸기·찾아가기 | `.goto.*` |
| `charmap.*` | 문자표 | `.area.*` |
| `equation.*` / `formula.*` | 수식·계산 | — |
| `compare.*` / `history.*` | 문서 비교·이력 | — |
| `prefs.*` | 환경 설정 | — |
| `header_footer.*` | 머리말·꼬리말 | `.tpl.*`, `.apply.*` |
| `footnote.*` | 각주·미주 | — |

### 공통 (2단)
| prefix | 자리 |
|---|---|
| `context_menu.*` | 우클릭 |
| `button.*` | 공통 버튼 |
| `toast.*` | 토스트 |
| `empty.*` | 빈 상태 |
| `time.*` | 시간 표기 |
| `error.client.*` | 클라이언트 에러 (3단) |
| `error.server.*` | 서버 에러 (3단) |
| `confirm.*` | 확인 다이얼로그 |
| `font.*` | 글꼴·언어 (`.lang.*`, `.preset.*`) |

## 4. 역할 suffix

같은 자리 자체 의미 분기 자체 *suffix* 박는다.

| suffix | 의미 | 예 |
|---|---|---|
| `_btn` | 버튼 라벨 (자명한 자리는 생략) | `bookmark.insert_btn`, `bookmark.delete_btn` |
| `_title` | dialog·panel 제목 | `compare.dialog_title`, `find.goto_title` |
| `_label` | 입력 필드·옵션 라벨 (자명하면 생략) | `bookmark.new_name`, `find.find_target` |
| `_placeholder` | input placeholder | `history.note_placeholder` |
| `_hint` | 옵션 옆 부연·미리보기 부연 | `para_shape.preview.hint_second_line` |
| `_msg` / `_body` | 일반 본문 | `compare.run_first` |
| `_toast` | 토스트 자체 (자명하면 생략) | `history.cleared` |
| `_aria` | 눈에 안 보이는 aria-label | — |
| `_one` / `_multi` | 단일 vs 다중 *의미 단위* (단복수 자리 자체 아님) | — |
| `_failed` | 에러 자체 | `compare.failed`, `bookmark.add_failed` |
| `_confirm` | confirm dialog 자체 | `confirm.delete_history` |

**suffix 자체 자체 *자명하면 생략***. 모든 키에 `_btn` 박지 않는다.

## 5. placeholder 카탈로그

vfinder + rhwp 자리 자체 도입.

| placeholder | 뜻 |
|---|---|
| `{count}` | 항목 개수 |
| `{n}` | 일반 수치 (시간·진행 자리) |
| `{current}` | 현재 번호 |
| `{total}` | 전체 번호 |
| `{i}` | 진행 번호 (`인쇄 준비 중... ({i}/{total})`) |
| `{name}` | 단일 항목 이름 |
| `{names}` | 이름 여러 개 comma-join |
| `{path}` | 절대 경로 |
| `{message}` | 일반 에러 원문 |
| `{error}` | (deprecated — `{message}` 자체로 통일) |
| `{detail}` | bad request detail |
| `{value}` | 일반 값 |
| `{max}` | 최대 자리 (`File too large (max {max} MB)`) |
| `{left}`, `{right}` | 비교 자리 (`compare.detail_title_pair`) |

**규칙**: 코드에서 변수명이 `selectedCount` 라도 사전에선 `{count}` 자리 통일. 변환은 호출 자리에서: `t('delete.confirm', { count: selectedCount })`.

## 6. 자주 가는 키 명명 결정

- *글머리표 모양* / *번호 모양* (한국어) — 영어 *Bullet style* / *Numbering style*. **`menu.format.bullet_shape`** 자리 자체 *style* 자리 자체 박지 않고 *shape* 박은 자리 자체 자체 *한국어 라벨 직역*. *한컴 도메인 충실* 정합.
- *위치* (한국어) — 영어 *Position* 자리 자체 *통일* (`Location` 자체 자체 자체 박지 않음 — sub-agent 검증 결과 박은 정정 자리).
- *종류* (한국어) — 영어 *Type* 자리 자체 *통일* (`Kind` 자체 박지 않음).
- *간격* (한국어) — *spacing* 또는 *interval* — *spacing* 자리 우선 (`Cell spacing`, `Line spacing`, `Column spacing`).
- *자간* — 영어 *Character spacing* 자체 자체 자체 자체 *명확화* (sub-agent 검증 자체).

## 7. 검증 자리

키 자체 자체 자체 정합 검증 자료 자체 (m700-4 cycle):

```bash
# 키 추출
grep -oE "^\| \`[a-z_.]+\`" mydocs/manual/i18n_translation_table.md \
  | sed -E 's/\| `//; s/`$//' | sort -u > /tmp/all_keys.txt

# 깊이 분포
awk -F. '{print NF}' /tmp/all_keys.txt | sort | uniq -c
```

m700-4 cycle 자리 자료:
- 깊이 1: 1 자리 (grep 자체 자체 자체 자체 자체 헤더 박힌 자리. 무시)
- 깊이 2: 208 자리 — 정합
- 깊이 3: 585 자리 — 정합 (메뉴바 + 대화상자 그룹 자리)
- 깊이 4: *9 → 0 자리*. `page.section.obj.*`·`shape.line.arrow.*`·`shape.line.cap.*` 자리 자체 *깊이 3 통일* 정정 박힘.
- camelCase·kebab: 0 자리 — 정합.

## 8. TS 자리 `as const` + `Record<keyof typeof messages_ko, string>` 박는 자리

vfinder playbook §3.3 정합. *messages.ko.ts* 에 `as const` 박고 *messages.en.ts·ja.ts* 에 `Record<keyof typeof messages_ko, string>` 박으면 *en·ja 자리 키 누락·typo* 자리 자체 TS 컴파일 에러로 자동 검출. m700-5 cycle 자리 자체 박는다.

## 9. 새 키 추가 자리

m700-5 자리 자체 자체 *이후* 코드 치환 자리 (m700-6 ~ 9) 자리에서 *발견되는 누락 키*는:

1. 한국어 자체 자체 자체 사전에 박는다 (`messages.ko.ts` 자체 자체 자체 박은 자리 자체 자체 *세 사전 동시* 박는다)
2. EN·JA 자체 자체 자체 *fallback ko* 안 박는다 — *조용한 버그* 자체 자체 자체 자체 자체 자체 박는다. vfinder playbook §4.6 정합.
3. 적절한 카테고리 자체 자체 자체 박는다. 새 카테고리 자체 자체 자체 자체 자체 박을 자리 자체 *prefix 카탈로그 (§3) 자체 자체 자체* 박는다.
