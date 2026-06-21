# Task #m600-38 최종 보고서

cell_path::tests 7건의 hardcoded 외부 경로 의존을 `samples/hwpx_roundtrip/baseline_business_table.hwp` in-tree fixture 로 옮김.

## 결과

```
cargo test --lib document_core::commands::cell_path  # 7 passed (0 failed)
cargo test --lib                                     # 1498 passed / 0 failed / 6 ignored
```

cycle 33~37 누적 blocker (cargo test --lib 7건 실패) 해소.

## 코드 변경

- [src/document_core/commands/cell_path.rs:259-272](../../src/document_core/commands/cell_path.rs#L259-L272) — `SAMPLE_HWP` 절대 경로 상수 → `SAMPLE_HWP_REL` 상대 경로 + `sample_hwp_path()` 헬퍼.
- `load_core` 가 환경 독립 경로로 fixture 를 읽도록 정정.

## 검증

`baseline_business_table.hwp` 가 cycle 29 작성 시점의 `1. (★사업중 필독) 사업관리 참조표.hwp` 와 동일 자료임이 7/7 테스트 통과로 확정. cycle 31 에서 deep diff fixture 로 박아둔 자리와 cycle 29 cell_path 테스트가 같은 nested 3x2 표 (s0.p4.c0 outer 1x1 → cell[0].paragraphs[8].controls[0]) 를 가리킴.
