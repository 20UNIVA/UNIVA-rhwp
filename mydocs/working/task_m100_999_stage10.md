# Task #999 Stage 10(후속) — 저장(서버 minio 덮어쓰기)

- 브랜치: `feature/ssr`
- 배경: upload API에 `file_id` formData 추가 시 **덮어쓰기**(저장) 지원이 생김.
  - file_id 없이 → 신규 발급(`updated:false`)
  - file_id + 파일 → 해당 위치 덮어쓰기(이름 달라도), 경로/이름 갱신(`updated:true`), 없는 id면 404

## 구현

### `server/src/storage.rs`
- `upload(bytes, filename, file_id: Option<&str>)` — `file_id=Some`이면 formData에 `file_id` 추가
- 반환을 `UploadResult { file_id, minio_key, updated }` 로 변경(덮어쓰기 시 새 경로 확인)

### `server/src/main.rs`
- `Session.filename` 추가 + `default_filename(file_id, format)`
- 모든 세션 생성부에 filename 채움
- **`POST /sessions/{id}/save`** — 현재 세션 문서를 export(hwp/hwpx) → `upload(..., Some(file_id))` 로 minio 덮어쓰기 → `{fileId, minioKey, updated}` 반환
- `create_document` 의 upload 호출은 `None`(신규) + `.file_id`

### `rhwp-studio`
- `CommandServices.saveToServer?: () => Promise<boolean>` 추가
- `saveCurrentDocument` 맨 앞: `saveToServer` 있으면 우선 호출(성공 시 markClean+`'saved'`, 실패 시 로컬 저장 폴백)
- `main.ts`: SSR 모드면 `saveToServer` 주입 — 대기 op `flushOps()` 후 `POST /save`. dev 전역 `__dispatcher` 노출

## 검증

### 서버 (실서버 + 실제 minio)
```
업로드→편집("SAVED!")→POST /save (updated=true, minioKey 갱신)
  → 다른 서버(빈 db)가 같은 fileId로 download → "SAVED!" 반영   PASS
```

### studio E2E (`ssr-save.test.mjs`)
```
빈문서 진입 → 편집("SAVEME") → dirty=true
file:save → 서버 /save 라우팅 → dirty=false                    PASS
서버 IR에 "SAVEME" 반영                                         PASS
```

## 비고

- "저장" = 현재 세션을 같은 file_id로 minio 덮어쓰기(작업 중 sqlite 영속과 별개로 **원본 갱신**).
- filename은 복원 경로(sqlite/download)에선 `{file_id}.{format}` 기본 — 원본 파일명 보존은 store 스키마에 filename 추가 시 가능(후속). minio는 덮어쓰기 시 새 이름/경로를 반환하므로 기능상 문제 없음.
- 대용량 업로드(axum body limit 2MB) 후속 과제 여전.
