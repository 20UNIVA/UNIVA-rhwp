# Task #999 Stage 7(후속) — IR 조회 페이지 필터 (`GET /ir?page=N`)

- 브랜치: `feature/ssr`
- 성격: 6단계 구현 완료 후 작업지시자 요청으로 추가한 기능

## 목표

`GET /sessions/{id}/ir` 에 `page` 파라미터 추가:
- 미지정 → 전체 문서(기존과 동일)
- `page=N`(0-based) → 해당 페이지에 배치된 **본문 문단만** 반환 (대용량 문서 응답 절약)
- 페이지 일부만 보여줘도 **편집 op 좌표(section/para/offset)는 절대값 유지** → 모델이 본 좌표로 그대로 `ops` 호출 가능

## 구현

### `src/model/ir_view.rs`
- `DocumentIrView` 에 `page`/`page_count` 메타 필드(Option, 없으면 생략)
- `paragraph_view(pi, para)` 헬퍼 추출(전체/필터 공용)
- `Document::to_ir_view_filtered(keep: &HashSet<(sec,para)>)` — keep에 속한 문단만, **index는 절대값**, `paragraph_count`는 섹션 전체 수 유지

### `src/document_core/queries/rendering.rs`
- `page_item_para_index(PageItem)` — Full/Partial Paragraph·Table·Shape 5변종 → para_index
- `page_paragraph_set(page) -> HashSet<(sec,para)>` — pagination 순회(global page), 본문 문단(`para_index < body_len`)만. 미주 등 본문 밖 제외
- `to_ir_json_paged(page: Option<u32>)` — page=None 전체 / Some(n) 필터, 항상 `page_count` 포함

### `server/src/main.rs`
- `IrQuery { page: Option<u32> }` + `get_ir` 가 `to_ir_json_paged(q.page)` 호출

## 검증 (실서버, 21페이지 문서 `3-09월_교육_통합_2023.hwp`)

```
page_count=21
page=0   → 본문 문단 88개, 절대좌표 (0,0)~(0,87)
page=5   → 본문 문단 68개, 절대좌표 (0,305)~      ← 페이지마다 다른 문단
page=0 ∩ page=10 = ∅ (안 겹침)
page=10  → 0개  ← 미주 전용 페이지(dump-pages로 pi=544~ 全 "FullParagraph[미주]" 확인). 본문 밖이라 의도적 제외
page=9999→ 빈 결과(섹션 0)

[편집 정합] page=5에서 본 문단(0,305) 끝(offset=9)에 "[E]" 삽입(POST /ops)
  → GET /ir?page=5 재조회 시 그 문단 끝 "[E]" 반영  PASS
```

- 단위: `to_ir_view_filtered` 절대 인덱스 유지 테스트 추가 → ir_view 3/3
- 회귀: `cargo test --lib` **1413 passed, 0 failed**

## 동작 특성 / 한계

- **편집 안전성**: 페이지 필터는 "어떤 문단을 보여줄지"만 줄일 뿐, 각 문단의 `section/para` 절대 인덱스와 전체 `text`(문단 전체)를 그대로 주므로 op 좌표가 어긋나지 않는다. 한 문단이 여러 페이지에 걸쳐도 문단 단위로 포함되어 offset 계산이 안전하다.
- 미주/머리말·꼬리말 등 본문 밖 문단은 페이지 필터에서 제외(EditOperation 좌표계가 본문 기준).
- **후속 과제(별건)**: axum 기본 body limit(2MB)로 일부 대용량 hwp(>~1.4MB)가 `POST /sessions` 에서 413/connection reset. `DefaultBodyLimit` 상향 또는 멀티파트 업로드 필요.
