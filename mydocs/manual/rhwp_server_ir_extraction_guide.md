# rhwp-server IR 추출 가이드 (개발 환경)

`.hwp` / `.hwpx` 파일을 `rhwp-server`에 등록하고, 그 *IR* (문서의 본문 구조를 JSON 으로 풀어 둔 표현, internal representation) 을 가져오는 절차를 정리한다. 이 글은 **외부 파일 저장 서비스 (vfinder / minio 등) 가 같이 떠 있지 않은 개발 환경** 을 전제로 한다.

목적은 두 가지다.

- 사용자 PC 한 대에서 `rhwp-server` 를 띄워 두고, `curl` 로 파일을 등록·IR 을 받아 보는 최소 흐름을 확보한다.
- 필요하면 같은 세션을 `rhwp-studio` (브라우저 편집기) 로 *눈으로 확인* 할 수 있도록 한다.

---

## 0. 필요한 것

- Rust toolchain (`rustup`, 안정 채널)
- Node.js (studio 까지 띄워볼 때만 필요)
- 이 레포(`UNIVA-rhwp/`) 의 working copy

---

## 1. rhwp-server 띄우기

서버 소스는 `UNIVA-rhwp/rhwp-server/` 디렉터리에 있다 (`server/` 가 아니라는 점에 주의 — 워크스페이스에 등록된 별도 크레이트다).

```bash
cd UNIVA-rhwp/rhwp-server
RHWP_DEFAULT_USER=jerry cargo run --release
```

처음 빌드는 수십 초~몇 분. 다음 줄이 뜨면 준비 완료다.

```
rhwp-server listening on 127.0.0.1:7710 (db=rhwp-sessions.db, frame_ancestors=None)
```

### 1.1 주요 환경변수

| 변수 | 용도 | 권장값 |
| --- | --- | --- |
| `RHWP_DEFAULT_USER` | `X-Rhwp-User` 헤더가 빠진 요청의 *사용자 식별자 폴백*. 미설정 시 모든 요청이 400 으로 거부된다. | 임의 문자열 (예: `jerry`) |
| `RHWP_SERVER_ADDR` | 바인딩 주소·포트. 기본 `0.0.0.0:7710`. | `127.0.0.1:7710` |
| `RHWP_SERVER_DB` | 세션 영속화 SQLite 파일 경로. | 기본 `rhwp-sessions.db` (현재 디렉터리) |
| `UPLOAD_URL` / `DOWNLOAD_URL` | 외부 파일 저장 서비스 연동 주소. *개발 환경에선 두지 않는다* — 비어 있으면 자동으로 *비활성 모드* 로 동작한다. | 미설정 |

`X-Rhwp-User` 폴백 동작은 [rhwp-server/src/main.rs:108-121](../../rhwp-server/src/main.rs#L108-L121) 에 박혀 있다. `RHWP_DEFAULT_USER` 를 안 주면 *모든 API 가 400 으로 막힌다* — 가장 흔한 첫 사고 자리다.

### 1.2 포트 충돌

기본 포트 `7710` 이 이미 점유 중이면 서버가 `bind 실패: Address already in use` 로 즉시 죽는다. 다른 포트로 옮기려면 다음과 같이 한다.

```bash
RHWP_SERVER_ADDR=127.0.0.1:7712 RHWP_DEFAULT_USER=jerry cargo run --release
```

이후 모든 호출에서 `7710` 자리를 `7712` 로 바꿔 읽으면 된다.

---

## 2. 파일을 `file_id` 슬롯에 등록한다

`file_id` 는 *파일 보관함의 어느 칸을 가리키는 주소표* 다. `rhwp-server` 가 부여하는 게 아니라, **호출자가 임의로 정한 문자열** 을 그대로 슬롯 키로 쓴다 ([rhwp-server/src/main.rs:128-134](../../rhwp-server/src/main.rs#L128-L134)).

엔드포인트는 `POST /hwp/sessions`. 모든 라우트가 `/hwp/` 아래에 묶여 있다는 점을 기억한다 ([rhwp-server/src/main.rs:1474-1524](../../rhwp-server/src/main.rs#L1474-L1524)). prefix 를 빼먹으면 404 가 나온다.

```bash
FILE=/path/to/your.hwp   # 또는 .hwpx
FID=my-file-1            # 원하는 임의 식별자

TMP=$(mktemp -t rhwp.XXXX.json)
B64=$(base64 -i "$FILE" | tr -d '\n')
printf '{"fileId":"%s","format":"hwp","fileBase64":"%s"}' "$FID" "$B64" > "$TMP"

curl -X POST http://127.0.0.1:7710/hwp/sessions \
  -H "Content-Type: application/json" \
  --data-binary @"$TMP"
rm "$TMP"
```

응답 예:

```json
{"fileId":"my-file-1","seq":0,"sectionCount":1,"paragraphCount":468}
```

확장자가 `.hwpx` 라면 `"format":"hwp"` 를 `"hwpx"` 로 바꾼다.

### 왜 `POST /hwp/documents` 가 아니라 `POST /hwp/sessions` 인가

studio 의 *파일 열기 UI* 가 호출하는 `POST /hwp/documents` 는 file_id 를 외부 저장 서비스가 발급해 주는 흐름이다. `UPLOAD_URL` 이 비어 있는 dev 환경에선 [storage.rs:69-71](../../rhwp-server/src/storage.rs#L69-L71) 의 가드에 걸려 *반드시 500* 을 돌려준다. `POST /hwp/sessions` 는 호출자가 직접 `fileId` 를 지정하므로 외부 저장 서비스 없이 동작한다 — 이게 dev 환경에서 안전한 진입점이다.

---

## 3. IR 추출

세 가지 엔드포인트가 있다. 모두 `GET` 요청이며 `X-Rhwp-User` 헤더는 (서버에 `RHWP_DEFAULT_USER` 가 설정돼 있으면) 생략 가능하다.

### 3.1 전체 IR

```bash
curl http://127.0.0.1:7710/hwp/sessions/my-file-1/ir | jq .
```

문서 전체를 한 덩어리 JSON 으로 돌려준다. 사용자에게 전달한 예시 파일(31 페이지, 문단 468 개) 기준 약 94 KB.

### 3.2 페이지별 IR

```bash
curl "http://127.0.0.1:7710/hwp/sessions/my-file-1/ir?page=0" | jq .
```

`page` 인자는 0 부터 시작한다.

### 3.3 모델 친화적 compact 슬라이스

문단 범위를 잘라 *언어 모델이 읽기 편한 평탄 IR* 형식으로 받는다. `mode=compact` 권장 ([rhwp-server/src/ir_compact.rs:1-35](../../rhwp-server/src/ir_compact.rs#L1-L35)).

```bash
curl "http://127.0.0.1:7710/hwp/sessions/my-file-1/ir-slice?sec=0&para_start=0&para_end=10&mode=compact" | jq .
```

- `sec` — 섹션 인덱스 (0 부터)
- `para_start` / `para_end` — 문단 범위. 끝은 *exclusive*

---

## 4. (선택) rhwp-studio 로 같은 세션을 *눈으로* 확인하기

studio 는 브라우저에서 동작하는 한 글 편집기 UI 다. 위에서 만든 세션을 `fileId` 로 가리키면, studio 부팅 시 `GET /sessions/{file_id}/export` 를 통해 *현재 서버 상태를 그대로 화면에 복원* 한다 ([rhwp-studio/src/main.ts:535-547](../../rhwp-studio/src/main.ts#L535-L547)).

### 4.1 studio 띄우기

```bash
cd UNIVA-rhwp/rhwp-studio
npm install
npm run dev
```

`Local: http://127.0.0.1:7700/hwp/` 가 뜨면 준비 완료.

### 4.2 진입 URL

```
http://127.0.0.1:7700/hwp/?ssrBase=http://127.0.0.1:7710/hwp&fileId=my-file-1
```

쿼리 두 개의 의미:

- `ssrBase` — studio 가 API 호출 시 *어느 서버에 보낼지* 의 기준 주소. **반드시 끝에 `/hwp` 까지 같이 박는다.** studio 는 이 값에 `/sessions/...` 를 단순 결합하므로 ([rhwp-studio/src/main.ts:81-82, 133, 506](../../rhwp-studio/src/main.ts#L81-L82)), `/hwp` 가 빠지면 서버 라우트 prefix 와 어긋나 404 가 난다.
- `fileId` — 4.1 에서 만든 세션 식별자.

화면에 그 문서가 그대로 뜨면 정상.

### 4.3 studio 에서 새 파일을 *드래그&드롭으로 올리는* 시도는 dev 환경에선 막혀 있다

studio 의 파일 열기 동작은 내부적으로 `POST /hwp/documents` 를 호출한다. 위 2장 말미에서 다룬 대로 — 외부 저장 서비스가 켜져 있어야 하는 경로라 dev 환경에선 500 으로 떨어지고, *주소창의 `fileId` 도 갱신되지 않는다*. 대신 본 가이드의 2장처럼 `curl` 로 세션을 직접 만들고, studio 는 `fileId` 쿼리로 *복원* 만 한다.

---

## 5. 문제 해결 표

| 증상 | 진짜 원인 | 처치 |
| --- | --- | --- |
| `{"error":"사용자 식별자 누락 ..."}` HTTP 400 | 서버에 `RHWP_DEFAULT_USER` 가 없고 요청에 `X-Rhwp-User` 도 안 붙음 | 서버를 `RHWP_DEFAULT_USER=...` 로 재기동하거나, 호출에 `-H "X-Rhwp-User: jerry"` 를 박는다 |
| HTTP 404 (라우트 매칭 실패) | URL 에 `/hwp` prefix 누락 | 모든 경로 앞에 `/hwp/` 를 박는다. 서버 라우트는 [rhwp-server/src/main.rs:1474-1524](../../rhwp-server/src/main.rs#L1474-L1524) 에서 한꺼번에 nest 된다 |
| `{"error":"저장소 업로드 실패 ..."}` HTTP 500 | `POST /hwp/documents` 호출 — 외부 저장 서비스가 비활성 | `POST /hwp/sessions` 로 우회 (본 가이드 2장) |
| `{"error":"세션 없음: ..."}` IR 조회 시 | 그 `file_id` 로 세션이 만들어진 적이 없음 | 같은 `file_id` 로 2장의 등록을 먼저 수행 |
| studio 진입 후 주소창에 `fileId` 가 갱신되지 않음 | studio 의 새 파일 업로드가 위 500 사고로 실패. studio 는 응답 본문을 안 찍고 status 만 콘솔에 남김 (`[SSR] 문서 업로드 실패 500`) | 2장 우회로로 미리 세션을 만들어 두고, studio 는 `fileId` 쿼리로 복원 진입 |
| 빌드는 됐는데 `bind 실패: Address already in use` 로 서버가 죽음 | 같은 포트에 다른 프로세스가 떠 있음 | `lsof -nP -iTCP:7710 -sTCP:LISTEN` 으로 확인. 죽일 게 아니라면 `RHWP_SERVER_ADDR=127.0.0.1:7712` 로 옮긴다 |

---

## 6. 한 줄 요약

`rhwp-server` 를 `RHWP_DEFAULT_USER` 와 함께 띄우고 → `curl` 로 `POST /hwp/sessions` 에 base64 본문을 던져 *임의의 `file_id`* 슬롯에 파일을 등록한 뒤 → `GET /hwp/sessions/{file_id}/ir` (또는 `/ir-slice`) 로 IR 을 뽑는다. studio 는 그 세션을 `fileId` 쿼리로 *복원만* 한다 — 직접 업로드는 dev 환경에서 막혀 있다.
