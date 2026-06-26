# Task #m700-i18n Stage 3 보고서 — Phase 1 전체 종결 (사전 채워넣기 + 맥락 검증)

## 자리 — m700-1·2·3 cycle 종결

vfinder playbook Phase 1 (translation-table 사전 만들기) 자리 자체 종결. 33 카테고리 자리 자체 한·영·일 박은 후 sub-agent 맥락 검증 박음.

## 누적 자리

| cycle | 카테고리 | 자리 수 |
|---|---|---|
| m700-0 (stage 1·1.1) | §1·2·3·8·10 + sub-agent 검증 정렬 | 33 |
| m700-1 (stage 2.1~2.6) | §32·11·12·13·15·26 | 468 |
| m700-2 (stage 3) | §4·5·6·7·9·25·28·29·30·14·33 | 168 |
| m700-3 (stage 4) | §17·18·19·20·21·22·23·24·27·31 | 75 |
| **stage 4.1 정정** | sub-agent 검증 박은 정정 | -16 자리 (정정만, 신규 자리 없음) |
| **합계** | | **744** |

## 커밋 자리

| commit | 자리 |
|---|---|
| `36357a9f` | m700-0 stage 1 — Phase 0 + Phase 1A 수집·카테고리 골격 (33) |
| `bd09e270` | m700-0 stage 1.1 — sub-agent 검증·25→33 카테고리·9 자리 정정 |
| `e17fbb1b` | m700-1.1 — §32 compare/history (44) |
| `3367c3ee` | m700-1.2 — §11 char_shape (78) |
| `8d1ae24f` | m700-1.3 — §12 para_shape (92) |
| `0fdc2a8b` | m700-1.4 — §13 table (95) |
| `817d34e0` | m700-1.5 — §15 shape (145) |
| `0019252b` | m700-1.6 — §26 header_footer (14) |
| `31525515` | m700-1 stage 2 보고서 |
| `505f2123` | m700-2 stage 3 — §4·5·6·7·9·25·28·29·30·14·33 (168) |
| `603aa312` | m700-3 stage 4 — §17·18·19·20·21·22·23·24·27·31 (75) |
| `8261a403` | stage 4.1 — sub-agent 검증 박은 16 자리 정정 |

## sub-agent 맥락 검증 (stage 4.1)

m700-1·2·3 cycle 자리에서 박은 711 자리 자체 자료 자체에 sub-agent 박아 *맥락 적합성* 검증. 한컴 한글(2022 한국어판) + MS Word(영·일판) + 한컴 한글 일본어판 자리 자체 자체 자체 기준 점검.

### 발견된 결함 (정정 박힘)

**치명 12자리**:
1. `shape.effects.soft_edge*` 한국어 *열은 테두리* → *옅은 테두리* (오탈자)
2. `table.placement.keep_with_anchor` 일본어 *開封符号* → *組版符号* (오타)
3. `shape.textbox.fit_text_with_box` *legacy* 자리 → *글에 어울리게* / *Fit to text* / *文字に合わせる* 박음
4. `font.preset.*` 5자리 일본어 가타카나 박힘 → 원어 그대로 (한컴 일본어판 정합)
5. `table.text_dir.rotate_text/upright_text` 한국어 *문 눕힘/세움* → *문자 눕힘/세움*
6. `shape.textbox.text_horizontal/vertical` 영어 *Lay/Upright English* → *Rotate Latin chars/Upright Latin* (MS Word 자리)
7. `menu.insert.caption_top_center/bottom_center` 한국어 *왼쪽/오른쪽 가운데* → *위/아래 가운데*
8. `charmap.dialog_title` 영어 *Insert symbol* → *Symbol* (MS Word 자리)
9. `table.border.backslash` 일본어 *円記号(¥)* → *バックスラッシュ*
10. `charmap.area.dingbats` 일본어 *デフバット* → *ディングバット*
11. `prefs.dialog_title` 영어 *Preferences* → *Options* (한컴 영문판 자리)
12. `find.find_replace_title` 일본어 *検索/置換* → *検索と置換* (MS Word 자리)

**권장 4자리**:
1. `char_shape.underline.location` 영어 *Location* → *Position* (통일)
2. `bookmark.sort.kind` 영어 *Kind* → *Type* (통일)
3. `char_shape.spacing` 영어 *Spacing(P)* → *Character spacing(P)* (자간 명확화)
4. `error.client.init_failed` placeholder *{error}* → *{message}* (통일)

### 검증 결과 자체

- 사전 자리 자체 *기본 골격*은 한컴·MS Word 자리 정합
- 세부 자리 자체 *오기/오타* 12자리·*불일치* 4자리 박혔으나 *정정 박혔음*
- *동일 한국어 / 다른 EN-JA* 자리 자체 정합 (기본 / 크기 / 종류 / 모양 / 간격 자리)
- placeholder 자리 자체 카탈로그 정합

## 다음 cycle (m700-4) 진입 자리

Phase 2 — *키 명명 정합 검토*.

vfinder playbook §2 자리 자체:
1. 키 구조 `카테고리.역할` 두 단 (메뉴바 자리 예외 3단) — *정합 확인*
2. snake_case·역할 suffix 자리 정합 — *정합 확인*
3. 사용자 검토 한 차례

m700-4 cycle 자리 자체 자체 자체 비용 적은 자리 — 정합 확인·자리 자체 정리.

이어서 m700-5 — *Phase 3 사전 파일 + 헬퍼* (messages.ko.ts, messages.en.ts, messages.ja.ts, t.ts, lang-boundary.ts).

## 사용자 검토 자리

- 744 자리 사전 자료 자체 자체 자체 자체 자체 자체 자체 박힌 자리 자체 정합
- sub-agent 자리 자체 자체 자체 자체 검증 결과 박은 16 자리 정정 자체 자체 정합
- m700-4 (Phase 2 키 명명 정합) 진입 OK 인가
