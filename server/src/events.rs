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
