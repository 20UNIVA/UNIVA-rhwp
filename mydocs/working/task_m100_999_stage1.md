# Task #999 Stage 1 완료보고서 — Document IR → JSON 직렬화 경로

- 브랜치: `feature/ssr`
- 구현계획서: [task_m100_999_impl.md](../plans/task_m100_999_impl.md) Stage 1

## 목표

모델 조회(`GET /sessions/{fileId}/ir`)의 기반인 **Document IR → JSON 직렬화** 경로를 추가한다. 내부 IR을 직접 직렬화하지 않고, 모델이 읽기 좋은 **안정적 뷰 스키마**로 투영한다.

## 구현 내용

### 신규: `src/model/ir_view.rs`

- `IR_VIEW_SCHEMA_VERSION = 1` — 뷰 스키마 버전 상수 (호환 불가 변경 시 증가)
- 뷰 DTO (serde `Serialize`):
  - `DocumentIrView { schema_version, section_count, sections }`
  - `SectionView { index, paragraph_count, paragraphs }`
  - `ParagraphView { index, text, char_count, para_shape_id, style_id, char_runs, controls }`
  - `CharRunView { start, char_shape_id }`
  - `ControlView { kind, rows?, cols? }`
- `Document::to_ir_view(&self) -> DocumentIrView` — IR → 뷰 투영
- `Document::to_ir_json(&self) -> Result<String, serde_json::Error>` — JSON 직렬화
- 컨트롤 종류 매핑(`control_kind`): table/picture/shape/header/footer/footnote/… 22종. 표는 `row_count`/`col_count` 포함.

### 변경: `src/model/mod.rs`

- `pub mod ir_view;` 등록

## 설계 결정

- **별도 뷰 DTO** 방식 채택: 기존 `Document`/`Paragraph` 모델에 serde derive를 직접 달지 않고 `From`-스타일 투영. 라운드트립용 raw 바이트/캐시 필드(`raw_stream`, `char_offsets` 등)를 제외하고 조회에 필요한 정보만 노출 → IR 내부 변경으로부터 조회 스키마 격리.
- 빈 컬렉션은 `skip_serializing_if`로 생략하여 JSON 간결화.

## 검증

```
cargo test --lib model::ir_view
  test model::ir_view::tests::test_to_ir_view_text_and_meta ... ok
  test model::ir_view::tests::test_to_ir_json_roundtrip_parse ... ok
  test result: ok. 2 passed; 0 failed
  Finished in 42.69s (첫 빌드 포함)
```

- `test_to_ir_view_text_and_meta`: 텍스트/char_count/para_shape_id/style_id/char_runs 투영 검증
- `test_to_ir_json_roundtrip_parse`: JSON 직렬화 후 재파싱하여 schema_version·중첩 text 경로 검증

## 비고

- 개발 환경에 Rust toolchain이 없어 `rustup` 신규 설치(1.93.1, `rust-toolchain.toml` 기준) 후 검증함.
- 표 외 컨트롤의 상세 속성(위치/크기 등)은 1차 뷰에서 `kind`만 노출. 모델이 구조 파악에 필요하면 후속 확장.

## 다음 단계

Stage 2 — `EditOperation` 양방향 프로토콜 + native 적용기.
