# Task #m600-30 — 시각 튜닝 이슈 트래커

cycle 28·29 의 round-trip path 가 IR 단위로는 정합되었으나, *세부 paragraph 의 char_shape·para_shape 자료*가 cell 안에서 손실되는 잔존 결함 추적. 사용자 시각 보고를 누적하면서 하나씩 처리.

원본 hwp: `/Users/yuniba_01/Downloads/icon/1. (★사업중 필독) 사업관리 참조표.hwp`.

## 이슈 1 — cell 안 paragraph 의 들여쓰기 (para_shape margin_left) 손실

### 재현 절차

1. 원본 hwp 자료를 server 에 업로드 후 studio 열람.
2. 4행3열 표의 2행 2열 cell ("주요 내용" 자체) 자체 자체 자체 "개인정 보동의서수정본도 재첨부" 뒤에 커서 두고 ` 앵 뭐가 달라진거지? ㅋㅋㅋㅋ 여기다가 이렇게하면 똑같잖아.` 텍스트 추가.
3. 화면 자체 (편집 직후) — 굵게·들여쓰기·번호 자료 모두 정상.
4. 브라우저 새로고침.
5. **결함** — cell 안 일부 단락의 *왼쪽 들여쓰기* 가 사라짐.

### 시각 자료

| 자리 | Before (편집 직후) | After (새로고침) |
|---|---|---|
| "❶ NIPA NXT시스템 접속..." | 굵게, 들여쓰기 없음 | 굵게 유지 ✓ |
| "- (변경개요 첨부파일) ..." | *왼쪽 들여쓰기* 박힘 | **들여쓰기 사라짐** (왼쪽 끝 붙음) |
| "※ 참여인력 변동 시..." | *깊은 들여쓰기* 박힘 | **들여쓰기 사라짐** |

### 가설

cycle 28 의 HWPX put_snapshot 자료 path round-trip 자체 — *cell 안 paragraph 의 para_shape_id* 가 직렬화·역직렬화 자체 자체 자체 자체 자체 자체 손실 또는 default (0) 로 박힘.

가능 자리:
- HWPX serializer 의 `write_sub_list` 안 `<hp:p paraPrIDRef>` 박는 자료 — *cell.paragraphs[i].para_shape_id* 그대로 박는지 정독.
- HWPX parser 의 `<hp:p paraPrIDRef>` 자료 → `Paragraph.para_shape_id` 자료 매핑.
- HWP serializer 의 cell paragraph 직렬화 자료 — `PARA_HEADER` 의 para_shape_id 박는지.

### 진단 단계

1. 원본 hwp parse 후 cell (table_para=0, row=1, col=1) 의 paragraphs[i].para_shape_id 자료 dump.
2. 같은 cell 의 round-trip 후 paragraphs[i].para_shape_id 비교.
3. 손실 자리 식별.

## 이슈 (이후 추가)

(사용자가 자료 박는 시점 자체 자체 추가)
