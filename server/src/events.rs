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
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerEvent {
    /// 서버가 자기 DocumentCore에 진짜 적용한 op들.
    ///
    /// `origin_client_id` — *그 op 를 처음 보낸 브라우저의 식별자*. WS broadcast 가
    /// *발신자 자신에게도 echo* 되는 구조에서, 발신자는 이 필드가 *자기 client_id 와 같으면*
    /// 이미 로컬에 적용했다고 보고 skip 한다. HTTP `/workbench` 같은 외부 호출은
    /// 발신자 식별 없이 `None` → 모든 클라가 적용 대상.
    Ops {
        seq: i64,
        ops: Vec<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin_client_id: Option<String>,
    },
    /// 서버가 적용하지 않은 워크벤치 명령(패스스루).
    Workbench {
        seq: i64,
        action: String,
        payload: serde_json::Value,
    },
    /// Sub-2: 워크벤치 종료. 다른 탭에 알림.
    Complete { seq: i64 },
    /// Sub-2: undo 등으로 서버가 전체 스냅샷 복원. 클라는 wasm 통째 교체.
    SnapshotRestored { seq: i64, snapshot_base64: String },
}

/// 클라 → 서버 메시지.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientMessage {
    /// 사용자가 직접 편집한 op를 서버에 미러링.
    ///
    /// `client_id` — *이 브라우저 인스턴스의 식별자* (SessionClient 생성 시점에
    /// `crypto.randomUUID()` 로 발급). 서버는 이 값을 그대로 broadcast 의
    /// `ServerEvent::Ops.origin_client_id` 에 실어 — 발신자가 self echo 를
    /// 식별·skip 할 수 있게 한다. 구 클라(필드 누락) 호환: `None` 으로 처리.
    Ops {
        ops: Vec<serde_json::Value>,
        #[serde(default)]
        client_id: Option<String>,
    },
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
                origin_client_id: None,
            },
        );
        let msg = rx.recv().await.expect("recv 실패");
        match msg {
            ServerEvent::Ops { seq, ops, origin_client_id } => {
                assert_eq!(seq, 1);
                assert_eq!(ops.len(), 1);
                assert!(origin_client_id.is_none());
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
            origin_client_id: None,
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
            ClientMessage::Ops { ops, client_id } => {
                assert_eq!(ops.len(), 1);
                assert!(client_id.is_none(), "client_id 누락 시 None");
            }
            _ => panic!("Ops여야 함"),
        }

        let raw2 = r#"{"kind":"snapshot","file_base64":"AAAA"}"#;
        let parsed2: ClientMessage = serde_json::from_str(raw2).expect("parse 실패");
        match parsed2 {
            ClientMessage::Snapshot { file_base64 } => assert_eq!(file_base64, "AAAA"),
            _ => panic!("Snapshot이어야 함"),
        }
    }

    #[test]
    fn server_event_complete_serializes_with_snake_case() {
        let ev = ServerEvent::Complete { seq: 42 };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains(r#""kind":"complete""#));
        assert!(json.contains(r#""seq":42"#));
    }

    #[test]
    fn server_event_snapshot_restored_serializes_with_snake_case() {
        let ev = ServerEvent::SnapshotRestored {
            seq: 7,
            snapshot_base64: "AAAA".to_string(),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains(r#""kind":"snapshot_restored""#));
    }

    #[test]
    fn server_event_ops_still_lowercase_compat() {
        let ev = ServerEvent::Ops {
            seq: 1,
            ops: vec![],
            origin_client_id: None,
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains(r#""kind":"ops""#));
    }

    // ── [Sub-6] origin_client_id 라벨 전파 단위 테스트 ──

    /// `ClientMessage::Ops` 에 `client_id` 가 있으면 그대로 파싱.
    #[test]
    fn client_message_ops_parses_client_id() {
        let raw = r#"{"kind":"ops","client_id":"cli-A","ops":[{"op":"insert_text","section":0,"para":0,"offset":0,"text":"x"}]}"#;
        let parsed: ClientMessage = serde_json::from_str(raw).expect("parse 실패");
        match parsed {
            ClientMessage::Ops { ops, client_id } => {
                assert_eq!(ops.len(), 1);
                assert_eq!(client_id.as_deref(), Some("cli-A"));
            }
            _ => panic!("Ops여야 함"),
        }
    }

    /// `ServerEvent::Ops` 가 *origin 이 Some* 일 때 JSON 에 필드가 박힌다.
    #[test]
    fn server_event_ops_serializes_origin_client_id_when_present() {
        let ev = ServerEvent::Ops {
            seq: 7,
            ops: vec![],
            origin_client_id: Some("cli-A".into()),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(
            json.contains(r#""origin_client_id":"cli-A""#),
            "origin_client_id 필드가 직렬화 결과에 있어야 함: {json}"
        );
    }

    /// `ServerEvent::Ops` 가 *origin 이 None* 이면 `skip_serializing_if` 로 키 자체가 누락.
    /// (구 클라/구 e2e 가 origin_client_id 키 없는 JSON 만 보던 호환성 회귀 방지)
    #[test]
    fn server_event_ops_skips_origin_client_id_when_none() {
        let ev = ServerEvent::Ops {
            seq: 7,
            ops: vec![],
            origin_client_id: None,
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(
            !json.contains("origin_client_id"),
            "origin_client_id 가 None 이면 직렬화 결과에 키 자체가 없어야 함: {json}"
        );
    }

    /// broadcast 채널이 *origin_client_id 라벨을 그대로 전파*하는지 — Sub-6 의 핵심 통로.
    #[tokio::test]
    async fn publish_carries_origin_client_id_through_broadcast() {
        let hub = EventsHub::new();
        let mut rx = hub.sender_for("F-origin").subscribe();
        hub.publish(
            "F-origin",
            ServerEvent::Ops {
                seq: 1,
                ops: vec![serde_json::json!({"op":"insert_text","text":"z"})],
                origin_client_id: Some("cli-A".into()),
            },
        );
        let msg = rx.recv().await.expect("recv 실패");
        match msg {
            ServerEvent::Ops { origin_client_id, .. } => {
                assert_eq!(origin_client_id.as_deref(), Some("cli-A"));
            }
            _ => panic!("Ops 변종이어야 함"),
        }
    }
}
