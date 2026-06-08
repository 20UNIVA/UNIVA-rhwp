# Sub-6 — WS broadcast self-echo 차단 (origin_client_id 라벨링)

## 배경

브라우저에서 텍스트를 빠르게 치면 *600ms 디바운스 단위마다* 입력 묶음이 한 번 더 화면에 박힌다 (예: `abcdefg` → `abcdefgabcdefg`). 원인은 서버의 broadcast 채널 동작:

1. 키 입력 → 브라우저 `wasm.insertText(...)` *즉시 로컬 적용*
2. `sessionClient.queueOp(...)` → 600ms 디바운스 후 WS `{kind:'ops', ops:[...]}` 전송 ([session-client.ts:111](../../rhwp-studio/src/core/session-client.ts#L111))
3. 서버 `handle_client_text` → `apply_op_with_stash` → `events.publish(ServerEvent::Ops {...})` ([ws.rs:96-117](../../server/src/ws.rs#L96))
4. *같은 WS 연결의 send_task* 가 자기 fileId broadcast 를 subscribe 중 ([ws.rs:47](../../server/src/ws.rs#L47)) — `tokio::sync::broadcast` 는 *self-subscribe 시 자기 발행도 받음*
5. 클라 `onServerEvent` 가 받음 → `wasm.insertText(...)` *두 번째 적용* ([main.ts:124-143](../../rhwp-studio/src/main.ts#L124))

영향 받는 입력 경로:

| 경로 | 영향 | 비고 |
|---|---|---|
| 브라우저 키 입력 (`queueOp` → WS) | **복제** | 본 사고 |
| HTTP `POST /workbench` (외부 모델/노트북) | 정상 | self echo 없음 (HTTP 호출은 broadcast 구독 안 함) |
| 협업 — 다른 탭/사용자의 WS 입력 | 정상 (그대로 받아야 함) | broadcast 의 본래 목적 |

## 목표

WS `Ops` 메시지에 *발신 식별자* 를 실어, 서버 broadcast 가 원본 발신자에게 echo 될 때 클라가 *자기 메시지를 인지하고 skip* 한다.

1. `ClientMessage::Ops` 에 `client_id: Option<String>` 추가
2. `ServerEvent::Ops` 에 `origin_client_id: Option<String>` 추가
3. `apply_op_with_stash` 가 origin 을 받아 broadcast 페이로드에 attach
4. `SessionClient` 가 생성 시점에 `crypto.randomUUID()` 로 자기 client_id 발급, 모든 `flushOps` 메시지에 포함
5. `main.ts` 의 `onServerEvent` 가 `ev.origin_client_id === sessionClient.getClientId()` 면 *전체 분기 skip*

## 비목표

- HTTP `/workbench` 동작 변경 (origin=None 그대로 — 외부 호출자는 self echo 없음)
- `ServerEvent::Workbench` / `Complete` / `SnapshotRestored` 에 origin 추가 (echo 사고가 발생하지 않는 경로)
- 협업 시나리오 — 다른 client_id 가 보낸 ops 는 그대로 적용 (broadcast 의 본래 목적)
- e2e 헬퍼·노트북 변경 (외부 HTTP 호출이라 영향 없음)

## 설계

### Protocol — JSON wire format

**ClientMessage::Ops (브라우저 → 서버)**

```json
{
  "kind": "ops",
  "client_id": "550e8400-e29b-41d4-a716-446655440000",
  "ops": [{"op": "insert_text", "section": 0, "para": 0, "offset": 0, "text": "a"}]
}
```

`client_id` 누락 시 서버는 `None` 으로 처리 (구 클라 호환). 누락 = self-skip 불가 (구 동작 유지) → 점진적 배포 가능.

**ServerEvent::Ops (서버 → 모든 클라)**

```json
{
  "kind": "ops",
  "seq": 42,
  "origin_client_id": "550e8400-e29b-41d4-a716-446655440000",
  "ops": [...]
}
```

`origin_client_id` 가 *자기 것* 이면 클라가 skip. *다른 것* 이거나 *null* (= HTTP /workbench 발신) 이면 그대로 적용.

### 서버 코드 변경

**server/src/events.rs**

```rust
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerEvent {
    Ops {
        seq: i64,
        ops: Vec<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin_client_id: Option<String>,
    },
    // ... 나머지 동일
}

pub enum ClientMessage {
    Ops {
        ops: Vec<serde_json::Value>,
        #[serde(default)]
        client_id: Option<String>,
    },
    // ... 나머지 동일
}
```

`skip_serializing_if` 로 None 일 때 직렬화 누락 → 기존 JSON 응답 형태 유지 (e2e 회귀 면역).

**server/src/main.rs — apply_op_with_stash 시그니처**

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
            origin_client_id,
        },
    );
    // ...
}
```

**HTTP 12개 호출부** ([main.rs:580~906](../../server/src/main.rs#L580)) — 모두 `None` 전달:

```rust
let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
```

**WS 호출부** ([ws.rs:96-117](../../server/src/ws.rs#L96)) — ClientMessage 에서 받은 client_id 그대로 전달:

```rust
ClientMessage::Ops { ops, client_id } => {
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

### 클라 코드 변경

**rhwp-studio/src/core/session-client.ts**

```ts
export class SessionClient {
    private readonly clientId: string = crypto.randomUUID();

    getClientId(): string {
        return this.clientId;
    }

    flushOps(): Promise<void> {
        if (this.queue.length === 0) return Promise.resolve();
        const ops = this.queue;
        this.queue = [];
        const msg = JSON.stringify({
            kind: 'ops',
            client_id: this.clientId,   // 신규
            ops,
        });
        this.sendOrBuffer(msg);
        return Promise.resolve();
    }
}
```

**rhwp-studio/src/main.ts — onServerEvent 진입부 가드**

```ts
onServerEvent: (ev) => {
    if (!ev || typeof ev !== 'object' || !('kind' in ev)) { /* ... */ return; }

    // [Sub-6] self-echo skip — 자기 client_id 가 발행한 op 는 로컬에 이미 적용됨.
    if (ev.kind === 'ops'
        && typeof (ev as any).origin_client_id === 'string'
        && (ev as any).origin_client_id === sessionClient?.getClientId()) {
        return;
    }

    if (ev.kind === 'ops') { /* 기존 분기 */ }
    // ...
}
```

## 변경 파일

| 파일 | 변경 |
|---|---|
| [server/src/events.rs](../../server/src/events.rs) | `ServerEvent::Ops.origin_client_id`, `ClientMessage::Ops.client_id` 두 필드 추가 |
| [server/src/main.rs](../../server/src/main.rs) | `apply_op_with_stash` 시그니처 + 12 HTTP 호출부 `None` 추가 |
| [server/src/ws.rs](../../server/src/ws.rs) | `ClientMessage::Ops { ops, client_id }` 패턴 + 호출 시 전달 |
| [rhwp-studio/src/core/session-client.ts](../../rhwp-studio/src/core/session-client.ts) | `clientId` 필드 + `getClientId()` + `flushOps` 페이로드 attach |
| [rhwp-studio/src/main.ts](../../rhwp-studio/src/main.ts) | `onServerEvent` 진입부 self-echo 가드 |

## 단계 분해 (sub-agent 한 명에게 일괄 위임)

### Step 1 — Protocol 양쪽 동시 변경

- `server/src/events.rs` 두 enum 변종에 필드 추가 (serde default + skip_serializing_if)
- `server/src/main.rs::apply_op_with_stash` 시그니처 + body 의 publish 호출
- `server/src/main.rs` 12 HTTP 호출부 `None` 추가
- `server/src/ws.rs::handle_client_text` 의 `ClientMessage::Ops` 패턴 매칭

**검증**: `cargo build` 통과, `cargo test --no-run` 통과.

### Step 2 — 서버 단위 테스트

- `events.rs` 의 `publish_delivers_to_subscriber` 테스트가 origin 필드 추가에도 동작하는지 (Option 이라 호환)
- ws.rs 신규 테스트: `client_id` 가 있는 메시지 → broadcast 의 `origin_client_id` 가 같은 값

**검증**: `cargo test` 70+ tests pass.

### Step 3 — 클라 변경

- `session-client.ts`: `clientId` 필드 + `getClientId()` + `flushOps` 페이로드
- `main.ts`: `onServerEvent` 진입부 self-echo 가드

**검증**: `npm run build` 통과.

### Step 4 — e2e 회귀 + 신규 시나리오

- 기존 9 e2e 전수 통과 확인
- 신규 e2e `sub6-ws-echo-skip.test.mjs`:
  1. 두 가짜 클라 (A, B) 가 같은 fileId WS 연결
  2. A 가 `{kind:'ops', client_id:'A', ops:[insert_text]}` 전송
  3. A 가 받은 broadcast 에 `origin_client_id:'A'` 가 있어야 함 (skip 의도)
  4. B 가 받은 broadcast 에 `origin_client_id:'A'` 가 있고, B 입장에서는 자기 것 아님 → 적용 대상
  5. (옵션) A 가 `client_id` 없이 보내면 broadcast 의 `origin_client_id` 가 직렬화 누락됨

**검증**: `node e2e/sub6-ws-echo-skip.test.mjs` 통과 + `npm run test:e2e:sub2-all` 회귀 통과.

### Step 5 — Live smoke test

`./dev_run.sh` 로 서버 띄우고 브라우저에서 직접 타이핑 → 600ms 디바운스 뒤에도 복제 안 일어남 확인.

### Step 6 — 보고서 + git push

- `mydocs/report/task_m200_zephy_bridge_sub6_report.md` 작성
- `feature/jerry-command-expansion` 에 commit + push

## 검증 체크리스트

- [ ] `cargo test` 회귀 0
- [ ] `npm run build` 통과 (`vite build`)
- [ ] e2e 9 시나리오 회귀 0
- [ ] 신규 `sub6-ws-echo-skip.test.mjs` 통과
- [ ] Live 브라우저 타이핑 복제 0 (수동 확인)
- [ ] HTTP `/workbench` 호출 시 모든 클라가 broadcast 받음 (외부 모델 경로 회귀 0)

## 리스크

- *구 클라 (client_id 없는 메시지)* — `Option<String>` + `serde default` 로 호환. 단 self-skip 안 됨 → 구 클라는 복제 사고 그대로. 클라/서버 동시 배포 필요.
- *HTTP /workbench → broadcast → 자기에게 echo* — HTTP 호출자는 WS 구독 안 함 → echo 안 받음 (관계없음).
- *Multiple SessionClient 인스턴스* — 같은 페이지에 두 개 만들면 clientId 가 달라 self-skip 안 됨. 현재 main.ts 는 하나만 유지 ([main.ts:59](../../rhwp-studio/src/main.ts#L59)) → 문제 없음.
- *Sub-2 의 ws.rs `[4-2 fix]* 회귀* — 본 변경은 페이로드 필드만 추가, apply 로직 동일.

## 다음 단계

승인 즉시 sub-agent dispatch → Step 1~6 일괄 진행. 작업 디렉토리 `/Users/yuniba_01/code/parallel-repo/multiple-agent-reconstruction/UNIVA-rhwp`, 브랜치 `feature/jerry-command-expansion`.
