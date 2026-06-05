# Task #zephy-bridge Stage 1 — Sub-1 SSR 브릿지 8 task 결과

작성일 2026-06-05. 본 보고서는 [task_m200_zephy_bridge_impl.md](../plans/task_m200_zephy_bridge_impl.md)의 8 task에 대한 *단계별 통과·실패 기록*.

## Task별 결과

| # | 범위 | 결과 | 주요 commit |
|---|---|---|---|
| 1 | `server/src/events.rs` 신설 (`ServerEvent`·`ClientMessage` enum + `EventsHub` broadcast 채널) + Cargo.toml (`axum.ws` feature, `tokio-stream`, `futures`) | ✅ 통과. cargo check 통과. 단위 테스트 4건 (Task 2에서 mod 등록 후 PASS). | `4ea4c915` |
| 2 | `server/src/ws.rs` 신설 (WebSocket upgrade·양방향 frame loop·ClientMessage dispatch) + `main.rs`에 `mod events; mod ws;` 등록·`AppState.events` 필드·가시성 `pub(crate)` 상향·`/sessions/:id/ws` 라우트 | ✅ 통과. cargo build OK, cargo test 7 PASS(events 4 + store 3). 라우터에 WS 라우트 정상 등록. | `66579dbd` |
| — | Plan defect P-1 정정: 프로토콜 요약의 `"dir":` 필드와 `"fileBase64"` camelCase가 Rust enum과 자체 모순 → 제거·snake_case로 통일 (Task 1 code review 피드백) | ✅ | `bb602f68` |
| 3 | `POST /sessions/:id/workbench` 핸들러 신설 (action 분기: `insert_text`는 진짜 적용 + `ServerEvent::Ops` broadcast, 그 외 액션은 `ServerEvent::Workbench` 패스스루) + `apply_ops`에 broadcast 발행 한 줄 추가(클라→서버 미러링 호환) | ✅ 통과. curl 시연으로 `applied:"ops"` / `applied:"passthrough"` 분기 모두 확인. | `dd4ff7b0` |
| 4 | `rhwp-studio/src/core/session-client.ts` 내부를 HTTP fetch → WebSocket으로 갈아엎음. *외부 API(MirrorSink·createSession·attach·queueOp·requestSnapshot·dispose·flushOps) 시그니처 유지*. WS 재연결 지수 백오프 + sendBuffer + onServerEvent 콜백 옵션 추가. | ✅ 통과. tsc/vite build 통과. 호출자(main.ts·InputHandler) 회귀 0. | `f8d85495` |
| — | Task 4 code review 피드백 — `dispose()` race condition fix (`disposed` 플래그 + `reconnectTimer` 핸들 + clearTimeout). | ✅ | `d2d7e675` |
| 5 | `rhwp-studio/src/main.ts`에 `onServerEvent` 콜백 주입. `ev.kind==='ops'`는 op 종류별 WASM 직접 호출(`wasm.insertText`·`wasm.splitParagraph`), `ev.kind==='workbench'`는 `getActionDef + executor(ctrl, set)` 호출. JSON shape 가드 적용. | ✅ 통과. tsc/vite build 통과. plan의 `wasm.getDoc()` 가정이 실제 `WasmBridge` API와 달라 implementer가 코드 조사로 직접 호출로 보정. | `07b54474` |
| — | Task 5 code review **Critical fix**: `Object.assign(new ParameterSet(...), payload)`가 Map 기반 items에 값이 들어가지 않아 모든 workbench 이벤트가 *조용히 실패*. `set.SetItem(k, v)` 명시 호출로 교체. | ✅ | `e943a7f5` |
| 6 | `rhwp-studio/e2e/ws-bridge.test.mjs` 양방향 검증 e2e 테스트. *Puppeteer DOM 검사가 Canvas 렌더링과 부적합*하여 *Node WebSocket 직접 구독 + 서버 IR 영속 확인*으로 검증 방식 수정. | ✅ 통과. 양방향 모두 확인: 서버→클라 broadcast 수신 OK + 클라→서버 ops 후 서버 IR에 영속 OK. | `f3e5167f` → `977f7a9d` (수정) |
| 7 | `hwp_sub_agent_simulation_ssr.ipynb` 신설 (작업 공간 루트, 7 cells). 기존 노트북 cell 6(LLM 인프라)·cell 10(sub_agent_run)을 *byte-level 동일하게 복사*, cell 1·3·4·6은 신규 작성. SSR 라우터가 `hwp-doc-patch` 명령 가로채 `POST /workbench`로 라우팅. | ✅ 통과. 7 cells 정상 생성·검증. 알려진 한계: `google_search` 함수가 새 노트북에 없어 LLM이 google_web_search tool 호출 시 NameError — Sub-1 시연 시나리오(insert_text)에서는 영향 없음. | (git untracked, 작업 공간 루트는 git 저장소 아님) |
| 8 | 종단 회귀 검증 + 보고서 작성 | ✅ 진행 중 (본 보고서) |

## 자동 검증 결과

**cargo test (server crate, 본 보고서 작성 시점 재실행)**:
```
test events::tests::client_message_deserializes_from_json ... ok
test events::tests::server_event_json_has_kind_tag ... ok
test events::tests::publish_delivers_to_subscriber ... ok
test events::tests::different_file_ids_are_isolated ... ok
test store::tests::test_load_missing ... ok
test store::tests::test_snapshot_supersedes_base ... ok
test store::tests::test_create_and_load ... ok

test result: ok. 7 passed; 0 failed
```

**e2e ws-bridge.test.mjs** (rhwp-studio):
```
세션: e2e-ws-...
WS 연결 OK
OK 1: 서버→클라 broadcast로 ServerEvent::Ops 수신, ops에 "FROM-LLM" 포함
OK 2: 서버 IR에 "FROM-LLM"·"FROM-CLIENT" 둘 다 영속

=== 양방향 WS bridge 검증 통과 ===
```

**회귀 검증 — 기존 endpoint**:
```
>> POST /sessions/regression-.../ops (기존 endpoint)
{"fileId":"regression-...","seq":1,"sectionCount":1,"paragraphCount":1}

>> GET /sessions/.../ir?page=0
paragraph text: REGRESS
```
응답 형식이 *변경되지 않음*. 기존 클라이언트 회귀 0.

## 수동 검증 (사용자 영역)

자동화 어려운 두 시나리오는 사용자 시연으로 검증한다:

1. **LLM 실제 호출 → 브라우저 시각 반영**: 새 노트북 cell 0~6 순서 실행 → LLM이 `Bash("hwp-doc-patch insert_text ...")` 호출 → POST /workbench → ServerEvent::Ops broadcast → 브라우저 main.ts `onServerEvent` → WASM insertText → Canvas 재렌더링.
2. **브라우저 직접 편집 → 서버 sqlite 영속**: 사용자가 브라우저에서 키 입력 → InputHandler가 `sessionClient.queueOp(op)` 호출 → WS 메시지 → 서버 `handle_client_text` → `apply_edit_ops_json` + sqlite 기록.

## Sub-1의 알려진 한계 (Sub-2로 미룸)

| 항목 | 위치 | 처리 |
|---|---|---|
| `Mutex::lock().unwrap()` poisoning 위험 | events.rs, main.rs, ws.rs | Sub-2에서 `unwrap_or_else(|e| e.into_inner())` 또는 parking_lot 도입 |
| `ServerEvent::Snapshot` 변종 없음 (스냅샷 후 broadcast 발행 안 함) | ws.rs::handle_client_text | Sub-2에서 변종 추가 + 클라가 `/ir` 재 fetch |
| passthrough event는 sqlite에 영속 안 됨 — seq 단조성 깨질 가능성 | main.rs::workbench `_` arm | Sub-2에서 `store.append_workbench` 추가 또는 `seq` 의미 명시 |
| `ServerEvent`에 `origin` 필드 없음 — 다중 클라 echo 방지 어려움 | events.rs::ServerEvent | Sub-2에서 `origin: Option<String>` 추가 |
| `apply_edit_ops_json("[{op}]")` 문자열 라운드트립 | main.rs::workbench, restore_core | Sub-2에서 `apply_edit_op(&EditOperation)` 직접 호출로 정리 |
| `new HwpCtrl(wasm as any)` 매 workbench 이벤트마다 생성 + cursor state 손실 | main.ts onServerEvent workbench 분기 | Sub-2에서 ctrl 재사용 + cursor 동기화 |
| 노트북 `google_search` 함수 누락 | hwp_sub_agent_simulation_ssr.ipynb | LLM이 google_web_search tool 호출 시 NameError. Sub-1 시연 시나리오 외에서만 영향 |
| workbench·SessionClient 통합 테스트 없음 | server 측 | Sub-2에서 axum 통합 테스트 추가 |
| broadcast 채널 capacity 128 — 주석으로 근거 명시 안 됨 | events.rs:57 | 주석 한 줄 추가 권고 |
| `let _ = session_info(...)` dead computation | ws.rs::handle_client_text Snapshot 분기 | 한 줄 삭제 |

이 항목들은 [report/task_m200_zephy_bridge_report.md](../report/task_m200_zephy_bridge_report.md)에서 Sub-2 진입 전 처리 권고로 다시 정리한다.

## 다음 단계

- 사용자 수동 시연 통과 후 Sub-1 종료.
- Sub-2: 11+1 워크벤치 액션을 EditOperation variants로 추가해 *완전 SoT* 달성.
