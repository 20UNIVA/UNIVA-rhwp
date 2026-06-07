//! IR Compact — 모델 친화적 평탄 IR 응답.
//!
//! 옛 rhwp 원본의 `rhwp/rhwp-studio/src/llm-replay/ir-builder.ts` 알고리즘을
//! 서버 측 Rust 로 포팅. DocumentCore 의 내부 struct 를 직접 읽어
//! init.md 가이드의 응답 형식 (type/runs/cell_locator/defaults) 으로 변환한다.
//!
//! 호출 위치: `server/src/main.rs::ir_slice_handler` 의 compact 분기.

#![allow(dead_code)]  // 구현 진행 중 일시 허용. Phase 5 종료 시 제거.

// 이하 Task 1.2 ~ 1.3 에서 채워짐.
