# 09 — UNIVA-rhwp SSR 통합 Sub-1 설계: LLM 노트북 ↔ SSR 서버 ↔ rhwp-studio 실시간 양방향 브릿지

작성일 2026-06-05. 본 문서는 *접합 인프라*만 다루는 Sub-1 설계서. Sub-2(액션 확장 합본)는 별도 문서로 따라온다. *2026-06-05 갱신*: SSE 한 방향 push에서 *WebSocket 양방향*으로 프로토콜 변경.

## 1. 배경

작업 공간에는 두 시스템이 별개로 살아 있다.

**(A) 원본 `rhwp/`** — Rust + WebAssembly 기반 HWP/HWPX 뷰어·에디터. 동작 모델은 *브라우저 안에서 WASM이 직접 파일을 파싱·렌더링*. `hwp_sub_agent_simulation.ipynb` 시뮬레이션 노트북이 다음 흐름으로 LLM 편집을 시연한다.

```
노트북 LLM → Bash 도구로 hwp-doc-patch CLI 호출
   → CLI가 노트북 안 bridge 서버(127.0.0.1:8765)에 POST
   → bridge가 SSE로 브라우저 embed-host 페이지에 전달
   → 브라우저 안 rhwp WASM이 실제 편집을 수행하고 결과를 /result/<id>로 회신
```

**(B) `UNIVA-rhwp/`** — 같은 rhwp 본체에 *서버 측 세션 영속성*을 추가한 변형. `rhwp-server` axum 바이너리가 `DocumentCore` 인스턴스를 sqlite에 저장하면서 [server/src/main.rs](../../server/src/main.rs)의 API를 같은 포트(7710)에서 노출한다. studio 정적 자산도 같은 포트에서 같이 서빙한다.

| 기존 endpoint | 용도 |
|---|---|
| `POST /sessions` | fileId + 원본 바이트로 세션 생성 |
| `POST /sessions/:id/ops` | `[EditOperation, …]` 적용 |
| `PUT /sessions/:id/snapshot` | 전체 스냅샷 동기화 |
| `GET /sessions/:id/ir` | 현재 상태 IR을 JSON으로 반환 (페이지 필터) |
| `GET /sessions/:id/export` | hwp/hwpx 바이너리로 내보내기 |
| `POST /sessions/:id/save` | minio 덮어쓰기 |
| `DELETE /sessions/:id` | 메모리 세션 해제 |

여기서 *(A)의 LLM 편집 파이프라인*을 *(B)의 SSR 서버*에 접합하여, 노트북이 보내는 편집 명령이 서버를 거쳐 브라우저로 실시간 반영되는 구조를 만든다. *브라우저 ↔ 서버 채널은 WebSocket 양방향*이라 사용자가 직접 편집한 내용도 같은 채널로 서버에 미러링된다.

## 2. 목표와 비목표

**목표 (Sub-1)**:

1. LLM이 노트북에서 보낸 편집 명령이 *SSR 서버를 거쳐* 브라우저 rhwp-studio 화면에 실시간 반영되는 *접합 인프라*를 완성한다.
2. 브라우저 ↔ 서버를 *WebSocket 양방향* 채널로 통합한다. 기존 클라→서버 미러링(`session-client.ts`)을 *WebSocket 메시지로 갈아엎고*, 서버→클라 push도 같은 채널에 합친다.
3. 인프라 검증을 위해 `insert_text` 명령 1개는 서버가 자기 `DocumentCore`에 *진짜로 적용*하고 sqlite에 영속화한다.
4. 그 외 11+1개 LLM 명령은 *그대로 전달*(passthrough)되어 클라이언트 hwpctl이 처리한다. 서버 상태와 클라이언트 상태가 일시적으로 어긋나는 *알려진 한계*는 Sub-2에서 해소한다.

**비목표 (Sub-2로 미룸)**:

- `replace_runs`, `set_paragraph_style`, `insert_table`, `merge_cells`, 셀·표 편집 11+1개 액션의 서버 진짜 적용
- WebSocket 메시지 순번(`seq`) 기반 미수신 재전송
- 다중 사용자 동일 세션 협업 충돌 해결
- 기존 `hwp_sub_agent_simulation.ipynb`·bridge(8765)·embed-host 흐름 변경(*손대지 않음*)

## 3. 전체 로드맵 중 Sub-1의 위치

| 번호 | 범위 | 산출물 |
|---|---|---|
| **Sub-1 (본 spec)** | 접합 인프라 + WebSocket 양방향 채널 + `insert_text` 1개 진짜 적용 | events 모듈, ws handler, workbench endpoint, session-client WS 갈아엎기, 새 노트북 |
| **Sub-2** | 나머지 11+1 액션을 `EditOperation` variants로 추가 | edit_op.rs 확장, workbench match 분기 채우기. 1~2 세션 추정 |

Sub-1을 마치고 *insert_text 종단 시연*이 통과하면 Sub-2 brainstorm을 새로 시작한다.

## 4. 컴포넌트와 책임

| # | 컴포넌트 | 위치 | 책임 | 외부 의존 |
|---|---|---|---|---|
| 1 | events 타입 + broadcast | `server/src/events.rs` (신설) | `ServerEvent`/`ClientMessage` enum + `EventsHub`(세션별 broadcast 채널) | `tokio::sync::broadcast`, serde |
| 2 | WebSocket handler | `server/src/ws.rs` (신설) | `GET /sessions/:id/ws` upgrade, 프레임 loop, 클라 메시지 dispatch | axum ws feature, futures |
| 3 | workbench 핸들러 | `server/src/main.rs` (수정) | `POST /sessions/:id/workbench`, action별 분기, broadcast 발행 | events, DocumentCore |
| 4 | 클라 양방향 WS 클라이언트 | `rhwp-studio/src/core/session-client.ts` (내부 갈아엎기) | *외부 API 유지*(MirrorSink, queueOp, requestSnapshot, attach, createSession), *내부를 HTTP fetch → WebSocket으로*, *서버→클라 수신 콜백 통합* | 브라우저 `WebSocket` API |
| 5 | main.ts entry | `rhwp-studio/src/main.ts` (수정) | `SessionClient`에 `onServerEvent` 콜백 주입 — ops/workbench 분기 | hwpctl ActionRegistry, WASM bridge |
| 6 | 새 노트북 | `hwp_sub_agent_simulation_ssr.ipynb` (신설, 작업 공간 루트) | 세션 시작 helper, Bash 실행기 *HTTP* 라우팅, LLM agentic loop | `requests`, 기존 vLLM 인프라 |

각 컴포넌트는 한 단위의 책임. *서버 PR*과 *클라 PR*과 *노트북 PR*을 독립적으로 만들고 통합하는 것이 자연스럽다.

## 5. 데이터 흐름

```
┌─ 노트북: hwp_sub_agent_simulation_ssr.ipynb ────────────────────────────────┐
│                                                                              │
│  cell 2 (세션 시작):                                                          │
│    file_id, info = create_session(<빈 hwpx 바이트>)                         │
│      → requests.post('http://127.0.0.1:7710/sessions')      ─HTTP─▶          │
│    print(file_id, URL)                                                       │
│                                                                              │
│            사용자가 출력된 URL을 브라우저에 직접 입력                          │
│                                                                              │
│  cell 12 (LLM agentic loop):                                                 │
│    sub_agent_run("...")                                                      │
│      └─ tool: Bash("hwp-doc-patch insert_text --file-id X --payload '...'") │
│            └─ run_bash_command(cmd)  ─── 라우팅 분기                         │
│                  ├─ cmd가 'hwp-doc-patch '로 시작:                          │
│                  │    parse → (action, payload, file_id)                     │
│                  │    POST /sessions/{fid}/workbench  ─HTTP─▶ 서버 ◇         │
│                  │       body: {action, payload}                              │
│                  │       ◀── {seq, applied, info} 응답                        │
│                  └─ 그 외 명령(cat, ls 등): subprocess.run                  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
                                                  │
┌─ rhwp-server (axum, 7710) ───────────────────────▼──────────────────────────┐
│                                                                              │
│  POST /sessions/:id/workbench ◇  (HTTP, 노트북 전용)                         │
│    action별 분기:                                                              │
│    ├─ "insert_text" → EditOperation::InsertText 변환                          │
│    │   → core.apply_edit_ops_json([op]) → store.append_op                    │
│    │   → events.publish(ServerEvent::Ops)                                    │
│    │   응답: {seq, applied:"ops", info}                                       │
│    └─ 그 외 → events.publish(ServerEvent::Workbench)                         │
│        응답: {seq, applied:"passthrough"}                                     │
│                                                                              │
│  GET /sessions/:id/ws  (WebSocket upgrade, 브라우저 전용)                    │
│    양방향 텍스트 프레임:                                                       │
│      서버 → 클라: ServerEvent                                                  │
│         {"kind":"ops", seq, ops}                                              │
│         {"kind":"workbench", seq, action, payload}                            │
│      클라 → 서버: ClientMessage                                                │
│         {"kind":"ops", ops}        → apply_edit_ops_json + broadcast 발행    │
│         {"kind":"snapshot", file_base64} → put_snapshot 인라인               │
│         {"kind":"ping"}            → no-op                                    │
│                                                                              │
│  기존 GET /sessions/:id/ir (초기 로드용, 변경 없음)                          │
└──────────────────────────────────────────────────│──────────────────────────┘
                                                  │
┌─ 브라우저: rhwp-studio (?fileId=<id>) ───────────▼──────────────────────────┐
│                                                                              │
│  main.ts entry에서 SessionClient 생성 (기존 코드 일부 수정)                  │
│    new SessionClient({                                                        │
│       baseUrl: window.location.origin,                                       │
│       fileId,                                                                 │
│       getSnapshotBytes: () => wasm.exportHwpx(),                             │
│       onServerEvent: (ev) => { ... ops/workbench 분기 ... },                 │
│    })                                                                         │
│                                                                              │
│  SessionClient 내부:                                                          │
│    open WS → onmessage → JSON.parse → onServerEvent 콜백                    │
│    InputHandler가 queueOp(op) 호출 → 디바운스 → WS send {kind:"ops", ops}   │
│                                                                              │
│  onServerEvent 콜백(main.ts):                                                │
│    ev.kind=="ops"        → wasmCore.insertText / splitParagraph             │
│    ev.kind=="workbench"  → getActionDef(ev.action).executor(ev.payload)     │
│    → studio 자동 재렌더링                                                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 6. 인터페이스

### 6.1 `POST /sessions/:id/workbench` (신설, HTTP)

**요청**
```json
{ "action": "insert_text",
  "payload": { "section": 0, "para": 0, "offset": 0, "text": "안녕하세요" } }
```

**응답 200**
```json
{ "seq": 1,
  "applied": "ops",
  "info": { "fileId": "demo-...", "seq": 1, "sectionCount": 1, "paragraphCount": 1 } }
```

- `applied: "ops"` — 서버가 `EditOperation`으로 변환해 자기 `DocumentCore`에 진짜 적용. Sub-1에서는 `insert_text` 1개만.
- `applied: "passthrough"` — 서버는 broadcast만, 자기 상태는 변경하지 않음. Sub-2 완료 시 이 분기 사라짐.

**에러**: 400(payload 형식 오류), 404(세션 없음), 422(`EditOperation` 변환 실패), 500.

### 6.2 `GET /sessions/:id/ws` (신설, WebSocket upgrade)

axum의 `WebSocketUpgrade`로 응답. 양방향 텍스트 프레임. 모든 본문은 한 줄 JSON.

**서버 → 클라 (`ServerEvent`)**:

```
{"kind":"ops","seq":42,"ops":[{"op":"insert_text","section":0,"para":0,"offset":0,"text":"가"}]}
{"kind":"workbench","seq":43,"action":"replace_runs","payload":{ … }}
```

**클라 → 서버 (`ClientMessage`)**:

```
{"kind":"ops","ops":[{"op":"insert_text",…}]}
{"kind":"snapshot","file_base64":"<base64>"}
{"kind":"ping"}
```

연결 시 *현재 상태 스냅샷을 보내지 않는다*. 클라이언트는 WS 구독 *직전 또는 직후*에 `GET /ir`로 초기 상태를 받는다. WS가 끊겨서 다시 붙으면 그동안 발행된 이벤트는 *유실*된다 — 클라이언트가 다시 `GET /ir`로 복원. WS 재연결은 클라이언트에서 *지수 백오프*로 자동.

세션이 없거나 사용자 확보 실패 시 서버가 `CloseFrame { code: 4404, reason: "session not found" }`로 닫는다.

### 6.3 노트북 Bash 실행기 라우팅 (의사코드)

```python
HWP_DOC_PATCH_PREFIX = 'hwp-doc-patch '
SESSION_FILE_ID: str  # cell 2에서 채워지는 전역
SSR_BASE = 'http://127.0.0.1:7710'

def run_bash_command(command: str) -> dict:
    cmd = command.strip()
    if cmd.startswith(HWP_DOC_PATCH_PREFIX):
        action, file_id_arg, payload = parse_hwp_doc_patch_call(cmd)
        fid = file_id_arg or SESSION_FILE_ID
        r = requests.post(
            f'{SSR_BASE}/sessions/{fid}/workbench',
            json={'action': action, 'payload': payload},
            timeout=30,
        )
        body = r.json()
        return {
            'exit_code': 0 if r.ok else 1,
            'stdout': format_as_sentinel_json(body) if r.ok else '',
            'stderr': '' if r.ok else json.dumps(body),
            'truncated': False,
        }
    return run_normal_subprocess(cmd)  # cat, ls 등은 기존 동작
```

- `format_as_sentinel_json`은 기존 SKILL.md가 LLM에게 가르친 `<<<HWP_DOC_PATCH_JSON_BEGIN>>>{...}<<<HWP_DOC_PATCH_JSON_END>>>` 형식.
- 노트북은 WebSocket을 사용하지 않는다 — LLM ↔ 노트북 ↔ 서버 통신은 요청-응답 패턴이라 HTTP가 자연스럽다.

## 7. 에러와 엣지 처리

| # | 케이스 | 서버 응답 | 노트북 처리 | 클라이언트 처리 |
|---|---|---|---|---|
| 1 | 세션 미존재 (HTTP) | 404 `{error: "session not found"}` | stderr → LLM에 전달 | — |
| 2 | 세션 미존재 (WS) | `CloseFrame {code:4404}` | — | 클라가 close 받으면 사용자에 안내. 재연결 시도 X (404는 영구). |
| 3 | insert_text payload 형식 오류 | 422 + 사유 | stderr → LLM 재시도 | — |
| 4 | WS 끊김(네트워크) | 채널 유지 | 영향 없음 | 지수 백오프 재연결([500,1000,2000,5000,10000]ms). 유실 이벤트 복구 안 함 — 즉시 `GET /ir`로 동기화. |
| 5 | 미지원 액션 (Sub-1 시점 `insert_text` 외 전부) | 200 + `applied:"passthrough"` + WS `kind:"workbench"` 발행 | 정상 응답 → LLM 진행 | hwpctl로 실행. *서버 IR과 클라 IR 분기* — Sub-2에서 해소 |
| 6 | 사용자 새로고침/탭 닫음 | broadcast 채널 살아있음 (수신자 0 허용) | 영향 없음 | 새 진입 시 `GET /ir`로 fresh start, 그동안 발행된 이벤트 유실 |
| 7 | 동시 LLM 호출(이론) | `apply_ops`가 Mutex 직렬화, seq 단조 증가 | — | WS 메시지 순서 보존, 순서대로 적용 |
| 8 | 클라가 WS 닫혀 있는 동안 편집(InputHandler) | — | — | `sendBuffer`에 누적, WS 재연결 시 자동 flush |
| 9 | base64/JSON 파싱 실패 (cell 2 세션 생성) | 400 + 사유 | cell 2 helper가 명확한 에러 메시지 | — |

## 8. 테스트 전략

### 수동 시연 시나리오

1. `rhwp-server` 가동
2. 새 노트북 cell 2 실행 → fileId 발급, 빈 hwpx 세션 생성, URL 출력
3. 브라우저로 `http://127.0.0.1:7710/?fileId=<id>` 접속 → 빈 페이지 + DevTools Network → WS 탭에 연결 보임
4. cell 12 실행 → LLM에 *"첫 문단에 '안녕하세요' 삽입"* 요청
5. 기대 결과: LLM이 `Bash("hwp-doc-patch insert_text ...")` 호출 → 워크벤치 라우팅 → 서버 진짜 적용 → WS 텍스트 프레임 → *브라우저 화면에 '안녕하세요' 즉시 표시*
6. 추가: LLM이 `replace_runs` 같은 미지원 액션 호출 → WS `kind:"workbench"` → 클라가 hwpctl 실행. 화면 반영 OK, 서버 IR은 그대로(Sub-1 한계)
7. *양방향 검증*: 브라우저에서 사용자가 직접 편집 → InputHandler가 `sessionClient.queueOp(op)` → WS 메시지 → 서버 sqlite 기록 → 다른 탭 열어 같은 fileId 들어가면 즉시 보임

### 자동화 테스트

| 단위 | 도구 | 검증 |
|---|---|---|
| 서버 events 모듈 | `cargo test`(server crate) | `EventsHub` broadcast, `ServerEvent`/`ClientMessage` JSON 직렬화·역직렬화 |
| 서버 workbench 핸들러 | 수동 curl 시나리오 | `insert_text` → `applied:"ops"`, 그 외 → `applied:"passthrough"` |
| 서버 WS endpoint | wscat 또는 e2e | WS 핸드셰이크 + 양방향 메시지 처리 |
| 클라 WS bridge | `rhwp-studio/e2e/ws-bridge.test.mjs` (Puppeteer headless, 기존 [text-flow.test.mjs](../../rhwp-studio/e2e/text-flow.test.mjs) 패턴 답습) | 서버→클라(workbench → WS push → DOM 반영) + 클라→서버(WS send → 서버 IR 영속) 둘 다 |
| 노트북 Bash 라우팅 | 노트북 안에 self-test cell | `hwp-doc-patch …` 명령 파싱 + 실제 HTTP 호출 형태 검증 |

LLM 호출을 포함하는 end-to-end 자동 테스트는 *Sub-1에서 생략* — 비결정적이라 수동 시연으로 충분.

### 회귀 방지

- 기존 `hwp_sub_agent_simulation.ipynb`·bridge·embed-host는 *손대지 않는다* — 기존 시뮬 회귀 0.
- 서버의 기존 `/sessions`, `/ops`, `/ir`, `/export`, `/save`, `/snapshot` 핸들러는 *변경하지 않는다*(단, `/ops` 핸들러 내부에 broadcast 발행 한 줄만 추가 — 응답 형식 동일). 기존 클라이언트 동작 회귀 0.
- 클라의 `session-client.ts` *외부 API는 유지*. `MirrorSink` 인터페이스·`queueOp`·`requestSnapshot`·`attach`·`createSession` 시그니처 동일. 호출자(InputHandler, main.ts의 mirrorSink 주입) 손대지 않음.
- 새 옵션 `onServerEvent`는 *선택적* — 기존 사용처가 안 줘도 동작(undefined check). 점진 활용.

## 9. Sub-1 완료 기준 (Definition of Done)

1. 수동 시연 시나리오 7단계가 모두 통과한다.
2. 자동화 테스트 5단위가 모두 통과한다.
3. 회귀 0이 검증된다(기존 노트북 시뮬·기존 studio dist 동작 확인).
4. 본 spec의 모든 인터페이스(6.1·6.2·6.3)가 코드에 그대로 구현된다.
5. WebSocket 양방향 채널이 *실제로 양쪽 다 흐름*을 확인 — 서버→클라(workbench 결과) + 클라→서버(InputHandler 편집).

## 10. 위험과 가정

**가정 1.** `doc.insertText` WASM API가 `DocumentCore::insert_text` Rust 메서드와 1:1 대응. 만약 WASM 노출 함수가 cursor 의존성 같은 클라이언트 한정 처리를 끼워 넣었다면 Sub-2에서 매핑이 약간 늘어난다. spec 검증 단계에서 [src/wasm_api.rs](../../src/wasm_api.rs)를 확인한다.

**가정 2.** `tokio::sync::broadcast` 채널이 LLM 호출 빈도(분당 수십회 안팎)를 무리 없이 처리한다. PoC 단계에서 메모리 한계는 발생하지 않는다.

**가정 3.** 브라우저 표준 `WebSocket` API의 자동 close + 클라 재연결 백오프가 Sub-1 PoC 안정성에 충분하다. (Sub-2 이후 long-running 운영에서 더 정교한 재연결·재구독·미수신 이벤트 재전송이 필요해질 가능성 있음.)

**위험 1.** Sub-1 단계에서 *미지원 액션 패스스루*로 서버 IR과 클라 IR이 일시적으로 분기. Sub-2 합본 완료 시 자동 해소되지만, 그 전까지 *서버 측 `GET /ir` 응답으로 LLM이 문서 상태를 조회하는 흐름*에서 일부 액션 결과가 누락된 IR이 LLM에 노출될 수 있다. Sub-1 시연 시나리오에서는 `insert_text`만 검증하므로 영향 없으나, 시연 외 자유 형식 시뮬을 돌릴 때 이 점을 인지해야 한다.

**위험 2.** WS 재연결 시 유실 이벤트가 *복구 안 됨*. 시연 중 브라우저 새로고침 같은 케이스에서 `GET /ir`로 fresh start하는 것으로 충분하지만, 장시간 운영에서는 seq 기반 재전송이 필요해진다. 별도 sub로 미룬다.

**위험 3.** `session-client.ts` 내부를 WebSocket으로 갈아엎는 작업이 InputHandler 미러링 흐름(디바운스·beforeunload flush·snapshot 시점)을 *부분적으로 변경*. 외부 API는 유지하지만 *동작 타이밍이 미세하게 달라질 수 있어*, Sub-1 검증 시 기존 클라→서버 미러링 회귀를 명시적으로 확인해야 한다(§8 회귀 방지).

## 참고 — 코드 위치

본 문서는 `UNIVA-rhwp/mydocs/plans/`에 있으므로 UNIVA-rhwp 본체까지는 `../../`. 외부 레포(노트북·skills)는 작업 공간 루트까지 `../../../`.

- 서버 기존 핸들러: [server/src/main.rs:318-338](../../server/src/main.rs#L318-L338) (`apply_ops`)
- 서버 router 정의: [server/src/main.rs:451-476](../../server/src/main.rs#L451-L476)
- Rust `EditOperation` + `apply_edit_ops_json`: [src/document_core/commands/edit_op.rs](../../src/document_core/commands/edit_op.rs)
- TS `EditOperation` mirror: [rhwp-studio/src/engine/edit-op.ts](../../rhwp-studio/src/engine/edit-op.ts)
- 기존 클라 SSR 미러링: [rhwp-studio/src/core/session-client.ts](../../rhwp-studio/src/core/session-client.ts) (*Sub-1에서 내부를 WebSocket으로 갈아엎음, 외부 API는 유지*)
- hwpctl actions 카탈로그: [rhwp-studio/src/hwpctl/actions/](../../rhwp-studio/src/hwpctl/actions/)
- 기존 노트북: `../../../hwp_sub_agent_simulation.ipynb`
- 기존 LLM 시뮬 SKILL: `../../../26ZEPHY-skills/skills/document_edit/hwp-doc-edit/SKILL.md`
- 배포 가이드(Sub-1 인프라가 따르는 single-origin 모델): [deploy/README.md](../../deploy/README.md)
- 시각화: [task_m200_zephy_bridge_architecture.html](task_m200_zephy_bridge_architecture.html)
- 구현 계획서: [task_m200_zephy_bridge_impl.md](task_m200_zephy_bridge_impl.md)
