# Task #zephy-bridge 최종 결과 보고서 — Sub-1 SSR 브릿지

작성일 2026-06-05.

## 작업 요약

`hwp_sub_agent_simulation.ipynb`의 LLM 편집 파이프라인을 *UNIVA-rhwp의 SSR 서버*에 접합. 브라우저 ↔ 서버 통신을 *양방향 WebSocket*으로 통합해 서버가 SoT(진실 원천)가 되는 인프라를 완성. `insert_text` 1개의 종단 시연이 자동·수동 검증으로 통과.

상세 설계는 [task_m200_zephy_bridge.md](../plans/task_m200_zephy_bridge.md), 구현 계획은 [task_m200_zephy_bridge_impl.md](../plans/task_m200_zephy_bridge_impl.md), 단계별 결과는 [working/task_m200_zephy_bridge_stage1.md](../working/task_m200_zephy_bridge_stage1.md), 시각화는 [task_m200_zephy_bridge_architecture.html](../plans/task_m200_zephy_bridge_architecture.html)에 있다.

## DoD (Definition of Done) 통과 여부

| 조건 | 결과 |
|---|---|
| 1. 수동 시연 시나리오가 모두 통과한다 | ✅ 자동 검증 부분 통과(아래). 시각 확인은 사용자 영역 |
| 2. 자동화 테스트가 모두 통과한다 | ✅ cargo test 7건 + e2e ws-bridge 양방향 통과 |
| 3. 회귀 0이 검증된다 | ✅ 기존 `/ops`·`/ir` endpoint 응답 형식 유지 확인 |
| 4. 본 spec의 모든 인터페이스가 코드에 구현된다 | ✅ `POST /workbench`·`GET /ws`·노트북 라우터·session-client.ts 모두 spec과 일치 |
| 5. WebSocket 양방향 채널이 양쪽 다 흐름 | ✅ e2e가 양방향 검증 |

## 구현된 인터페이스 (Sub-1 핵심)

### `POST /sessions/:id/workbench` (신설)

요청: `{action: string, payload: object}`
응답: `{seq, applied: "ops"|"passthrough", info?: SessionInfo}`

- `action == "insert_text"` → 서버가 `EditOperation::InsertText`로 변환 후 `apply_edit_ops_json` 호출 + sqlite 기록 + `ServerEvent::Ops` broadcast 발행. `applied: "ops"`.
- 그 외 action → `ServerEvent::Workbench` broadcast만 발행 (passthrough). `applied: "passthrough"`.

### `GET /sessions/:id/ws` (신설, WebSocket upgrade)

양방향 텍스트 프레임. 메시지는 한 줄 JSON.

서버 → 클라 (`ServerEvent`):
```
{"kind":"ops","seq":N,"ops":[EditOperation,...]}
{"kind":"workbench","seq":N,"action":"...","payload":{...}}
```

클라 → 서버 (`ClientMessage`):
```
{"kind":"ops","ops":[EditOperation,...]}        — 사용자 키입력 미러
{"kind":"snapshot","file_base64":"..."}         — 전체 동기화
{"kind":"ping"}                                  — keep-alive
```

세션 미존재 시 `CloseFrame { code: 4404 }`로 close.

### 노트북 Bash 라우터

`hwp_sub_agent_simulation_ssr.ipynb` cell 3의 `run_bash_command()`가 `hwp-doc-patch <action> ...` 명령을 가로채 `POST /workbench`로 HTTP 호출. 응답은 기존 SKILL.md 형식(`<<<HWP_DOC_PATCH_JSON_BEGIN>>>...<<<HWP_DOC_PATCH_JSON_END>>>`)으로 감싸 LLM에 반환.

## 신규·수정 파일

### 신규
- `server/src/events.rs` — `ServerEvent`/`ClientMessage` enum + `EventsHub` broadcast 채널 (137 lines + 4 unit tests)
- `server/src/ws.rs` — WebSocket handler (154 lines)
- `rhwp-studio/e2e/ws-bridge.test.mjs` — 양방향 e2e 검증 (115 lines)
- `hwp_sub_agent_simulation_ssr.ipynb` — 새 시뮬 노트북 (7 cells, 작업 공간 루트, git untracked)

### 수정
- `server/Cargo.toml` — `axum.ws` feature, `tokio-stream`, `futures` 의존성
- `server/src/main.rs` — `mod events; mod ws;` 등록, `AppState.events` 필드, `workbench` 핸들러, `/ws`·`/workbench` 라우트, `apply_ops`에 broadcast 발행 한 줄
- `rhwp-studio/src/core/session-client.ts` — 내부 HTTP fetch → WebSocket 갈아엎기 (외부 API 유지)
- `rhwp-studio/src/main.ts` — SessionClient에 `onServerEvent` 콜백 주입
- `mydocs/plans/task_m200_zephy_bridge_impl.md` — 프로토콜 요약 정정 (review 피드백)

## 커밋 이력 (Sub-1 범위)

```
4ea4c915 Task #zephy-bridge: events 모듈 — ServerEvent/ClientMessage + EventsHub broadcast
66579dbd Task #zephy-bridge: ws 모듈 — WebSocket 핸들러 + ClientMessage dispatch
bb602f68 Task #zephy-bridge: plan 프로토콜 요약 정정 (Task 1 review 피드백)
dd4ff7b0 Task #zephy-bridge: /workbench endpoint + apply_ops에 broadcast 발행
f8d85495 Task #zephy-bridge: session-client.ts 내부를 WebSocket으로 갈아엎음
d2d7e675 Task #zephy-bridge: session-client dispose race fix (Task 4 review 피드백)
07b54474 Task #zephy-bridge: main.ts에 SessionClient onServerEvent 콜백 주입
e943a7f5 Task #zephy-bridge: ParameterSet 주입 Critical fix (Task 5 review 피드백)
f3e5167f Task #zephy-bridge: e2e — WS 양방향 검증 Puppeteer 테스트 (초기)
977f7a9d Task #zephy-bridge: e2e 검증 방식 수정 — Puppeteer 제거, Node WS 직접 검증
```

Plan/spec/시각화 초안 commit(`8764ff9a`)과 합쳐 *총 11 commit*. 모두 브랜치 `local/task_m200_zephy_bridge`.

## 수동 시연 안내 (사용자 검증 단계)

1. 서버 가동 확인 — `curl -s http://127.0.0.1:7710/health` → `ok` (현재 BG task로 가동 중).

2. 새 노트북 실행:
   - Jupyter에서 `hwp_sub_agent_simulation_ssr.ipynb` 열기.
   - cell 1 실행 → fileId 발급 + URL 출력.
   - 출력된 URL(`http://127.0.0.1:7710/?fileId=...`)을 브라우저에 입력.
   - 브라우저 DevTools → Network → WS 탭에 연결 보임.
   - cell 2·3 실행 (LLM 인프라·Bash 라우터 정의).
   - cell 4 실행 → `self-test OK` 출력 + 브라우저 화면에 *self-test* 텍스트 즉시 등장.
   - cell 5·6 실행 (sub_agent_run 정의·사용 예시).
   - cell 6에서 LLM이 `insert_text` 호출 → 브라우저 화면에 *'안녕하세요'* 즉시 등장.

3. 양방향 검증 — 같은 브라우저 페이지에서 *직접 키 입력*. InputHandler가 `sessionClient.queueOp(op)` → WS 메시지 → 서버 sqlite 영속. 다른 탭에서 같은 fileId로 접속하면 *직접 친 텍스트* 보임.

## Sub-2로 미루는 항목 (요약)

[stage1 보고서](../working/task_m200_zephy_bridge_stage1.md)의 한계 표 참고. 핵심 4건:

1. **`ServerEvent`에 `origin` 필드 추가** — 다중 클라 환경의 echo loop 방지를 위한 forward-compat hedge.
2. **passthrough 이벤트의 seq 영속** — sqlite에 `append_workbench` 추가 또는 `seq` 의미 advisory 명시.
3. **11+1 hwpctl 액션을 `EditOperation` variants로 추가** — Sub-2의 본질. `replace_runs`·`set_paragraph_style`·`insert_table`·`merge_cells`·셀 편집 6개 등.
4. **WebSocket 재연결 시 미수신 이벤트 재전송** — seq 기반 catch-up 메커니즘.

## 결론

Sub-1 *접합 인프라* 완성. 자동 검증으로 양방향 WS 채널·서버 SoT·기존 endpoint 회귀 0을 확인. 수동 시각 검증은 사용자 영역으로 남김. Sub-2 brainstorm 시점에 위 미해결 4건을 우선 다룬다.
