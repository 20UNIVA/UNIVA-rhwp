# Task #999 Stage 6 완료보고서 — 통합/E2E 검증 + WASM↔native 결정성 회귀

- 브랜치: `feature/ssr`
- 구현계획서: [task_m100_999_impl.md](../plans/task_m100_999_impl.md) Stage 6

## 목표

전체 시나리오를 검증하고, WASM↔native 편집 결정성을 회귀 테스트로 고정한다.

## 검증 내용

### 1. WASM↔native 결정성 회귀 (신규 테스트)

`src/document_core/commands/edit_op.rs` 에 `test_op_apply_equals_direct_native` 추가:
- 동일 편집 시퀀스를 (a) `apply_edit_op(EditOperation)` (b) `insert_text_native`/`split_paragraph_native` 직접 호출(= 클라이언트 WASM 경로)로 수행
- 두 결과 문서 텍스트가 **동일**함을 단언 → "서버 op 적용 == 클라이언트 WASM 편집" 결정성 고정

```
cargo test --lib document_core::commands::edit_op
  5 passed (roundtrip 3 + json + 결정성)
```

### 2. 전체 lib 회귀

```
cargo test --lib
  test result: ok. 1412 passed; 0 failed; 6 ignored
  (ir_view 2 + edit_op 5 신규 포함, 기존 테스트 무손상)
```

### 3. 서버 단위 + lint

```
cargo test (server)       3 passed; 0 failed
cargo clippy (server)     error 0 (warning 1: store::exists — 테스트 전용)
```

### 4. End-to-End 통합 시나리오 (실서버, 핵심 요구 실증)

`samples/re-align-center-hancom.hwp` 로:
```
1) POST /sessions                          → 200
2) POST /ops [insert_text "통합"]           → 200
3) DELETE /sessions/S  (프론트 연결 끊김)    → 204 (메모리 세션 해제)
4) GET /sessions/S/ir  (복원 조회)          → "통합가나다라마바사…"  PASS
   (메모리에 없지만 sqlite base+op 재적용으로 복원)
5) GET /sessions/S/export?fmt=hwp           → 7680 bytes
```

→ **"클라이언트 연결을 끊어도 서버단에 파일·패치가 유지되고, 모델이 조회·export 가능"** 이라는 핵심 요구를 실증.

### 기타 검증 (이전 단계 누적)
- Stage 3: 서버 재시작 후 복원 PASS
- Stage 5: export 라운드트립(hwp/hwpx 재파싱 시 편집 보존) PASS
- Stage 2/4: op 양방향 라운드트립, TS serialize 타입 클린, 프로토콜 일치

## 한계 (환경 제약)

- **브라우저 E2E 미수행**: WASM `pkg/` 산출물 생성은 Docker WASM 빌드가 필요한데, 본 환경에서 Docker daemon 미실행으로 불가. studio 측은 타입 검증 + 프로토콜 일치 + 결정성 회귀로 대체 검증. WASM 빌드 가능 환경에서 `rhwp-studio` 를 `?fileId=&ssrBase=` 로 띄워 브라우저 E2E 후속 권장.

## 다음 단계

최종 결과보고서(`task_m100_999_report.md`) 작성 → 작업지시자 승인 → (승인 시) devel 머지 절차.
