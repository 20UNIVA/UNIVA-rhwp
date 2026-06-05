# rhwp SSR 서버 ↔ 외부(minio) 모듈 인터페이스 계약

본 문서는 rhwp SSR 세션 서버(`server/`)와 **외부 모듈**(minio 연동 등) 사이의 책임 경계와 호출 계약을 정의한다. (Task #999)

## 책임 경계

| 책임 | 담당 |
|------|------|
| minio에서 fileId로 문서 다운로드 | **외부 모듈** |
| 문서 파싱·편집·상태 보유·patch 영속 | rhwp-server |
| 현재 상태 IR JSON 제공 (모델 조회) | rhwp-server |
| 확정 시점의 hwp/hwpx 바이트 제공 | rhwp-server (`GET /export`) |
| export 바이트를 minio에 업로드 | **외부 모듈** |
| 인증/인가 | 외부 게이트웨이 |

→ rhwp-server는 **minio를 모른다.** input은 `fileId` + 파일 바이트, output은 IR JSON + export 바이트뿐이다.

## 전체 흐름

```
1. [외부] minio에서 fileId로 다운로드 → bytes
2. [외부/프론트] 세션 시작:
   (a) 직접:  POST /sessions { fileId, format, fileBase64 }
   (b) iframe: editor.loadFile(bytes, name, { fileId })  → studio가 (a) 수행
3. [사용자] iframe에서 편집 → studio가 op/스냅샷을 서버로 자동 미러링
   (프론트를 닫아도 서버 세션·sqlite에 상태 유지)
4. [모델]  GET /sessions/{fileId}/ir            → 현재 상태 IR JSON
5. [외부] 확정 저장 시:
   GET /sessions/{fileId}/export?fmt=hwp|hwpx   → 바이트
   → minio에 업로드 (외부 책임)
6. (선택) DELETE /sessions/{fileId}             → 메모리 세션 해제 (영속 유지)
```

## API 계약

### POST /sessions — 세션 생성/재생성
요청:
```json
{ "fileId": "minio-abc", "format": "hwp|hwpx", "fileBase64": "<base64 파일>" }
```
응답 `200`:
```json
{ "fileId": "minio-abc", "seq": 0, "sectionCount": 1, "paragraphCount": 12 }
```
- `format` 생략 시 `"hwpx"`. 파싱은 바이트 magic으로 자동 판별되므로 export 기본 포맷 힌트 용도.
- 같은 fileId 재호출 시 ops/snapshots 초기화 후 재생성.

### POST /sessions/{id}/ops — 연산형 patch 적용
요청: `EditOperation` 배열
```json
[ { "op": "insert_text", "section": 0, "para": 0, "offset": 0, "text": "안녕" } ]
```
- 연산 종류: `insert_text` / `delete_text` / `split_paragraph` / `merge_paragraph` (양방향).
- 보통 studio가 자동 전송. 외부가 직접 편집 주입도 가능.

### PUT /sessions/{id}/snapshot — 스냅샷형 동기화
요청: `{ "fileBase64": "<현재 전체 문서 base64>" }`
- 붙여넣기/객체/표 편집 등 연산으로 표현 불가한 변경에 사용.

### GET /sessions/{id}/ir — 모델 조회
응답 `200 application/json`: `DocumentIrView`
```json
{ "schema_version": 1, "section_count": 1,
  "sections": [ { "index":0, "paragraph_count":12,
    "paragraphs": [ { "index":0, "text":"…", "char_count":5,
      "para_shape_id":3, "style_id":1,
      "char_runs":[{"start":0,"char_shape_id":7}],
      "controls":[{"kind":"table","rows":3,"cols":4}] } ] } ] }
```

### GET /sessions/{id}/export?fmt=hwp|hwpx — 확정 저장용 바이트
응답 `200 application/octet-stream` (+ `Content-Disposition: attachment; filename="{id}.{ext}"`)
- `fmt` 생략 시 세션 생성 시 format 사용.
- **확정 저장 트리거**: 외부 모듈이 이 엔드포인트를 호출하는 시점이 곧 "확정". 서버는 바이트 제공만, minio 업로드는 외부.

### DELETE /sessions/{id} — 메모리 세션 해제
- 메모리에서 제거(영속 sqlite는 유지). 이후 재요청 시 sqlite에서 복원.

### GET /health — 헬스 체크

## 영속·복원

- 작업 중 상태는 sqlite(`RHWP_SERVER_DB`, 기본 `rhwp-sessions.db`)에 영속:
  `sessions`(원본 base) + `ops`(연산 로그) + `snapshots`(스냅샷).
- 서버 재시작/세션 미적재 시: "최근 snapshot(없으면 base) + 이후 ops 재적용"으로 복원.
- 클라이언트 연결 여부와 무관하게 상태 유지 → 모델은 언제든 `GET /ir` 조회 가능.

## 설정 (환경변수)

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `RHWP_SERVER_ADDR` | `0.0.0.0:7710` | bind 주소 |
| `RHWP_SERVER_DB` | `rhwp-sessions.db` | sqlite 경로 (`:memory:` 가능) |
| `RUST_LOG` | `rhwp_server=info` | 로그 레벨 |

## 1차 스코프 한계

- 단일 편집자(fileId 세션 단위). 멀티 편집자 동시성(CRDT/OT)은 범위 외.
- 세션 TTL/정리 정책 미구현 (DELETE는 수동).
- char 오프셋은 BMP 문자 기준(서로게이트 페어 미정규화).
