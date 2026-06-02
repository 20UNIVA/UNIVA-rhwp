# Task #999 Stage 9(후속) — studio 빈문서 업로드 + 열기 분기 워크플로우

- 브랜치: `feature/ssr`
- 의존: Stage 8(서버 minio 프록시: POST /documents, download 폴백)

## 목표 (작업지시자 요구)

- 편집기 진입 시 **fileId 없으면** upload API로 **빈 문서**를 올리고 발급된 fileId로 세팅
- 거기서 "열기"로 다른 문서를 올리면 분기:
  - 빈 문서를 **한 번도 편집 안 했으면** → 빈문서 세션 닫고 새 문서를 upload+세팅
  - **편집했으면** → 빈문서 세션 유지 + 새 문서 upload+세팅

## 구현 (`rhwp-studio/src/main.ts`)

- SSR 상태: `currentSsrFileId`, `currentIsBlank`, `SSR_MODE`(fileId/ssrBase/ssr 중 하나라도 존재)
- `ssrUploadNewDocument(bytes, filename)` → `POST /documents` → 발급 fileId
- `ssrDeleteSession(fileId)` → `DELETE /sessions/{id}` (메모리 해제, 영속 유지)
- `startBlankSsrDocument()` → 빈 문서 생성 → export → 업로드 → 미러링, `isBlank=true`
- `attachSsrMirror`/`createSsrSession` 에 `currentSsrFileId` 갱신
- `ssrBootstrap()` (부팅): fileId 있으면 복원(`restoreSsrSessionIfNeeded`), 없으면 `startBlankSsrDocument()`
- **`loadBytes` 열기 분기**: 새 문서 로드(=markClean) **전에** 직전 세션의 "빈문서+미편집"(`currentIsBlank && !documentState.isDirty()`)을 캡처 → 로컬 열기 시:
  - 미편집이면 `ssrDeleteSession(prevFileId)` (빈세션 닫기) 후 새 문서 업로드
  - 편집했으면 닫지 않고(서버 보존) 새 문서 업로드
- dev 전역 `window.__ssr = { fileId, isBlank }` 노출(E2E용)

## 검증 (E2E, 실서버 + 실제 minio)

`rhwp-studio/e2e/ssr-upload-flow.test.mjs` (puppeteer headless):
```
[A] ?ssrBase= (fileId 없음) 진입
    → 빈문서 자동 업로드, fileId 발급(624b…), isBlank=true, 서버 세션 생성   PASS
[C] 빈문서 편집("KEEPME") 후 다른 문서 열기
    → 새 fileId(47c4…)로 전환, isBlank=false                              PASS
    → 편집했던 빈문서 세션은 서버에 "KEEPME" 보존                          PASS  ← 유지
[B] 빈문서 미편집 후 다른 문서 열기
    → 새 fileId(ae69…)로 전환                                            PASS  ← 빈세션 닫고 전환
```

## 비고

- 로컬 "열기"는 내부적으로 모든 문서를 minio에 업로드(fileId 발급) → 일관된 세션 관리.
- 편집 여부 판단은 `DocumentDirtyState.isDirty()`. 새 문서 로드가 markClean 하므로 **로드 전에** 직전 상태를 캡처하는 게 핵심.
- "빈세션 닫기"는 `DELETE`(메모리 해제)이며 minio 원본/sqlite는 남는다(필요 시 재조회는 download 폴백으로 복원). 미편집 빈문서를 영구 삭제하려면 별도 정리 정책 필요(후속).
