# Sub-1 SSR 브릿지 구현 계획 (task_m200_zephy_bridge_impl)

> **For agentic workers:** REQUIRED SUB-SKILL — Use `superpowers:subagent-driven-development` (권장) 또는 `superpowers:executing-plans`로 이 계획을 task별로 실행하세요. 단계는 `- [ ]` 체크박스 형식이며, 각 단계 완료 후 그 자리에서 체크.

**Goal:** LLM 노트북이 SSR 서버를 거쳐 브라우저 rhwp-studio에 실시간 편집 명령을 전달하는 *양방향 WebSocket* 접합 인프라를 완성하고, `insert_text` 1개의 종단 시연을 통과시킨다.

**Architecture:** axum 서버에 `events`(broadcast 채널 + 메시지 타입) + `ws`(WebSocket handler + 클라→서버 메시지 dispatch)를 추가하고, rhwp-studio의 기존 `session-client.ts` 내부 구현을 *HTTP fetch에서 WebSocket으로 갈아엎으면서 서버→클라 수신 로직을 통합*하고, 새 노트북에서 Bash 실행기를 SSR HTTP로 라우팅한다.

**Tech Stack:** Rust 1.93.1 · axum 0.7 (ws feature) · `tokio::sync::broadcast` · `tokio-stream` · `futures` · serde / TypeScript · Vite · 브라우저 표준 `WebSocket` API / Python 3 · `requests` · `openai` SDK

**Spec 출처:** [task_m200_zephy_bridge.md](task_m200_zephy_bridge.md). 시각화: [task_m200_zephy_bridge_architecture.html](task_m200_zephy_bridge_architecture.html).

**파일 구조 (생성·수정 한눈에):**

| 작업 | 경로 | 책임 |
|---|---|---|
| 생성 | `server/src/events.rs` | `ServerEvent`·`ClientMessage` enum + `EventsHub`(세션별 broadcast 채널) |
| 생성 | `server/src/ws.rs` | `ws_handler` — WS upgrade, 프레임 loop, 클라 메시지 dispatch |
| 수정 | `server/Cargo.toml` | `tokio-stream`·`futures` 의존성 + axum `ws` feature 명시 |
| 수정 | `server/src/main.rs` | `events`·`ws` 모듈 등록, `AppState.events` 필드, `/sessions/:id/ws`(GET) + `/sessions/:id/workbench`(POST) 라우트, `apply_ops`·`workbench`·`put_snapshot`에서 broadcast 발행 |
| 수정 | `rhwp-studio/src/core/session-client.ts` | *외부 API 유지(MirrorSink 인터페이스, queueOp, requestSnapshot, attach, createSession)*, *내부를 HTTP fetch → WebSocket으로 갈아엎음*, *서버→클라 수신 콜백 추가* |
| 수정 | `rhwp-studio/src/main.ts` | `SessionClient`에 onServerEvent 콜백 주입 (op 적용·workbench 패스스루) |
| 생성 | `rhwp-studio/e2e/ws-bridge.test.mjs` | Puppeteer 양방향 종단 검증 |
| 생성 | `multiple-agent-reconstruction/hwp_sub_agent_simulation_ssr.ipynb` | 새 노트북: HTTP POST `/sessions`·`/workbench` 호출 (WS와 무관) |

작업 공간 루트는 `/Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/`. UNIVA-rhwp 루트는 그 하위 `UNIVA-rhwp/`. 본 plan의 경로는 UNIVA-rhwp 루트 기준이 디폴트, 노트북은 작업 공간 루트.

**프로토콜 요약 (Section C 인터페이스와 같이 보세요):**

```
WS endpoint:  GET ws://127.0.0.1:7710/sessions/:id/ws
방향:         양방향 텍스트 프레임 (JSON). "kind" 필드가 메시지 종류 식별.
              방향은 채널 컨텍스트로 결정되므로 별도 dir 필드 불필요.

서버 → 클라 (ServerEvent enum):
  {"kind":"ops","seq":N,"ops":[EditOperation,…]}
  {"kind":"workbench","seq":N,"action":"…","payload":{…}}

클라 → 서버 (ClientMessage enum):
  {"kind":"ops","ops":[EditOperation,…]}            — 사용자 키입력 미러
  {"kind":"snapshot","file_base64":"…"}             — 전체 동기화 (snake_case)
  {"kind":"ping"}                                    — keep-alive

다른 endpoint는 HTTP 그대로:
  POST /sessions              세션 생성 (노트북·클라 모두 사용)
  POST /sessions/:id/workbench LLM 워크벤치 명령 (노트북 전용)
  GET  /sessions/:id/ir       초기 IR 로드 (클라가 WS 연결 직전 호출)
  나머지 기존 라우트 그대로
```

---

## Task 1: 서버 — `events.rs` 신설 (메시지 타입 + broadcast 채널)

**Files:**
- Create: `server/src/events.rs`
- Modify: `server/Cargo.toml`

- [ ] **Step 1-1: Cargo.toml 갱신**

Modify `server/Cargo.toml` — `[dependencies]` 섹션에 두 줄 추가, 기존 `axum = "0.7"` 줄 교체.

```toml
axum = { version = "0.7", features = ["ws"] }
tokio-stream = "0.1"
futures = "0.3"
```

- [ ] **Step 1-2: `events.rs` 작성**

Create `server/src/events.rs`:

```rust
//! WebSocket 메시지 타입 + 세션별 broadcast 채널.
//!
//! 서버는 fileId별 `tokio::sync::broadcast` 채널을 가지고,
//! 그 채널을 구독한 모든 WS 연결(같은 fileId의 다른 클라이언트 포함)에
//! `ServerEvent`를 fan-out한다. 클라→서버 메시지(`ClientMessage`)는
//! 각 WS 연결이 직접 받아 처리하므로 broadcast 채널을 거치지 않는다.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// 서버 → 클라 메시지.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ServerEvent {
    /// 서버가 자기 DocumentCore에 진짜 적용한 op들.
    Ops {
        seq: i64,
        ops: Vec<serde_json::Value>,
    },
    /// 서버가 적용하지 않은 워크벤치 명령(패스스루).
    Workbench {
        seq: i64,
        action: String,
        payload: serde_json::Value,
    },
}

/// 클라 → 서버 메시지.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ClientMessage {
    /// 사용자가 직접 편집한 op를 서버에 미러링.
    Ops { ops: Vec<serde_json::Value> },
    /// 전체 문서 스냅샷 동기화 (붙여넣기·복잡 변경 시).
    Snapshot { file_base64: String },
    /// keep-alive ping. 서버는 무시 또는 pong 응답.
    Ping,
}

/// 세션별 broadcast 채널 관리.
#[derive(Clone, Default)]
pub struct EventsHub {
    senders: Arc<Mutex<HashMap<String, broadcast::Sender<ServerEvent>>>>,
}

impl EventsHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sender_for(&self, file_id: &str) -> broadcast::Sender<ServerEvent> {
        let mut g = self.senders.lock().unwrap();
        g.entry(file_id.to_string())
            .or_insert_with(|| broadcast::channel(128).0)
            .clone()
    }

    pub fn publish(&self, file_id: &str, ev: ServerEvent) {
        let _ = self.sender_for(file_id).send(ev);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_delivers_to_subscriber() {
        let hub = EventsHub::new();
        let mut rx = hub.sender_for("F1").subscribe();
        hub.publish(
            "F1",
            ServerEvent::Ops {
                seq: 1,
                ops: vec![serde_json::json!({"op":"insert_text","text":"a"})],
            },
        );
        let msg = rx.recv().await.expect("recv 실패");
        match msg {
            ServerEvent::Ops { seq, ops } => {
                assert_eq!(seq, 1);
                assert_eq!(ops.len(), 1);
            }
            _ => panic!("Ops 변종이어야 함"),
        }
    }

    #[tokio::test]
    async fn different_file_ids_are_isolated() {
        let hub = EventsHub::new();
        let mut rx_a = hub.sender_for("A").subscribe();
        let mut rx_b = hub.sender_for("B").subscribe();
        hub.publish(
            "A",
            ServerEvent::Workbench {
                seq: 1,
                action: "x".into(),
                payload: serde_json::Value::Null,
            },
        );
        assert!(rx_a.recv().await.is_ok());
        let try_b =
            tokio::time::timeout(std::time::Duration::from_millis(100), rx_b.recv()).await;
        assert!(try_b.is_err(), "B 채널엔 메시지 없어야 함");
    }

    #[test]
    fn server_event_json_has_kind_tag() {
        let ev = ServerEvent::Ops {
            seq: 42,
            ops: vec![],
        };
        let j = serde_json::to_value(&ev).unwrap();
        assert_eq!(j["kind"], "ops");
        assert_eq!(j["seq"], 42);
    }

    #[test]
    fn client_message_deserializes_from_json() {
        let raw = r#"{"kind":"ops","ops":[{"op":"insert_text","section":0,"para":0,"offset":0,"text":"가"}]}"#;
        let parsed: ClientMessage = serde_json::from_str(raw).expect("parse 실패");
        match parsed {
            ClientMessage::Ops { ops } => assert_eq!(ops.len(), 1),
            _ => panic!("Ops여야 함"),
        }

        let raw2 = r#"{"kind":"snapshot","file_base64":"AAAA"}"#;
        let parsed2: ClientMessage = serde_json::from_str(raw2).expect("parse 실패");
        match parsed2 {
            ClientMessage::Snapshot { file_base64 } => assert_eq!(file_base64, "AAAA"),
            _ => panic!("Snapshot이어야 함"),
        }
    }
}
```

- [ ] **Step 1-3: 모듈 컴파일 확인**

```bash
cd UNIVA-rhwp/server
PATH="$HOME/.cargo/bin:$PATH" cargo check 2>&1 | tail -10
```

Expected: warning(미사용) 정도. 에러 없음.

- [ ] **Step 1-4: 단위 테스트**

```bash
cd UNIVA-rhwp/server
PATH="$HOME/.cargo/bin:$PATH" cargo test events:: 2>&1 | tail -10
```

(`events::tests::` 4개 테스트 모두 PASS여야 — Step 1-3에서는 mod 등록 전이라 안 돌 수도 있으니 Task 2 이후 재확인.)

- [ ] **Step 1-5: Commit**

```bash
cd UNIVA-rhwp
git add server/Cargo.toml server/src/events.rs
git commit -m "Task #zephy-bridge: events 모듈 — ServerEvent/ClientMessage + EventsHub broadcast

axum WS feature 활성, tokio-stream/futures 의존성 추가."
```

---

## Task 2: 서버 — `ws.rs` 신설 (WebSocket handler + 클라 메시지 dispatch)

**Files:**
- Create: `server/src/ws.rs`

- [ ] **Step 2-1: `ws.rs` 작성**

Create `server/src/ws.rs`:

```rust
//! WebSocket 핸들러 — 양방향 메시지 처리.
//!
//! 한 WS 연결의 수명 동안:
//!   - 서버는 `EventsHub`의 broadcast 채널을 subscribe하여 `ServerEvent`를 텍스트 프레임으로 전송
//!   - 클라는 `ClientMessage`를 텍스트 프레임으로 보냄; 서버는 받아 DocumentCore에 적용 후
//!     자기 broadcast 채널에 `ServerEvent::Ops`를 다시 발행(다른 구독자에게 fan-out)

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast::error::RecvError;

use crate::events::{ClientMessage, ServerEvent};
use crate::{get_or_restore, session_info, AppError, AppState, Session};

/// HTTP GET → WebSocket upgrade. 같은 URL에 axum 자동 upgrade.
pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, file_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, file_id: String) {
    // 세션 확보(없으면 sqlite 복원 or minio 폴백 — 비활성이면 에러 후 close)
    let session = match get_or_restore(&state, &file_id).await {
        Ok(s) => s,
        Err(err) => {
            tracing::warn!("ws: 세션 확보 실패 fid={} err={:?}", file_id, err.msg);
            let mut s = socket;
            let _ = s
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 4404,
                    reason: format!("session not found: {file_id}").into(),
                })))
                .await;
            return;
        }
    };

    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.events.sender_for(&file_id).subscribe();

    // 서버 → 클라 발행 루프
    let send_state = state.clone();
    let send_fid = file_id.clone();
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    let json = match serde_json::to_string(&ev) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    // dir:"server" prefix를 client·server 공통 wire 형식에 맞춰 인라인.
                    // ServerEvent는 kind만 가지므로 dir 필드를 한 번 더 wrapping하지 않고
                    // 클라에서 dir == "server" 가정.
                    if sender.send(Message::Text(json)).await.is_err() {
                        break; // 연결 닫힘
                    }
                }
                Err(RecvError::Lagged(_)) => continue, // 뒤처졌으면 다음부터
                Err(RecvError::Closed) => break,
            }
        }
        let _ = send_state; // suppress unused
        let _ = send_fid;
    });

    // 클라 → 서버 수신 루프
    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!("ws recv error: {e:?}");
                break;
            }
        };
        match msg {
            Message::Text(text) => {
                if let Err(err) = handle_client_text(&state, &file_id, &session, &text).await {
                    tracing::warn!("ws client msg err: {err}");
                    // 클라에게 알림 — 별도 ServerEvent 변종은 안 만들고, 일단 로그.
                }
            }
            Message::Close(_) => break,
            Message::Ping(p) => {
                // axum이 자동 pong 처리하지만 명시적 처리 가능
                let _ = p;
            }
            _ => {} // Binary/Pong 무시 (Sub-1에서는 텍스트만)
        }
    }

    send_task.abort();
}

async fn handle_client_text(
    state: &AppState,
    file_id: &str,
    session: &Arc<std::sync::Mutex<Session>>,
    text: &str,
) -> Result<(), String> {
    let msg: ClientMessage =
        serde_json::from_str(text).map_err(|e| format!("ClientMessage JSON 파싱 실패: {e}"))?;
    match msg {
        ClientMessage::Ops { ops } => {
            let mut s = session.lock().unwrap();
            let ops_json = serde_json::to_string(&ops)
                .map_err(|e| format!("ops 직렬화 실패: {e}"))?;
            s.core
                .apply_edit_ops_json(&ops_json)
                .map_err(|e| format!("op 적용 실패: {e}"))?;
            for op in &ops {
                let seq = s.next_seq;
                state
                    .store
                    .append_op(file_id, seq, &op.to_string())
                    .map_err(|e| format!("sqlite append_op 실패: {e}"))?;
                s.next_seq += 1;
                state.events.publish(
                    file_id,
                    ServerEvent::Ops {
                        seq,
                        ops: vec![op.clone()],
                    },
                );
            }
            drop(s);
            Ok(())
        }
        ClientMessage::Snapshot { file_base64 } => {
            let bytes = STANDARD
                .decode(file_base64.as_bytes())
                .map_err(|e| format!("base64 디코드 실패: {e}"))?;
            let core = rhwp::parse_document(&bytes)
                .map_err(|e| format!("스냅샷 파싱 실패: {e}"))?;
            let mut s = session.lock().unwrap();
            let mut new_core = rhwp::DocumentCore::new_empty();
            new_core.set_document(core);
            s.core = new_core;
            let seq = s.next_seq;
            state
                .store
                .append_snapshot(file_id, seq, &bytes)
                .map_err(|e| format!("sqlite append_snapshot 실패: {e}"))?;
            s.next_seq += 1;
            let _ = session_info(file_id, &s);
            drop(s);
            // 스냅샷 후엔 클라들이 자기 IR을 재로드해야 정합 — 일단 type:"ops"는 안 보내고
            // 추후 ServerEvent::Snapshot 변종 추가 여지를 남김(Sub-2 합본 단계).
            Ok(())
        }
        ClientMessage::Ping => Ok(()),
    }
}
```

- [ ] **Step 2-2: 모듈 컴파일 확인**

```bash
cd UNIVA-rhwp/server
PATH="$HOME/.cargo/bin:$PATH" cargo check 2>&1 | tail -15
```

Expected: `crate::{AppError, AppState, get_or_restore, session_info, Session}` import 에러 — 이건 main.rs가 *바이너리 crate*라 `crate::` 임포트가 직접 안 됨. Step 2-3·2-4에서 main.rs에 `pub` 추가로 노출.

- [ ] **Step 2-3: main.rs에 `events`·`ws` 모듈 등록 + 필요한 항목 `pub(crate)` 노출**

Modify `server/src/main.rs`:

(a) 기존 `mod storage; mod store;` 위에 두 줄 추가:

```rust
pub mod events;
pub mod ws;
```

(b) `AppState`·`AppError`·`Session`·`get_or_restore`·`session_info`의 가시성을 `pub(crate)`로 변경(`ws.rs`에서 import용). 예시:

```rust
#[derive(Clone)]
pub(crate) struct AppState { ... }

pub(crate) struct Session { ... }

pub(crate) struct AppError { ... }
impl AppError { pub(crate) fn new(...) -> Self { ... } ... }

pub(crate) async fn get_or_restore(...) -> ... { ... }
pub(crate) fn session_info(...) -> SessionInfo { ... }
```

- [ ] **Step 2-4: `AppState`에 `events` 필드 + main()에서 EventsHub 주입**

Modify `server/src/main.rs`:

```rust
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) sessions: Arc<Mutex<HashMap<String, Arc<Mutex<Session>>>>>,
    pub(crate) store: Arc<store::Store>,
    pub(crate) storage: Arc<storage::Storage>,
    pub(crate) events: events::EventsHub,
}
```

main() 안:

```rust
let state = AppState {
    sessions: Arc::new(Mutex::new(HashMap::new())),
    store: Arc::new(store),
    storage: Arc::new(storage),
    events: events::EventsHub::new(),
};
```

- [ ] **Step 2-5: 라우터에 WS 라우트 추가**

Modify `server/src/main.rs` — `router(state)`의 라우트 체인에 한 줄 추가:

```rust
.route("/sessions/:id/ws", get(ws::ws_upgrade))
```

(SSE 라우트는 추가하지 않는다 — 기존 plan과 달라진 부분.)

- [ ] **Step 2-6: 빌드 + 테스트**

```bash
cd UNIVA-rhwp/server
PATH="$HOME/.cargo/bin:$PATH" cargo build --release 2>&1 | tail -10
PATH="$HOME/.cargo/bin:$PATH" cargo test 2>&1 | tail -10
```

Expected: 빌드 OK. `events::tests::` 4개 PASS.

- [ ] **Step 2-7: 수동 검증 — WS 핸드셰이크**

별도 BG로 새 바이너리 가동 후 `wscat` 또는 curl로 핸드셰이크 확인:

```bash
# wscat이 있다면
wscat -c ws://127.0.0.1:7710/sessions/test-fid/ws
# curl로 핸드셰이크만(실제 메시지는 못 봄):
curl -sI -H "Upgrade: websocket" -H "Connection: Upgrade" \
     -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
     -H "Sec-WebSocket-Version: 13" \
     http://127.0.0.1:7710/sessions/test-fid/ws
```

Expected (curl): `HTTP/1.1 101 Switching Protocols` + Sec-WebSocket-Accept 헤더. *세션이 없으면 4404 close*가 와야 하니, 실 검증은 세션 만들고 다시 시도(Task 5).

- [ ] **Step 2-8: Commit**

```bash
cd UNIVA-rhwp
git add server/src/main.rs server/src/ws.rs
git commit -m "Task #zephy-bridge: ws 모듈 — WebSocket 핸들러 + ClientMessage dispatch

GET /sessions/:id/ws에서 양방향 WS 메시지 처리.
ClientMessage::Ops를 받아 apply_edit_ops_json + broadcast publish.
ClientMessage::Snapshot를 받아 put_snapshot 로직 인라인."
```

---

## Task 3: 서버 — `/workbench` 핸들러 (broadcast 발행)

**Files:**
- Modify: `server/src/main.rs`

- [ ] **Step 3-1: WorkbenchReq/Resp DTO 추가**

Modify `server/src/main.rs` — `IrQuery` 정의 뒤에 추가:

```rust
#[derive(Deserialize)]
struct WorkbenchReq {
    action: String,
    payload: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkbenchResp {
    seq: i64,
    /// "ops" — 서버가 자기 DocumentCore에 진짜 적용.
    /// "passthrough" — 서버는 broadcast만, 실제 적용은 클라가 함.
    applied: String,
    info: Option<SessionInfo>,
}
```

- [ ] **Step 3-2: `workbench` 핸들러 작성**

Modify `server/src/main.rs` — `delete_session` 위에 추가:

```rust
async fn workbench(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Json(req): Json<WorkbenchReq>,
) -> Result<Json<WorkbenchResp>, AppError> {
    let session = get_or_restore(&state, &file_id).await?;

    match req.action.as_str() {
        "insert_text" => {
            let section = req
                .payload
                .get("section")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| AppError::bad_request("payload.section 누락"))?;
            let para = req
                .payload
                .get("para")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| AppError::bad_request("payload.para 누락"))?;
            let offset = req
                .payload
                .get("offset")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| AppError::bad_request("payload.offset 누락"))?;
            let text = req
                .payload
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::bad_request("payload.text 누락"))?;

            let op = serde_json::json!({
                "op": "insert_text",
                "section": section,
                "para": para,
                "offset": offset,
                "text": text,
            });

            let mut s = session.lock().unwrap();
            let ops_json = format!("[{}]", op);
            s.core
                .apply_edit_ops_json(&ops_json)
                .map_err(|e| AppError::unprocessable(format!("op 적용 실패: {e}")))?;
            let seq = s.next_seq;
            state.store.append_op(&file_id, seq, &op.to_string())?;
            s.next_seq += 1;
            let info = session_info(&file_id, &s);
            drop(s);

            state.events.publish(
                &file_id,
                events::ServerEvent::Ops {
                    seq,
                    ops: vec![op],
                },
            );

            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: Some(info),
            }))
        }
        _ => {
            let mut s = session.lock().unwrap();
            let seq = s.next_seq;
            s.next_seq += 1;
            drop(s);
            state.events.publish(
                &file_id,
                events::ServerEvent::Workbench {
                    seq,
                    action: req.action.clone(),
                    payload: req.payload.clone(),
                },
            );
            Ok(Json(WorkbenchResp {
                seq,
                applied: "passthrough".to_string(),
                info: None,
            }))
        }
    }
}
```

- [ ] **Step 3-3: 라우터에 `/workbench` 라우트 추가**

Modify `server/src/main.rs` — Task 2-5의 라우트 체인 *위*에 한 줄 추가:

```rust
.route("/sessions/:id/workbench", post(workbench))
```

- [ ] **Step 3-4: 기존 `apply_ops`에도 broadcast 발행 추가**

Modify `server/src/main.rs` — `apply_ops` 안 `for op in &ops { ... }` 루프를 다음으로 교체:

```rust
for op in &ops {
    let seq = s.next_seq;
    state.store.append_op(&file_id, seq, &op.to_string())?;
    s.next_seq += 1;
    state.events.publish(
        &file_id,
        events::ServerEvent::Ops {
            seq,
            ops: vec![op.clone()],
        },
    );
}
```

- [ ] **Step 3-5: 빌드**

```bash
cd UNIVA-rhwp/server
PATH="$HOME/.cargo/bin:$PATH" cargo build --release 2>&1 | tail -10
```

Expected: warning 외 에러 없음.

- [ ] **Step 3-6: 수동 curl + wscat 검증 (서버 재기동 후)**

서버 재기동(`TaskStop` BG → 새 BG):

```bash
cd UNIVA-rhwp
B64=$(base64 -i samples/hwpx/blank_hwpx.hwpx | tr -d '\n')
# 1) 세션 생성
curl -s -X POST http://127.0.0.1:7710/sessions \
  -H 'Content-Type: application/json' \
  -d "{\"fileId\":\"wb-ws-test\",\"format\":\"hwpx\",\"fileBase64\":\"$B64\"}" | head -c 200
echo
# 2) WS 구독 (wscat이 있다면 — 없으면 Task 5 e2e로 대체)
# wscat -c ws://127.0.0.1:7710/sessions/wb-ws-test/ws  &
# WS_PID=$!
# sleep 1
# 3) workbench로 insert_text 발사
curl -s -X POST http://127.0.0.1:7710/sessions/wb-ws-test/workbench \
  -H 'Content-Type: application/json' \
  -d '{"action":"insert_text","payload":{"section":0,"para":0,"offset":0,"text":"가"}}' | head -c 200
# 4) 패스스루
curl -s -X POST http://127.0.0.1:7710/sessions/wb-ws-test/workbench \
  -H 'Content-Type: application/json' \
  -d '{"action":"replace_runs","payload":{"sec":0,"para":0,"runs":[]}}' | head -c 200
echo
# kill $WS_PID 2>/dev/null
```

Expected:
- 1) `{"fileId":"wb-ws-test","seq":0,...}`
- 3) `{"seq":1,"applied":"ops","info":{...}}`
- 4) `{"seq":2,"applied":"passthrough","info":null}`
- (wscat 사용 시) 두 텍스트 프레임이 들어옴 — `{"kind":"ops",...}`, `{"kind":"workbench",...}`

- [ ] **Step 3-7: Commit**

```bash
cd UNIVA-rhwp
git add server/src/main.rs
git commit -m "Task #zephy-bridge: /workbench endpoint + apply_ops에 broadcast 발행

POST /workbench가 insert_text는 진짜 적용+broadcast Ops 발행,
그 외 액션은 broadcast Workbench 패스스루.
기존 /ops도 broadcast 발행 한 줄 추가(클라→서버 미러링과 호환)."
```

---

## Task 4: 클라이언트 — `session-client.ts` WebSocket 갈아엎기

**Files:**
- Modify: `rhwp-studio/src/core/session-client.ts`

기존 `SessionClient` class는 *클라→서버* 미러링을 HTTP POST `/sessions/{id}/ops`·`PUT /snapshot`으로 보내고 있다. 이걸 *내부적으로 WebSocket으로 갈아엎으면서* 서버→클라 수신 콜백도 같은 class에서 처리하도록 통합한다. *외부 API(MirrorSink 인터페이스, queueOp, requestSnapshot, attach, createSession)는 유지*해 호출자는 손대지 않는다.

- [ ] **Step 4-1: 현재 `session-client.ts` 전체 읽기 + 새 구조 확인**

```bash
wc -l UNIVA-rhwp/rhwp-studio/src/core/session-client.ts
cat UNIVA-rhwp/rhwp-studio/src/core/session-client.ts
```

기존 외부 API:
- `class SessionClient implements MirrorSink`
- `constructor(opts: SessionClientOptions)`
- `createSession(bytes: Uint8Array): Promise<void>`
- `attach(): void`
- `queueOp(op: EditOperation): void`
- `requestSnapshot(): void`

이 메서드 시그니처를 *그대로* 유지. 새로 추가할 것: `onServerEvent` 옵션·핸들러 등록.

- [ ] **Step 4-2: 새 `session-client.ts` 작성 (전체 교체)**

Modify `rhwp-studio/src/core/session-client.ts` — 파일 전체를 다음으로 교체:

```typescript
/**
 * SSR 세션 클라이언트 — *양방향 WebSocket*.
 *
 * 한 WS 채널로:
 *   - *클라 → 서버* 미러링: queueOp(디바운스), requestSnapshot, attach(beforeunload flush)
 *   - *서버 → 클라* 수신: onServerEvent(콜백)에 ServerEvent를 전달
 *
 * 기존 외부 API(MirrorSink, queueOp, requestSnapshot, attach, createSession)는 유지.
 * 호출자(InputHandler 등)는 변경 0.
 */
import type { EditOperation } from '@/engine/edit-op';

export interface MirrorSink {
  queueOp(op: EditOperation): void;
  requestSnapshot(): void;
}

/** WS 텍스트 프레임 본문 — 서버 → 클라 */
export type ServerEvent =
  | { kind: 'ops'; seq: number; ops: EditOpJson[] }
  | { kind: 'workbench'; seq: number; action: string; payload: unknown };

interface EditOpJson {
  op: string;
  section?: number;
  para?: number;
  offset?: number;
  text?: string;
  count?: number;
  deleted_text?: string;
  prev_len?: number;
}

function bytesToBase64(bytes: Uint8Array): string {
  let bin = '';
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    bin += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(bin);
}

export interface SessionClientOptions {
  /** 서버 base URL. http(s)://host:port. WS URL은 ws(s)://...로 자동 변환. */
  baseUrl: string;
  fileId: string;
  getSnapshotBytes: () => Uint8Array | null;
  format?: string;
  debounceMs?: number;
  /** 서버가 발행한 이벤트를 받았을 때 콜백. main.ts에서 ops/workbench 분기 처리. */
  onServerEvent?: (ev: ServerEvent) => void;
  /** WS 재연결 백오프 — 기본 [500, 1000, 2000, 5000, 10000] ms */
  reconnectDelaysMs?: number[];
}

const DEFAULT_BACKOFF = [500, 1000, 2000, 5000, 10000];

export class SessionClient implements MirrorSink {
  private readonly baseUrlHttp: string;
  private readonly baseUrlWs: string;
  private readonly fileId: string;
  private readonly getSnapshotBytes: () => Uint8Array | null;
  private readonly format: string;
  private readonly debounceMs: number;
  private readonly onServerEvent?: (ev: ServerEvent) => void;
  private readonly reconnectDelaysMs: number[];

  private ws: WebSocket | null = null;
  private reconnectIdx = 0;
  private connected = false;
  private sendBuffer: string[] = []; // WS 닫혀 있을 때 큐
  private queue: EditOperation[] = [];
  private opTimer: ReturnType<typeof setTimeout> | null = null;
  private unloadHandler: (() => void) | null = null;

  constructor(opts: SessionClientOptions) {
    this.baseUrlHttp = opts.baseUrl.replace(/\/$/, '');
    this.baseUrlWs = this.baseUrlHttp.replace(/^http/, 'ws');
    this.fileId = opts.fileId;
    this.getSnapshotBytes = opts.getSnapshotBytes;
    this.format = opts.format ?? 'hwpx';
    this.debounceMs = opts.debounceMs ?? 600;
    this.onServerEvent = opts.onServerEvent;
    this.reconnectDelaysMs = opts.reconnectDelaysMs ?? DEFAULT_BACKOFF;
  }

  /** fileId + 원본 바이트로 서버 세션을 생성/재생성. 이후 WS 연결. */
  async createSession(bytes: Uint8Array): Promise<void> {
    const body = JSON.stringify({
      fileId: this.fileId,
      format: this.format,
      fileBase64: bytesToBase64(bytes),
    });
    const res = await fetch(this.baseUrlHttp + '/sessions', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
    if (!res.ok) throw new Error(`세션 생성 실패: HTTP ${res.status}`);
    this.openWs();
    this.installUnloadFlush();
  }

  /** 이미 서버에 존재하는 세션에 WS만 연결. createSession을 호출하면 ops 초기화. */
  attach(): void {
    this.openWs();
    this.installUnloadFlush();
  }

  queueOp(op: EditOperation): void {
    this.queue.push(op);
    this.scheduleOpFlush();
  }

  requestSnapshot(): void {
    const bytes = this.getSnapshotBytes();
    if (!bytes) return;
    const msg = JSON.stringify({
      kind: 'snapshot',
      file_base64: bytesToBase64(bytes),
    });
    this.sendOrBuffer(msg);
  }

  private scheduleOpFlush(): void {
    if (this.opTimer) clearTimeout(this.opTimer);
    this.opTimer = setTimeout(() => this.flushOps(), this.debounceMs);
  }

  private flushOps(): void {
    if (this.queue.length === 0) return;
    const ops = this.queue;
    this.queue = [];
    const msg = JSON.stringify({ kind: 'ops', ops });
    this.sendOrBuffer(msg);
  }

  private sendOrBuffer(msg: string): void {
    if (this.ws && this.connected) {
      try {
        this.ws.send(msg);
      } catch {
        this.sendBuffer.push(msg);
      }
    } else {
      this.sendBuffer.push(msg);
    }
  }

  private openWs(): void {
    if (this.ws) return;
    const url = `${this.baseUrlWs}/sessions/${encodeURIComponent(this.fileId)}/ws`;
    this.ws = new WebSocket(url);

    this.ws.addEventListener('open', () => {
      this.connected = true;
      this.reconnectIdx = 0;
      // 버퍼된 메시지 flush
      while (this.sendBuffer.length > 0) {
        const m = this.sendBuffer.shift()!;
        try {
          this.ws!.send(m);
        } catch {
          this.sendBuffer.unshift(m);
          break;
        }
      }
    });

    this.ws.addEventListener('message', (e) => {
      let parsed: ServerEvent;
      try {
        parsed = JSON.parse(e.data) as ServerEvent;
      } catch {
        console.warn('[session-client] WS 메시지 JSON 파싱 실패:', e.data);
        return;
      }
      if (this.onServerEvent) {
        try {
          this.onServerEvent(parsed);
        } catch (err) {
          console.error('[session-client] onServerEvent 예외:', err);
        }
      }
    });

    this.ws.addEventListener('close', () => {
      this.connected = false;
      this.ws = null;
      this.scheduleReconnect();
    });

    this.ws.addEventListener('error', (e) => {
      console.warn('[session-client] WS 에러:', e);
      // close 이벤트가 뒤따라 옴 → 거기서 재연결
    });
  }

  private scheduleReconnect(): void {
    const delay =
      this.reconnectDelaysMs[
        Math.min(this.reconnectIdx, this.reconnectDelaysMs.length - 1)
      ];
    this.reconnectIdx += 1;
    setTimeout(() => this.openWs(), delay);
  }

  private installUnloadFlush(): void {
    if (this.unloadHandler) return;
    this.unloadHandler = () => {
      if (this.opTimer) clearTimeout(this.opTimer);
      this.flushOps();
      // WS는 자동으로 닫힘
    };
    window.addEventListener('beforeunload', this.unloadHandler);
  }
}
```

- [ ] **Step 4-3: TypeScript 컴파일 확인**

```bash
cd UNIVA-rhwp/rhwp-studio
npx tsc --noEmit 2>&1 | tail -10
```

Expected: 에러 없음. 만약 `EditOperation` import 경로(`@/engine/edit-op`) 에러면 기존 import 그대로 유지.

- [ ] **Step 4-4: Commit**

```bash
cd UNIVA-rhwp
git add rhwp-studio/src/core/session-client.ts
git commit -m "Task #zephy-bridge: session-client.ts 내부를 WebSocket으로 갈아엎음

외부 API(MirrorSink, queueOp, requestSnapshot, attach, createSession) 그대로.
HTTP POST /ops·PUT /snapshot 대신 WS 텍스트 프레임 송수신.
onServerEvent 콜백 옵션 추가 — main.ts가 ops/workbench 처리."
```

---

## Task 5: 클라이언트 — `main.ts` entry에 `onServerEvent` 핸들러 주입

**Files:**
- Modify: `rhwp-studio/src/main.ts`

- [ ] **Step 5-1: 현재 main.ts의 SessionClient 호출 위치 확인**

```bash
grep -n "new SessionClient\|SessionClient(\|attachSsrMirror" UNIVA-rhwp/rhwp-studio/src/main.ts
```

기존 코드에서 `new SessionClient({...})` 호출 부분을 찾아 그 옵션 객체에 `onServerEvent` 콜백을 추가.

- [ ] **Step 5-2: import 추가**

Modify `rhwp-studio/src/main.ts` — 파일 상단에:

```typescript
import { getActionDef } from './hwpctl/action-registry';
```

(`SessionClient`, `ServerEvent` 타입은 이미 session-client.ts에서 import 중일 것.)

- [ ] **Step 5-3: `onServerEvent` 콜백 정의 + SessionClient 옵션에 주입**

Modify `rhwp-studio/src/main.ts` — 기존 `new SessionClient({...})` 호출 옵션 객체에 다음 항목 추가:

```typescript
onServerEvent: (ev) => {
  if (ev.kind === 'ops') {
    for (const op of ev.ops) {
      try {
        switch (op.op) {
          case 'insert_text':
            wasm.getDoc().insertText(op.section!, op.para!, op.offset!, op.text!);
            break;
          case 'split_paragraph':
            wasm.getDoc().splitParagraph(op.section!, op.para!, op.offset!);
            break;
          default:
            console.warn(`[main] Sub-1 미지원 ops op: ${op.op}`);
        }
      } catch (e) {
        console.error('[main] WASM op 적용 실패:', op, e);
      }
    }
  } else if (ev.kind === 'workbench') {
    const def = getActionDef(ev.action);
    if (!def?.executor) {
      console.warn(`[main] 알 수 없는 hwpctl action: ${ev.action}`);
      return;
    }
    try {
      def.executor(ev.payload as never);
    } catch (e) {
      console.error(`[main] hwpctl executor 예외: ${ev.action}`, e);
    }
  }
},
```

(`wasm`은 기존 main.ts에서 WASM 인스턴스 변수 이름 — Step 5-1에서 확인한 정확한 이름으로 보정.)

- [ ] **Step 5-4: 빌드**

```bash
cd UNIVA-rhwp/rhwp-studio
npm run build 2>&1 | tail -15
```

Expected: built OK.

- [ ] **Step 5-5: 서버 재기동(새 dist 반영) — 기존 BG task `TaskStop` 후 새 BG로 시작**

(Claude 운영 노트: 새 dist이지만 STUDIO_DIR 경로는 동일이라 ENV 재지정만으로 됨.)

- [ ] **Step 5-6: 브라우저 진입 확인**

브라우저 DevTools → Network → WS 탭에 `ws://127.0.0.1:7710/sessions/<fid>/ws` 연결이 보이고 Frames 패널에 메시지가 흐름.

- [ ] **Step 5-7: Commit**

```bash
cd UNIVA-rhwp
git add rhwp-studio/src/main.ts
git commit -m "Task #zephy-bridge: main.ts에 SessionClient onServerEvent 콜백 주입

WS로 받은 ServerEvent를 ops(WASM 직접) / workbench(hwpctl) 로 분기 처리."
```

---

## Task 6: E2E — Puppeteer로 양방향 검증

**Files:**
- Create: `rhwp-studio/e2e/ws-bridge.test.mjs`

- [ ] **Step 6-1: 테스트 작성**

Create `rhwp-studio/e2e/ws-bridge.test.mjs`:

```javascript
/**
 * E2E: WS 양방향 — 서버→클라 push와 클라→서버 미러링 둘 다 검증.
 *
 * 시나리오:
 *   1) 빈 hwpx로 POST /sessions → fileId
 *   2) Puppeteer로 ?fileId 진입 → WS 연결됨
 *   3) curl로 POST /workbench (insert_text "FROM-LLM") → 서버 push → DOM에 "FROM-LLM"
 *   4) page.evaluate로 InputHandler 시뮬 — 직접 키 입력은 어려우니
 *      *WS 직접 호출* 방식: page에서 new WebSocket(...) 만들어 ClientMessage::Ops 발사,
 *      그 결과가 sqlite에 반영되었는지 GET /ir로 확인
 */

import puppeteer from 'puppeteer-core';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const SERVER = 'http://127.0.0.1:7710';
const WS_BASE = 'ws://127.0.0.1:7710';
const BLANK_HWPX = resolve(import.meta.dirname ?? '.', '..', '..', 'samples', 'hwpx', 'blank_hwpx.hwpx');
const CHROMIUM = process.env.CHROMIUM ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';

async function http(method, path, body) {
  const r = await fetch(`${SERVER}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await r.text();
  if (!r.ok) throw new Error(`HTTP ${r.status} ${path}: ${text}`);
  return text ? JSON.parse(text) : {};
}

async function main() {
  // 1) 세션
  const fileId = `e2e-ws-${Date.now()}`;
  const bytes = readFileSync(BLANK_HWPX);
  await http('POST', '/sessions', {
    fileId,
    format: 'hwpx',
    fileBase64: Buffer.from(bytes).toString('base64'),
  });
  console.log('세션:', fileId);

  // 2) 브라우저
  const browser = await puppeteer.launch({
    executablePath: CHROMIUM,
    headless: 'new',
    args: ['--no-sandbox'],
  });
  const page = await browser.newPage();
  page.on('console', (msg) => console.log('[browser]', msg.text()));
  await page.goto(`${SERVER}/?fileId=${fileId}`, { waitUntil: 'networkidle0', timeout: 30000 });

  // WS 연결 대기 — 콘솔에 "WS open" 로그가 없으면 잠시 대기
  await new Promise((r) => setTimeout(r, 1000));

  // 3) 서버→클라: workbench로 발사
  await http('POST', `/sessions/${fileId}/workbench`, {
    action: 'insert_text',
    payload: { section: 0, para: 0, offset: 0, text: 'FROM-LLM' },
  });

  let appeared = false;
  for (let i = 0; i < 50; i++) {
    if (await page.evaluate(() => document.body.innerText.includes('FROM-LLM'))) {
      appeared = true;
      break;
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  if (!appeared) {
    await browser.close();
    throw new Error('서버→클라 push가 5초 안에 DOM에 반영 안 됨');
  }
  console.log('OK 1: 서버→클라 push로 "FROM-LLM" 반영');

  // 4) 클라→서버: page 안에서 WS로 ops 발사
  await page.evaluate(
    async (url) => {
      const ws = new WebSocket(url);
      await new Promise((resolve, reject) => {
        ws.addEventListener('open', () => resolve());
        ws.addEventListener('error', (e) => reject(e));
        setTimeout(() => reject(new Error('WS open timeout')), 5000);
      });
      ws.send(
        JSON.stringify({
          kind: 'ops',
          ops: [
            {
              op: 'insert_text',
              section: 0,
              para: 0,
              offset: 0,
              text: 'FROM-CLIENT',
            },
          ],
        })
      );
      await new Promise((r) => setTimeout(r, 500));
      ws.close();
    },
    `${WS_BASE}/sessions/${fileId}/ws`
  );

  // 5) 서버 IR 확인 — FROM-CLIENT가 sqlite에 영속됐는지
  const ir = await http('GET', `/sessions/${fileId}/ir?page=0`);
  const irText = JSON.stringify(ir);
  if (!irText.includes('FROM-CLIENT')) {
    await browser.close();
    throw new Error(`클라→서버 ops가 서버 IR에 반영 안 됨. IR=${irText.slice(0, 500)}`);
  }
  console.log('OK 2: 클라→서버 ops로 "FROM-CLIENT"가 서버 IR에 영속');

  await browser.close();
  console.log('\n=== 양방향 WS bridge 검증 통과 ===');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
```

- [ ] **Step 6-2: 테스트 실행**

```bash
cd UNIVA-rhwp/rhwp-studio
node e2e/ws-bridge.test.mjs 2>&1 | tail -30
```

Expected: 마지막에 `=== 양방향 WS bridge 검증 통과 ===`. 실패 시 Step 6-1의 시나리오 어느 단계에서 어긋났는지 stderr가 알려줌.

- [ ] **Step 6-3: Commit**

```bash
cd UNIVA-rhwp
git add rhwp-studio/e2e/ws-bridge.test.mjs
git commit -m "Task #zephy-bridge: e2e — WS 양방향 검증

서버→클라(workbench → SSE 대체 WS push)와 클라→서버(WS ops) 둘 다 검증."
```

---

## Task 7: 새 노트북 — `hwp_sub_agent_simulation_ssr.ipynb`

**Files:**
- Create: `multiple-agent-reconstruction/hwp_sub_agent_simulation_ssr.ipynb`

노트북은 *HTTP POST `/sessions`·`/workbench`*만 사용. WebSocket과 무관하다 — LLM↔노트북↔서버 통신은 요청-응답 패턴이라 HTTP가 자연스럽고, 서버→클라 push는 *브라우저 측에서* 일어난다.

- [ ] **Step 7-1: 빈 ipynb 생성**

```bash
cd /Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction
cat > hwp_sub_agent_simulation_ssr.ipynb <<'EOF'
{"cells":[],"metadata":{"kernelspec":{"display_name":"Python 3","language":"python","name":"python3"}},"nbformat":4,"nbformat_minor":5}
EOF
```

- [ ] **Step 7-2: cell 1 (markdown) — 제목**

```markdown
# HWP Sub-agent SSR Simulation (WebSocket 통신)

`hwp_sub_agent_simulation.ipynb`의 SSR 버전. bridge(8765) 대신 *UNIVA-rhwp의 rhwp-server*가 메시지 경로.
브라우저 측 통신은 WebSocket(양방향)이지만, 본 노트북에서 LLM이 보내는 명령은
HTTP POST `/sessions/{fid}/workbench`로 가고 서버가 WS broadcast로 브라우저에 전달.

동작:
1. cell 2: 빈 hwpx로 POST /sessions → fileId
2. URL을 *수동*으로 브라우저에서 열기 (브라우저가 WS 연결)
3. LLM tool 호출 `Bash("hwp-doc-patch insert_text ...")`을 노트북이 가로채 POST /workbench
4. 서버가 진짜 적용(insert_text) 또는 패스스루 발행 → WS 텍스트 프레임 → 브라우저 자동 갱신
```

- [ ] **Step 7-3: cell 2 (code) — 환경 + 세션 생성**

```python
import base64, json, re, time, uuid
from pathlib import Path
import requests
from openai import AsyncOpenAI
from httpx import Timeout

SSR_BASE = 'http://127.0.0.1:7710'
VLLM_URL = 'https://serve-dev.rest.univa-internal.com/vllm-llm/v1/'
VLLM_MODEL = 'qwen3.5'
DEFAULT_TIMEOUT = 30.0
WORKSPACE_ROOT = Path('/Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction')
BLANK_HWPX = WORKSPACE_ROOT / 'UNIVA-rhwp' / 'samples' / 'hwpx' / 'blank_hwpx.hwpx'

def create_session(file_id: str, file_path: Path, fmt: str = 'hwpx') -> dict:
    b = file_path.read_bytes()
    r = requests.post(
        f'{SSR_BASE}/sessions',
        json={'fileId': file_id, 'format': fmt, 'fileBase64': base64.b64encode(b).decode('ascii')},
        timeout=60,
    )
    r.raise_for_status()
    return r.json()

SESSION_FILE_ID = f'sim-{int(time.time())}'
info = create_session(SESSION_FILE_ID, BLANK_HWPX)
print('=== 세션 생성 ===')
print(json.dumps(info, ensure_ascii=False, indent=2))
print()
print('아래 URL을 브라우저에서 여세요(브라우저가 WS 연결):')
print(f'  {SSR_BASE}/?fileId={SESSION_FILE_ID}')
```

- [ ] **Step 7-4: cell 3 (code) — LLM 인프라 (기존 노트북 cell 6 그대로 복사)**

기존 `hwp_sub_agent_simulation.ipynb`의 cell 6 *전체*를 복사. 변경 없음.

- [ ] **Step 7-5: cell 4 (code) — Bash 실행기 SSR 라우터**

```python
import os, shlex as _shlex, subprocess, argparse

HWP_DOC_PATCH_PREFIX = 'hwp-doc-patch '

def parse_hwp_doc_patch_call(cmd: str) -> tuple[str, str | None, dict]:
    tokens = _shlex.split(cmd)
    if not tokens or tokens[0] != 'hwp-doc-patch':
        raise ValueError('hwp-doc-patch 명령이 아님')
    p = argparse.ArgumentParser(prog='hwp-doc-patch', add_help=False)
    p.add_argument('action')
    p.add_argument('--file-id', default=None)
    p.add_argument('--chatroom-id', default=None)
    p.add_argument('--user-id', default=None)
    p.add_argument('--payload', default='{}')
    ns, _ = p.parse_known_args(tokens[1:])
    return ns.action, ns.file_id, json.loads(ns.payload)

def format_as_sentinel_json(body: dict) -> str:
    return f'<<<HWP_DOC_PATCH_JSON_BEGIN>>>{json.dumps(body, ensure_ascii=False)}<<<HWP_DOC_PATCH_JSON_END>>>'

BASH_TIMEOUT_SEC = 60.0

def run_bash_command(command: str) -> dict:
    cmd = command.strip()
    if cmd.startswith(HWP_DOC_PATCH_PREFIX) or cmd == 'hwp-doc-patch':
        try:
            action, fid_arg, payload = parse_hwp_doc_patch_call(cmd)
        except Exception as e:
            return {'exit_code': 1, 'stdout': '', 'stderr': f'(명령 파싱 실패: {e})', 'truncated': False}
        fid = fid_arg or SESSION_FILE_ID
        try:
            r = requests.post(
                f'{SSR_BASE}/sessions/{fid}/workbench',
                json={'action': action, 'payload': payload},
                timeout=DEFAULT_TIMEOUT,
            )
            try:
                body = r.json()
            except json.JSONDecodeError:
                body = {'raw': r.text[:300]}
        except requests.RequestException as e:
            return {'exit_code': 1, 'stdout': '', 'stderr': f'(HTTP 호출 실패: {e})', 'truncated': False}
        if r.ok:
            return {'exit_code': 0, 'stdout': format_as_sentinel_json({'ok': True, 'result': body}), 'stderr': '', 'truncated': False}
        return {'exit_code': 1, 'stdout': format_as_sentinel_json({'ok': False, 'error': body}), 'stderr': json.dumps(body, ensure_ascii=False), 'truncated': False}
    try:
        proc = subprocess.run(['bash', '-lc', cmd], capture_output=True, text=True, timeout=BASH_TIMEOUT_SEC)
        return {'exit_code': proc.returncode, 'stdout': proc.stdout or '', 'stderr': proc.stderr or '', 'truncated': False}
    except subprocess.TimeoutExpired:
        return {'exit_code': -1, 'stdout': '', 'stderr': f'(timeout {BASH_TIMEOUT_SEC}s)', 'truncated': False}

def format_bash_result_for_tool(res: dict) -> str:
    parts = [f'exit_code: {res["exit_code"]}']
    if res['stdout']: parts.append(f'--- stdout ---\n{res["stdout"]}')
    if res['stderr']: parts.append(f'--- stderr ---\n{res["stderr"]}')
    if not res['stdout'] and not res['stderr']: parts.append('(출력 없음)')
    return '\n\n'.join(parts)

print('Bash 실행기 (SSR 라우터) 준비 완료')
```

- [ ] **Step 7-6: cell 5 (code) — self-test**

```python
_a, _f, _p = parse_hwp_doc_patch_call(
    "hwp-doc-patch insert_text --file-id F1 --payload '{\"section\":0,\"para\":0,\"offset\":0,\"text\":\"가\"}'"
)
assert _a == 'insert_text' and _f == 'F1' and _p == {'section':0,'para':0,'offset':0,'text':'가'}
_res = run_bash_command(
    f"hwp-doc-patch insert_text --file-id {SESSION_FILE_ID} "
    "--payload '{\"section\":0,\"para\":0,\"offset\":0,\"text\":\"self-test\"}'"
)
assert _res['exit_code'] == 0 and '<<<HWP_DOC_PATCH_JSON_BEGIN>>>' in _res['stdout']
print('self-test OK — 브라우저 화면에 "self-test"가 보여야 합니다')
```

- [ ] **Step 7-7: cell 6 (code) — sub_agent_run (기존 노트북 cell 10 그대로 복사)**

기존 `hwp_sub_agent_simulation.ipynb`의 cell 10 *전체*를 복사. 변경 없음.

- [ ] **Step 7-8: cell 7 (code) — 사용 예시**

```python
result = await sub_agent_run(
    "현재 빈 문서의 첫 문단 시작에 '안녕하세요'를 삽입해줘. 그 다음 finish.",
    verbose=True, max_rounds=8, file_id=SESSION_FILE_ID,
)
print(f"\nrounds={result['rounds']} clean={result['finished_clean']} summary={result['finish_summary']}")
```

- [ ] **Step 7-9: 노트북 cell 2-5 실행 + self-test 통과 확인**

Jupyter에서 cell 2 → 3 → 4 → 5 순서. cell 5에서 `self-test OK` 출력. 브라우저에 *self-test* 문자열 즉시 등장.

(`hwp_sub_agent_simulation_ssr.ipynb`는 작업 공간 루트에 있고 git 추적되지 않으면 commit 생략.)

---

## Task 8: 종단 시연 + 회귀 확인

- [ ] **Step 8-1: 서버·dist 최신 확인**

```bash
ls -la UNIVA-rhwp/server/target/release/rhwp-server UNIVA-rhwp/rhwp-studio/dist/index.html
```

- [ ] **Step 8-2: 서버 재기동(Claude는 TaskStop 후 새 BG)**

```bash
cd UNIVA-rhwp
RHWP_SERVER_ADDR=127.0.0.1:7710 \
RHWP_STUDIO_DIR=$(pwd)/rhwp-studio/dist \
RHWP_SERVER_DB=/tmp/rhwp-ssr.db \
RUST_LOG=rhwp_server=info,tower_http=info \
./server/target/release/rhwp-server
```

- [ ] **Step 8-3: 종단 시연 — 새 노트북 cell 2-7 순서 실행**

기대:
1. cell 2 출력: 세션 생성 + URL.
2. 브라우저 진입 — DevTools Network → WS 탭에 연결 보임.
3. cell 5: `self-test OK` + 브라우저 화면에 *self-test* 즉시 등장.
4. cell 7: LLM이 `insert_text` 호출 → 브라우저 화면에 *'안녕하세요'* 즉시 등장.

- [ ] **Step 8-4: 클라→서버 미러링 검증 — 브라우저에서 직접 편집**

브라우저에서 사용자가 텍스트 입력 (또는 페이지에 노출된 입력 UI 이용) → InputHandler가 `sessionClient.queueOp(op)` 호출 → WS 메시지 발행 → 서버 sqlite에 기록 → 다른 탭 열어 같은 fileId 들어가면 즉시 보임.

빠른 확인:

```bash
# 한 탭에서 편집 후, 다른 터미널에서
curl -s "http://127.0.0.1:7710/sessions/<fid>/ir?page=0" | head -c 500
```

→ 사용자가 직접 친 텍스트가 IR에 보임.

- [ ] **Step 8-5: 회귀 확인**

- 기존 `hwp_sub_agent_simulation.ipynb` 그대로 동작 (변경 없음).
- 기존 서버 endpoint(`/ops`, `/ir`, `/export`, `/save`) 응답 형식 그대로.
- 빠른 sanity:

```bash
curl -s -X POST http://127.0.0.1:7710/sessions/<fid>/ops \
  -H 'Content-Type: application/json' \
  -d '[{"op":"insert_text","section":0,"para":0,"offset":0,"text":"X"}]' | head -c 200
```

→ 200 OK. 브라우저 화면에 X도 등장(broadcast 발행됨).

- [ ] **Step 8-6: 완료 보고서**

CLAUDE.md 규칙대로 [working/](../working/) 폴더에 `task_m200_zephy_bridge_stage1.md`로 단계별 보고. 본 plan의 8 task에 대해 통과/실패를 한 줄씩.

- [ ] **Step 8-7: 최종 결과 보고서 + PR**

[report/](../report/) 폴더에 `task_m200_zephy_bridge_report.md` — DoD 통과·다음 단계(Sub-2).

---

## Self-Review (작성자 — Sub-1 spec 대비 점검)

| Spec 요구사항 | 어느 task에서 구현 |
|---|---|
| §4 컴포넌트 1 — workbench 핸들러 | Task 3 |
| §4 컴포넌트 2 — events broadcaster + WS handler (이전 SSE에서 변경) | Task 1·2 |
| §4 컴포넌트 3 — session-client.ts (이전 ssr-listener.ts 신설에서 변경) | Task 4·5 |
| §4 컴포넌트 4 — 새 노트북 | Task 7 |
| §5 데이터 흐름의 모든 화살표 | Task 1-7 통합 |
| §6.1 POST /workbench 형식 | Task 3-1·3-2 |
| §6.2 WebSocket endpoint (이전 SSE에서 변경) + 양방향 메시지 스키마 | Task 1·2 (events.rs ServerEvent·ClientMessage, ws.rs handler) |
| §6.3 노트북 Bash 라우팅 | Task 7-5 |
| §7 에러 처리 | Task 2 (4404 close), Task 4 (재연결 백오프), Task 3 (4xx 응답) |
| §8 수동 시연 | Task 8-3·8-4 |
| §8 자동화 — events 단위 테스트 | Task 1-4 |
| §8 자동화 — e2e 양방향 | Task 6 |
| §8 자동화 — 노트북 self-test | Task 7-6 |
| §8 회귀 방지 | Task 8-5 |
| §9 DoD | Task 8 전체 |

**Placeholder 점검:** 모든 step에 실코드 또는 실명령. Step 5-1·5-3의 "정확한 변수 이름 보정"은 *코드 인스펙션이 step 본 내용* — 보정 방법까지 명시. 허용.

**타입 일관성 점검:**
- `ServerEvent`(Rust) ↔ `ServerEvent`(TS) — 같은 `kind` 태그·소문자, 같은 필드 키. OK.
- `ClientMessage`(Rust) ↔ TS 측 송신 JSON — `kind:"ops"|"snapshot"|"ping"`, `file_base64` snake_case. Task 4-2의 TS 코드와 Task 1-2의 Rust enum 일치. OK.
- `applied`: `"ops"`|`"passthrough"` 두 가지로 server·client 모두 일치. OK.
- WS URL — `ws://...`로 자동 변환 (`baseUrl.replace(/^http/, 'ws')`). HTTPS 환경에서는 자동으로 wss. OK.

**Scope 확인:** 본 plan은 Sub-1만. Sub-2(나머지 11+1 액션 진짜 적용)는 별도 plan.

---

## Execution Handoff

본 plan을 task별로 실행할 때 두 옵션:

**1. Subagent-Driven (권장)** — Claude가 task별로 새 subagent를 dispatch하고 task 사이에서 검토. 각 task의 context가 격리. `superpowers:subagent-driven-development` skill 사용.

**2. Inline Execution** — 같은 세션 안에서 task들을 순차 실행하며 체크포인트에서 검토. `superpowers:executing-plans` skill 사용.

작업지시자가 결정해 주세요. 결정 전 *spec과 시각화 HTML*도 같은 방향(WebSocket·양방향)으로 갱신할 예정입니다.
