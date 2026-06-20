# Task #m600-24 Stage 2 — 후보 A 패치 적용 보고서

## 자리

[src/renderer/composer/line_breaking.rs:902-1007](../../src/renderer/composer/line_breaking.rs#L902-L1007) 의 `reflow_line_segs` 함수.

## 변경 자료

### before

```rust
let make_line_seg = |utf16_start: u32, max_font_size: f64| -> LineSeg {
    let fs = if max_font_size > 0.0 {
        max_font_size
    } else {
        12.0   // 빈 paragraph + line 단위 max_font_size=0 자리 자료 동일 폴백
    };
    ...
};

if para.text.is_empty() {
    para.line_segs = vec![make_line_seg(0, 0.0)];   // 0.0 → 12.0 폴백
    return;
}

// fill_lines 결과 처리
for lb in &line_breaks {
    let fs = if lb.max_font_size > 0.0 {
        lb.max_font_size
    } else {
        12.0   // 호출자 자리 폴백 (make_line_seg 자리와 중복)
    };
    new_line_segs.push(make_line_seg(utf16_start as u32, fs));
}

if new_line_segs.is_empty() {
    new_line_segs.push(make_line_seg(0, 12.0));   // literal 12.0 안전장치
}
```

### after

```rust
// paragraph 첫 char_shape 자료 자리 한 번 계산
let para_first_fs = para
    .char_shapes
    .first()
    .and_then(|cs| styles.char_styles.get(cs.char_shape_id as usize))
    .map(|s| s.font_size)
    .unwrap_or(12.0);

let make_line_seg = |utf16_start: u32, max_font_size: f64| -> LineSeg {
    let fs = if max_font_size > 0.0 {
        max_font_size
    } else {
        para_first_fs   // 단일 폴백 자리: char_shapes 자료 살아 있으면 그 자료,
                        // 비면 12.0 (para_first_fs 안 unwrap_or)
    };
    ...
};

// 빈 paragraph 자리 그대로 — closure 안에서 자료 처리
if para.text.is_empty() {
    para.line_segs = vec![make_line_seg(0, 0.0)];
    return;
}

// fill_lines 결과 처리 — 호출자 자리 폴백 자리 제거. make_line_seg 안에서 처리.
for lb in &line_breaks {
    new_line_segs.push(make_line_seg(utf16_start as u32, lb.max_font_size));
}

// 안전장치 — literal 12.0 자리 자료 자리 0.0 자리. closure 안에서 para_first_fs 적용.
if new_line_segs.is_empty() {
    new_line_segs.push(make_line_seg(0, 0.0));
}
```

## 변경 효과

- *세 곳의 폴백이 단일 지점으로 통합*. make_line_seg closure 안에서 max_font_size=0.0 일 때 *paragraph 첫 char_shape font_size* 차용.
- char_shapes 가 정상이면 *원본 font_size 가 line_height 산출에 그대로 사용*. 빈 셀 paragraph (font_size=1.0pt) 의 lh ≈ 100 HWPUNIT.
- char_shapes 비어 있는 *예외 시점*에만 12.0pt 최종 폴백이 작동. 원본 자료 보존 의미 정합.

## 컴파일 검증

```bash
~/.cargo/bin/cargo check --quiet
# (무출력 = 통과)
```

## 다음 자리 — Stage 3

- `cargo test --workspace --lib` 회귀 0건 자리 자료 확보.
- `cargo clippy --workspace -- -D warnings` 0 warning 자리.
- 단위 테스트 추가 자리 — 빈 paragraph + char_shapes[0] font_size=1.0pt 자리 line_height 결과 자료.

## 위험

- *paragraph 첫 char_shape* font_size 가 *paragraph 안 가장 큰 font_size* 와 다른 경우. 행 안 max_font_size=0 가 *fill_lines 자체 결함* 에서 비롯되었다면 *부분 정합* 가능. row 1 시각 정합 확인 필요 → Stage 4 시각 검증.
