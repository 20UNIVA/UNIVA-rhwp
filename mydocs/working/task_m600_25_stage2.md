# Task #m600-25 Stage 2 — 패치 적용

## 변경

### `src/serializer/hwpx/context.rs`
SerializeContext 에 `resolved_styles: Option<ResolvedStyleSet>` + `dpi: f64` 추가. `collect_from_document` 안에서 `resolve_styles_with_variant` 호출해 박음.

### `src/serializer/hwpx/table.rs:281-330`
cell paragraph 직렬화 자리. before — 단일 정적 lineseg (vertsize=1000, spacing=600) 하드코딩. after:

```rust
let reflowed_segs = if !para.text.is_empty() && para.line_segs.len() <= 1 {
    if let Some(styles) = &ctx.resolved_styles {
        let mut p = para.clone();
        let available_px = (cell.width as f64) * ctx.dpi / 7200.0;
        reflow_line_segs(&mut p, available_px, styles, ctx.dpi);
        p.line_segs
    } else {
        para.line_segs.clone()
    }
} else {
    para.line_segs.clone()
};
// reflowed_segs 자료를 hp:lineseg 자료로 직렬화
```

조건 — *비어있지 않고 line_segs ≤ 1 자리*에만 reflow 호출. paragraph 자체는 mutate 안 함 (clone).

### `src/serializer/hwpx/shape.rs:249-296`
글상자 paragraph 같은 패턴. p.line_segs IR 그대로 직렬화. 비어있을 때만 fallback.

## 검증

| 케이스 | 결과 |
|---|---|
| `cargo test --workspace --lib` | 1491 passed / 0 failed / 6 ignored |
| `cargo clippy --workspace --lib` | warning 0 |
| `test-reflow` (원본 hwp + replace) `?fmt=hwpx` dump row 1 | `lh=2000`, 2 segments 정상 |
| 클라 console warnings | 18 → 3 (cell paragraph 정합 회복) |

## 남은 결함 (task 26 분리)

원본 hwp 의 `border_fill[7] = Gradient` 가 export hwpx 의 header.xml 에서 *Solid 만 직렬화*. 그라데이션 색띠·아래 표 색상 손실. header serializer 가 Gradient·Pattern·Image fill_type 미지원. cycle 26 으로 분리.
