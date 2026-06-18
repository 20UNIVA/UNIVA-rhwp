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

use crate::events::ClientMessage;
use crate::{apply_op_with_stash, get_or_restore, session_info, AppState, Session};

/// HTTP GET → WebSocket upgrade. 같은 URL에 axum 자동 upgrade.
pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, file_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, file_id: String) {
    // WebSocket 은 브라우저가 임의 헤더를 박지 못해 user 자리에 *환경변수 폴백* 만 박는다.
    // 세션이 메모리/sqlite 에 이미 있으면 user 는 무의미 (cache hit). storage download 까지
    // 가야 하는 자리에서만 의미가 있고, 그 자리에서 폴백이 비면 vfinder 가 거절한다.
    // 향후 쿼리·세션 토큰 결합 시 정착 (`docs/13-rhwp-vfinder-storage-integration.md` §10).
    let ws_user = std::env::var("RHWP_DEFAULT_USER").unwrap_or_default();

    // 세션 확보(없으면 sqlite 복원 or 외부 저장소 폴백 — 비활성이면 에러 후 close)
    let session = match get_or_restore(&state, &file_id, &ws_user).await {
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
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    let json = match serde_json::to_string(&ev) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    if sender.send(Message::Text(json)).await.is_err() {
                        break; // 연결 닫힘
                    }
                }
                Err(RecvError::Lagged(_)) => continue, // 뒤처졌으면 다음부터
                Err(RecvError::Closed) => break,
            }
        }
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
        ClientMessage::Ops { ops, client_id } => {
            // [4-2 fix] 각 op 를 EditOperation 으로 파싱 후 apply_op_with_stash 호출.
            // 모든 ops 가 op_stash 영속 + broadcast 통일 — 사용자 키 입력도 undo 대상.
            // [Sub-6] client_id 를 origin 으로 그대로 흘려보낸다 — broadcast 자신에게
            // echo 되는 메시지를 발신자가 식별·skip 할 수 있도록 라벨링.
            use rhwp::document_core::EditOperation;
            for op_value in ops {
                let op: EditOperation = serde_json::from_value(op_value)
                    .map_err(|e| format!("EditOperation 파싱 실패: {e}"))?;
                apply_op_with_stash(state, file_id, session.clone(), op, client_id.clone())
                    .await
                    .map_err(|e| format!("apply_op_with_stash: {}", e.msg))?;
            }
            Ok(())
        }
        ClientMessage::Snapshot { file_base64 } => {
            let bytes = STANDARD
                .decode(file_base64.as_bytes())
                .map_err(|e| format!("base64 디코드 실패: {e}"))?;
            // [Plan A.1] DocumentCore::from_bytes 사용 — build_core 와 동일 이유로
            // reflow_zero_height_paragraphs 등 후처리 통일.
            let new_core = rhwp::DocumentCore::from_bytes(&bytes)
                .map_err(|e| format!("스냅샷 파싱 실패: {e}"))?;
            let mut s = session.lock().unwrap();
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
