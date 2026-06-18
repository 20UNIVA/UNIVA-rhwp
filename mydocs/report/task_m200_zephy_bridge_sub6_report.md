# Sub-6 — WS broadcast self-echo 차단 (origin_client_id 라벨링)

## 배경

브라우저에서 텍스트를 빠르게 치면 *600ms 디바운스 단위마다 입력 묶음이 한 번 더 박히는* 현상 — 예: `abcdefg` → `abcdefgabcdefg`. 원인은 서버 broadcast 채널이 *원본 발신자에게도 echo* 하는 구조였다.

1. 키 입력 → 브라우저 `wasm.insertText(...)` *즉시 로컬 적용*
2. `sessionClient.queueOp(...)` → 600ms 디바운스 후 WS 전송
3. 서버 `apply_op_with_stash` → `events.publish(ServerEvent::Ops {...})`
4. *같은 WS 의 send_task* 가 fileId broadcast 구독 중 — `tokio::sync::broadcast` self-subscribe 시 자기 발행도 받음
5. 클라 `onServerEvent` 가 받음 → `wasm.insertText(...)` *두 번째 적용* = 복제

`tokio::sync::broadcast` 는 connection 식별을 모르므로, *발신 식별자를 페이로드에 실어* 클라가 자기 메시지를 알아보고 skip 하는 방식으로 해결.

## 변경

### 1. server/src/events.rs — Protocol

`ServerEvent::Ops` 와 `ClientMessage::Ops` 두 변종에 식별자 필드 한 줄씩 추가:

```rust
pub enum ServerEvent {
    Ops {
        seq: i64,
        ops: Vec<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin_client_id: Option<String>,
    },
    // ...
}

pub enum ClientMessage {
    Ops {
        ops: Vec<serde_json::Value>,
        #[serde(default)]
        client_id: Option<String>,
    },
    // ...
}
```

- `skip_serializing_if = "Option::is_none"` — None 일 때 JSON 키 자체 누락 → 구 클라/구 e2e 가 보던 응답 형태 유지 (회귀 면역).
- `#[serde(default)]` — 필드 누락된 메시지도 그대로 파싱 (구 클라 호환).

### 2. server/src/main.rs — apply_op_with_stash 시그니처 + 13 호출부

```rust
pub(crate) async fn apply_op_with_stash(
    state: &AppState,
    file_id: &str,
    session: Arc<Mutex<Session>>,
    op: rhwp::document_core::EditOperation,
    origin_client_id: Option<String>,   // 신규
) -> Result<(i64, Option<ir_compact::PatchDiff>), AppError> {
    // ...
    state.events.publish(
        file_id,
        events::ServerEvent::Ops {
            seq,
            ops: vec![op_json],
            origin_client_id: origin_client_id.clone(),
        },
    );
    // ...
}
```

12 HTTP 호출부 (`/workbench` 의 각 액션 분기) — 외부 호출이라 self-echo 사고 없음 → `None`:

```rust
let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
```

main.rs L364 부근에 *apply_op_with_stash 외 별도 publish 호출 1건* 추가 발견 (HTTP 우회 경로) — 동일하게 `origin_client_id: None` 처리.

### 3. server/src/ws.rs — client_id 통과

```rust
ClientMessage::Ops { ops, client_id } => {
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
```

### 4. rhwp-studio/src/core/session-client.ts — clientId 발급

```ts
export class SessionClient implements MirrorSink {
    // [Sub-6] 이 인스턴스의 고유 식별자. 서버가 broadcast 의 origin_client_id 에 그대로 실음.
    private readonly clientId: string = crypto.randomUUID();

    getClientId(): string {
        return this.clientId;
    }

    flushOps(): Promise<void> {
        // ...
        const msg = JSON.stringify({ kind: 'ops', client_id: this.clientId, ops });
        this.sendOrBuffer(msg);
        // ...
    }
}
```

`ServerEvent` 타입 정의에도 `origin_client_id?: string` 추가.

### 5. rhwp-studio/src/main.ts — self-echo skip 가드

`onServerEvent` 진입부 shape 가드 *뒤*, `ev.kind === 'ops'` 분기 *앞* 에:

```ts
// [Sub-6] WS broadcast self-echo skip — 자기 clientId 가 발행한 ops 는
// 이미 로컬 wasm 에 적용됨. 다시 적용하면 600ms 디바운스 단위마다 *복제* 발생.
if (ev.kind === 'ops'
    && typeof (ev as { origin_client_id?: string }).origin_client_id === 'string'
    && (ev as { origin_client_id?: string }).origin_client_id === sessionClient?.getClientId()) {
    return;
}
```

`origin_client_id` 가 *다른 값* (다른 탭/사용자) 이거나 *누락* (HTTP `/workbench` 등 외부 경로) 이면 그대로 적용 — 협업 경로 회귀 0.

### 6. rhwp-studio/e2e/sub6-ws-echo-skip.test.mjs (신규)

3 시나리오:

1. `client_id='A'` 발신 → A·B 두 WS 모두 `origin_client_id='A'` 수신 (A 는 skip, B 는 적용)
2. `client_id` 누락 발신 → broadcast 의 `origin_client_id` 키 자체 부재 (skip_serializing_if 동작)
3. HTTP `POST /workbench` 경유 → broadcast 의 `origin_client_id` 키 부재 (외부 경로 회귀 0)

Node 22+ 빌트인 `globalThis.WebSocket` 사용 (sub2-helpers / ws-bridge 패턴 답습) — 신규 의존성 0.

## 검증

### 단위

- `cargo test -p rhwp-server` — **74 tests pass** (Sub-6 신규 4건 포함, 회귀 0)
  - `client_message_ops_parses_client_id`
  - `server_event_ops_serializes_origin_client_id_when_present`
  - `server_event_ops_skips_origin_client_id_when_none`
  - `publish_carries_origin_client_id_through_broadcast`

### 빌드

- `cargo build --release` — rhwp-server 22.96s ok
- `npm run build` (vite) — tsc + vite build 모두 ok, PWA SW 53 entries 정상

### e2e 회귀

17 시나리오 전수 통과:

- sub6-ws-echo-skip (3) — 본 sub
- ws-bridge — 양방향 WS 통신
- sub2 15건 — undo/audit/diff/ir-slice + EditOperation variant 통합
- sub3-ir-compact — Compact IR slice
- sub4-patch-diff — PatchDiff 응답 셀 압축

### 사용자 검증

배포 후 브라우저에서 직접 타이핑 → 복제 0 확인 필요 (별도 보고).

## 효과

1. *self-echo 복제 차단* — 브라우저 직접 타이핑에서 600ms 디바운스 단위 복제 사고 해소.
2. *협업 경로 무영향* — 다른 client_id 의 ops 는 그대로 적용 → 다중 탭/사용자 동시 편집 시 broadcast 본래 목적 유지.
3. *외부 HTTP 경로 무영향* — `POST /workbench` 호출자는 broadcast 구독 안 함 + `origin_client_id=None` → 모든 클라가 받음 (기존 동작).
4. *구 클라 호환* — `Option<String>` + `#[serde(default)]` + `skip_serializing_if` — 필드 누락 메시지도 정상 파싱·직렬화.

## 트레이드오프

- *구 클라 (필드 미발급)* — self-skip 안 됨 → 복제 사고 그대로. 클라/서버 동시 배포 필요.
- *clientId 발급 시점* — `SessionClient` 인스턴스 생성 시 1회 발급, dispose/재생성 시 새 UUID. 같은 탭 안에서 세션 교체 (다른 fileId) 시 clientId 도 갱신됨 — 의도된 동작.

## 다음

`feature/jerry-command-expansion` 에 push → VM 재배포 → 사용자가 브라우저에서 빠른 타이핑으로 복제 0 확인.
