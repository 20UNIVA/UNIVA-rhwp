# Task #999 Stage 5 완료보고서 — export API + 외부(minio) 모듈 인터페이스 경계

- 브랜치: `feature/ssr`
- 구현계획서: [task_m100_999_impl.md](../plans/task_m100_999_impl.md) Stage 5

## 목표

확정 저장용 바이너리를 제공하는 export API를 추가하고, 외부(minio) 모듈과의 책임 경계·호출 계약을 명문화한다.

## 구현 내용

### `server/src/main.rs` — `GET /sessions/{id}/export`

- 쿼리 `?fmt=hwp|hwpx` (`ExportQuery`)
- `fmt=hwpx` → `rhwp::serializer::serialize_hwpx(doc)`
- `fmt=hwp`(기본/그 외) → `rhwp::serialize_document(doc)`
- 응답: `application/octet-stream` + `Content-Disposition: attachment; filename="{id}.{ext}"`
- fmt 생략 시 세션 생성 시 format 사용
- 라우트 등록: `/sessions/:id/export`

### 문서 — `mydocs/tech/ssr_server_external_module_contract.md`

- 책임 경계표(minio 다운로드/업로드=외부, 파싱/편집/영속/export=서버)
- 전체 흐름(다운로드→세션→편집 미러링→모델 조회→export→업로드)
- 전 API 계약(요청/응답 스키마), 영속·복원, 환경변수, 1차 한계
- **확정 저장 트리거** 정의: 외부 모듈이 `GET /export` 호출 시점 = 확정. 서버는 바이트 제공까지만.

## 검증

### 빌드
```
cargo build (server) — Finished (경고: exists dead_code 1건만)
```

### export 라운드트립 스모크 (실서버)
```
세션 e1 생성 → ops [insert_text "EXPORTTEST"] 적용
GET /export?fmt=hwp  → 7680 bytes
  → 새 세션 e2로 재파싱 → GET /ir 첫 문단 "EXPORTTEST가나다…" → PASS
GET /export?fmt=hwpx → 6357 bytes (ZIP magic 'PK' 확인)
```

→ 서버 보유 상태(편집 반영본)가 hwp/hwpx 바이트로 정확히 직렬화되고, 재파싱 시 편집이 보존됨을 실증.

## 다음 단계

Stage 6 — 통합/E2E 검증 + WASM↔native 결정성 회귀 + 최종 보고.
