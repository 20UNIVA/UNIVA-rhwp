# Task #m600-25 Stage 1 — 진단 결과

## 결함

`replace_cell_runs` 후 새로고침 시 서식 깨짐. 진단 결과 결함은 *서버 hwpx 직렬화*에 있음.

## 검증 path

| 케이스 | export 결과 |
|---|---|
| `test-eung-diag` (POST /sessions HWP, `?fmt=hwp` 기본) | row 1 `ls[0] lh=2000`, 2 segments — 정상 |
| `test-eung-diag` (`?fmt=hwpx`) | row 1 `ls[0] lh=1000`, 1 segment — 축소 |
| `sim-1781933565` (시뮬 PUT snapshot, 기본 fmt=hwpx) | row 1 `lh=1000`, 1 segment — 축소 |

같은 server in-memory IR · 같은 `from_bytes` 호출인데 *fmt=hwpx 일 때만* line_segs 가 축소·1 segment 압축.

## 원인 자리

[src/serializer/hwpx/table.rs:281-298](../../src/serializer/hwpx/table.rs#L281-L298) 의 `write_sub_list` 가 cell paragraph 의 line_segs 를 *완전 무시하고 하드코딩*:

```rust
empty_tag(w, "hp:lineseg", &[
    ("textpos", "0"), ("vertpos", "0"),
    ("vertsize", "1000"), ("textheight", "1000"),
    ("baseline", "850"), ("spacing", "600"),
    ...
]);
```

[src/serializer/hwpx/shape.rs:251-266](../../src/serializer/hwpx/shape.rs#L251-L266) 의 글상자 paragraph 도 같은 패턴.

## 추가 발견 (cycle 2)

원본 hwp 의 cell paragraph 는 *HWP5 규약대로 1 lineseg per paragraph*. table.rs 패치로 그 1 lineseg 를 그대로 옮기면 클라 측 validation 이 *LinesegTextRunReflow* 비표준으로 검출하고 auto-fix 발동 — cell.height·서식에 부수효과.

해결 — *셀 폭 기반 reflow_line_segs 호출 결과로 여러 segments 자료를 직렬화*.

## 별 자료 발견 (task 26 후속)

원본 hwp 의 `border_fill[7] = Gradient` 가 export hwpx 의 header.xml 에서는 *Solid 만 직렬화*. header serializer 가 Gradient·Pattern·Image fill_type 미지원. 그라데이션 색띠·아래 표 색상 손실의 별 결함. cycle 26 으로 분리.
