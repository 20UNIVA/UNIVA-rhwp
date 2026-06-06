//! rhwp SSR 세션 서버.
//!
//! 문서별 `fileId`(=minio fileId) 단위로 `DocumentCore` 를 서버 메모리에 보유하고
//! sqlite 에 영속한다. 클라이언트(iframe) 가 닫혀도 상태가 유지되며, AI 모델은
//! `GET /sessions/{id}/ir` 로 현재 문서 상태(IR JSON)를 조회할 수 있다.
//!
//! minio 다운로드/업로드는 **외부 모듈** 책임이다. 본 서버는 input 으로
//! `fileId` + 파일 바이트만 받는다.
//!
//! ## API
//! - `POST   /sessions`              세션 생성/재생성 `{fileId, format?, fileBase64}`
//! - `POST   /sessions/{id}/ops`     연산형 patch 적용 `[EditOperation, ...]`
//! - `PUT    /sessions/{id}/snapshot` 스냅샷형 동기화 `{fileBase64}`
//! - `GET    /sessions/{id}/ir`      현재 상태 IR JSON (모델 조회)
//! - `DELETE /sessions/{id}`         메모리 세션 해제 (영속 유지)
//! - `GET    /health`                헬스 체크

mod events;
mod ws;
mod storage;
mod store;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use rhwp::DocumentCore;

use storage::Storage;
use store::{PersistedSession, Store};

/// 서버 공유 상태.
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) sessions: Arc<Mutex<HashMap<String, Arc<Mutex<Session>>>>>,
    pub(crate) store: Arc<Store>,
    pub(crate) storage: Arc<Storage>,
    pub(crate) events: events::EventsHub,
}

/// 메모리에 보유되는 단일 세션.
pub(crate) struct Session {
    pub(crate) core: DocumentCore,
    pub(crate) format: String,
    /// 저장(덮어쓰기) 시 사용할 파일명.
    pub(crate) filename: String,
    /// 다음에 부여할 op/snapshot seq.
    pub(crate) next_seq: i64,
}

/// format 기반 기본 파일명.
fn default_filename(file_id: &str, format: &str) -> String {
    format!("{file_id}.{format}")
}

// ─── 요청/응답 DTO ────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateReq {
    file_id: String,
    format: Option<String>,
    file_base64: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotReq {
    file_base64: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateDocReq {
    filename: Option<String>,
    file_base64: String,
}

#[derive(Deserialize)]
struct ExportQuery {
    /// "hwp" | "hwpx". 미지정 시 세션 생성 시 포맷을 따른다.
    fmt: Option<String>,
}

#[derive(Deserialize)]
struct IrQuery {
    /// 0-based 페이지 번호. 미지정 시 전체 문서를 반환한다.
    page: Option<u32>,
}

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionInfo {
    file_id: String,
    seq: i64,
    section_count: usize,
    paragraph_count: usize,
}

// ─── 에러 ────────────────────────────────────────────

pub(crate) struct AppError {
    pub(crate) status: StatusCode,
    pub(crate) msg: String,
}

impl AppError {
    pub(crate) fn new(status: StatusCode, msg: impl Into<String>) -> Self {
        AppError {
            status,
            msg: msg.into(),
        }
    }
    pub(crate) fn bad_request(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, msg)
    }
    pub(crate) fn unprocessable(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, msg)
    }
    pub(crate) fn not_found(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, msg)
    }
    pub(crate) fn internal(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, msg)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.msg }))).into_response()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::internal(format!("sqlite: {e}"))
    }
}

// ─── 코어 빌드 헬퍼 ────────────────────────────────────

/// 파일 바이트를 파싱하여 레이아웃 준비된 `DocumentCore` 를 만든다.
fn build_core(bytes: &[u8]) -> Result<DocumentCore, AppError> {
    let doc = rhwp::parse_document(bytes)
        .map_err(|e| AppError::unprocessable(format!("문서 파싱 실패: {e}")))?;
    let mut core = DocumentCore::new_empty();
    core.set_document(doc);
    Ok(core)
}

/// 영속 데이터로부터 코어를 복원한다(base/snapshot + 이후 ops 재적용).
fn restore_core(p: &PersistedSession) -> Result<DocumentCore, AppError> {
    let mut core = build_core(&p.base_blob)?;
    if !p.ops.is_empty() {
        let joined = p
            .ops
            .iter()
            .map(|(_, j)| j.as_str())
            .collect::<Vec<_>>()
            .join(",");
        core.apply_edit_ops_json(&format!("[{joined}]"))
            .map_err(|e| AppError::internal(format!("op 재적용 실패: {e}")))?;
    }
    Ok(core)
}

/// 메모리 세션을 얻거나, 없으면 sqlite → (그래도 없으면) minio download 순으로 복원하여 등록한다.
pub(crate) async fn get_or_restore(state: &AppState, file_id: &str) -> Result<Arc<Mutex<Session>>, AppError> {
    // 1) 메모리
    {
        let guard = state.sessions.lock().unwrap();
        if let Some(s) = guard.get(file_id) {
            return Ok(s.clone());
        }
    }
    // 2) sqlite 복원 (작업 중 상태 우선 — 편집 진행분 보존)
    if let Some(persisted) = state.store.load(file_id)? {
        let core = restore_core(&persisted)?;
        let filename = default_filename(file_id, &persisted.format);
        let session = Arc::new(Mutex::new(Session {
            core,
            format: persisted.format,
            filename,
            next_seq: persisted.last_seq + 1,
        }));
        state
            .sessions
            .lock()
            .unwrap()
            .insert(file_id.to_string(), session.clone());
        return Ok(session);
    }
    // 3) minio download 폴백 (외부가 fileId만 지정하고 진입한 경우)
    if state.storage.enabled() {
        let bytes = state
            .storage
            .download(file_id)
            .await
            .map_err(|e| AppError::not_found(format!("세션·저장소 모두 없음: {file_id} ({e})")))?;
        let core = build_core(&bytes)?;
        state.store.create_session(file_id, "hwp", &bytes)?;
        let session = Arc::new(Mutex::new(Session {
            core,
            format: "hwp".to_string(),
            filename: default_filename(file_id, "hwp"),
            next_seq: 1,
        }));
        state
            .sessions
            .lock()
            .unwrap()
            .insert(file_id.to_string(), session.clone());
        return Ok(session);
    }
    Err(AppError::not_found(format!("세션 없음: {file_id}")))
}

/// 세션 정보 요약(문단 합계 포함)을 만든다.
pub(crate) fn session_info(file_id: &str, session: &Session) -> SessionInfo {
    let doc = session.core.document();
    let paragraph_count = doc.sections.iter().map(|s| s.paragraphs.len()).sum();
    SessionInfo {
        file_id: file_id.to_string(),
        seq: session.next_seq - 1,
        section_count: doc.sections.len(),
        paragraph_count,
    }
}

// ─── 핸들러 ───────────────────────────────────────────

async fn health() -> &'static str {
    "ok"
}

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateReq>,
) -> Result<Json<SessionInfo>, AppError> {
    let bytes = STANDARD
        .decode(req.file_base64.as_bytes())
        .map_err(|e| AppError::bad_request(format!("base64 디코드 실패: {e}")))?;
    let core = build_core(&bytes)?;
    let format = req.format.unwrap_or_else(|| "hwpx".to_string());

    state.store.create_session(&req.file_id, &format, &bytes)?;

    let filename = default_filename(&req.file_id, &format);
    let session = Session {
        core,
        format,
        filename,
        next_seq: 1,
    };
    let info = session_info(&req.file_id, &session);
    state
        .sessions
        .lock()
        .unwrap()
        .insert(req.file_id.clone(), Arc::new(Mutex::new(session)));
    Ok(Json(info))
}

/// 파일을 minio에 업로드하여 file_id를 발급받고, 그 file_id로 세션을 생성한다.
/// fileId가 없는 신규 문서(빈 문서 포함)의 진입점.
async fn create_document(
    State(state): State<AppState>,
    Json(req): Json<CreateDocReq>,
) -> Result<Json<SessionInfo>, AppError> {
    let bytes = STANDARD
        .decode(req.file_base64.as_bytes())
        .map_err(|e| AppError::bad_request(format!("base64 디코드 실패: {e}")))?;
    let filename = req.filename.unwrap_or_else(|| "document.hwp".to_string());

    // 1) 파싱 검증 (업로드 전에 유효성 확인)
    let core = build_core(&bytes)?;
    let format = if filename.to_lowercase().ends_with("hwpx") {
        "hwpx"
    } else {
        "hwp"
    }
    .to_string();

    // 2) minio upload → file_id (신규: file_id 미지정)
    let file_id = state
        .storage
        .upload(bytes.clone(), &filename, None)
        .await
        .map_err(|e| AppError::internal(format!("저장소 업로드 실패: {e}")))?
        .file_id;

    // 3) 발급된 file_id로 세션 생성
    state.store.create_session(&file_id, &format, &bytes)?;
    let session = Session {
        core,
        format,
        filename,
        next_seq: 1,
    };
    let info = session_info(&file_id, &session);
    state
        .sessions
        .lock()
        .unwrap()
        .insert(file_id.clone(), Arc::new(Mutex::new(session)));
    Ok(Json(info))
}

async fn apply_ops(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Json(ops): Json<Vec<serde_json::Value>>,
) -> Result<Json<SessionInfo>, AppError> {
    let session = get_or_restore(&state, &file_id).await?;
    let mut s = session.lock().unwrap();

    let ops_json = serde_json::to_string(&ops)
        .map_err(|e| AppError::bad_request(format!("ops 직렬화 실패: {e}")))?;
    s.core
        .apply_edit_ops_json(&ops_json)
        .map_err(|e| AppError::unprocessable(format!("op 적용 실패: {e}")))?;

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
    Ok(Json(session_info(&file_id, &s)))
}

async fn put_snapshot(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Json(req): Json<SnapshotReq>,
) -> Result<Json<SessionInfo>, AppError> {
    let session = get_or_restore(&state, &file_id).await?;
    let bytes = STANDARD
        .decode(req.file_base64.as_bytes())
        .map_err(|e| AppError::bad_request(format!("base64 디코드 실패: {e}")))?;
    let core = build_core(&bytes)?;

    let mut s = session.lock().unwrap();
    s.core = core;
    let seq = s.next_seq;
    state.store.append_snapshot(&file_id, seq, &bytes)?;
    s.next_seq += 1;
    Ok(Json(session_info(&file_id, &s)))
}

async fn get_ir(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(q): Query<IrQuery>,
) -> Result<Response, AppError> {
    let session = get_or_restore(&state, &file_id).await?;
    let s = session.lock().unwrap();
    // page 미지정 → 전체, page=n → 해당 페이지 문단만(절대 인덱스 유지 → 편집 op 그대로 유효)
    let json = s
        .core
        .to_ir_json_paged(q.page)
        .map_err(|e| AppError::internal(format!("IR 직렬화 실패: {e}")))?;
    Ok(([(header::CONTENT_TYPE, "application/json")], json).into_response())
}

/// 현재 세션 문서를 hwp/hwpx 바이너리로 내보낸다.
///
/// **확정 저장 경계**: 외부 모듈(minio 연동)이 이 엔드포인트로 바이트를 받아
/// minio에 업로드한다. 본 서버는 바이트 제공까지만 책임진다.
async fn export(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(q): Query<ExportQuery>,
) -> Result<Response, AppError> {
    let session = get_or_restore(&state, &file_id).await?;
    let s = session.lock().unwrap();
    let doc = s.core.document();
    let fmt = q.fmt.as_deref().unwrap_or(&s.format);

    let (bytes, ext) = match fmt {
        "hwpx" => (
            rhwp::serializer::serialize_hwpx(doc)
                .map_err(|e| AppError::internal(format!("hwpx 직렬화 실패: {e:?}")))?,
            "hwpx",
        ),
        _ => (
            rhwp::serialize_document(doc)
                .map_err(|e| AppError::internal(format!("hwp 직렬화 실패: {e:?}")))?,
            "hwp",
        ),
    };

    let disposition = format!("attachment; filename=\"{file_id}.{ext}\"");
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        bytes,
    )
        .into_response())
}

/// 저장 — 현재 세션 문서를 같은 file_id로 minio에 덮어쓰기 업로드한다.
/// (에디터 "저장" 버튼이 호출. 외부 upload API에 file_id 포함 → 해당 위치 덮어씀)
async fn save_document(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let session = get_or_restore(&state, &file_id).await?;
    // lock 안에서 동기 직렬화 → guard drop 후 await upload (MutexGuard는 await 경계를 넘지 않음)
    let (bytes, filename) = {
        let s = session.lock().unwrap();
        let doc = s.core.document();
        let bytes = match s.format.as_str() {
            "hwpx" => rhwp::serializer::serialize_hwpx(doc)
                .map_err(|e| AppError::internal(format!("hwpx 직렬화 실패: {e:?}")))?,
            _ => rhwp::serialize_document(doc)
                .map_err(|e| AppError::internal(format!("hwp 직렬화 실패: {e:?}")))?,
        };
        (bytes, s.filename.clone())
    };
    let res = state
        .storage
        .upload(bytes, &filename, Some(&file_id))
        .await
        .map_err(|e| AppError::internal(format!("저장(덮어쓰기) 실패: {e}")))?;
    Ok(Json(json!({
        "fileId": res.file_id,
        "minioKey": res.minio_key,
        "updated": res.updated,
    })))
}

/// 단일 EditOperation 을 적용하면서 sqlite op_stash 영속 + broadcast 한 묶음.
/// 1. core.export_hwpx_native() → before_blob
/// 2. core.apply_edit_op(&op)
/// 3. store.append_op_stash(file_id, seq, op_json, before_blob)
/// 4. events.publish(ServerEvent::Ops { seq, ops: [op] })
#[allow(dead_code)]
async fn apply_op_with_stash(
    state: &AppState,
    file_id: &str,
    session: Arc<Mutex<Session>>,
    op: rhwp::document_core::EditOperation,
) -> Result<i64, AppError> {
    let before_blob = {
        let s = session.lock().unwrap();
        s.core
            .export_hwpx_native()
            .map_err(|e| AppError::internal(format!("export_hwpx_native: {e}")))?
    };

    let op_json = serde_json::to_value(&op)
        .map_err(|e| AppError::internal(format!("op 직렬화: {e}")))?;
    let op_json_str = op_json.to_string();

    let seq = {
        let mut s = session.lock().unwrap();
        s.core
            .apply_edit_op(&op)
            .map_err(|e| AppError::unprocessable(format!("apply_edit_op: {e}")))?;
        let seq = s.next_seq;
        s.next_seq += 1;
        seq
    };

    state
        .store
        .append_op_stash(file_id, seq, &op_json_str, &before_blob)
        .map_err(|e| AppError::internal(format!("op_stash 영속: {e}")))?;

    state.events.publish(
        file_id,
        events::ServerEvent::Ops {
            seq,
            ops: vec![op_json],
        },
    );

    Ok(seq)
}

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

async fn delete_session(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> impl IntoResponse {
    state.sessions.lock().unwrap().remove(&file_id);
    StatusCode::NO_CONTENT
}

fn router(state: AppState) -> Router {
    let mut app = Router::new()
        .route("/health", get(health))
        .route("/sessions", post(create_session))
        .route("/documents", post(create_document))
        .route("/sessions/:id/ops", post(apply_ops))
        .route("/sessions/:id/snapshot", put(put_snapshot))
        .route("/sessions/:id/ir", get(get_ir))
        .route("/sessions/:id/export", get(export))
        .route("/sessions/:id/save", post(save_document))
        .route("/sessions/:id/workbench", post(workbench))
        .route("/sessions/:id/ws", get(ws::ws_upgrade))
        .route("/sessions/:id", delete(delete_session))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // RHWP_STUDIO_DIR 가 지정되면 studio 정적 자산(dist)도 같은 포트에서 서빙한다.
    // → single-origin 배포(별도 웹서버/CORS 불필요). 미지정 시 API 전용(기존 동작).
    if let Ok(dir) = std::env::var("RHWP_STUDIO_DIR") {
        if !dir.is_empty() {
            tracing::info!("studio 정적 서빙: {dir}");
            app = app.fallback_service(
                tower_http::services::ServeDir::new(dir).append_index_html_on_directories(true),
            );
        }
    }
    app
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rhwp_server=info,tower_http=info".into()),
        )
        .init();

    let db_path = std::env::var("RHWP_SERVER_DB").unwrap_or_else(|_| "rhwp-sessions.db".to_string());
    let store = Store::open(&db_path).expect("sqlite 열기 실패");
    let storage = Storage::from_env();
    tracing::info!("외부 저장소 연동: {}", if storage.enabled() { "활성" } else { "비활성(UPLOAD_URL/DOWNLOAD_URL 미설정)" });
    let state = AppState {
        sessions: Arc::new(Mutex::new(HashMap::new())),
        store: Arc::new(store),
        storage: Arc::new(storage),
        events: events::EventsHub::new(),
    };

    let addr = std::env::var("RHWP_SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:7710".to_string());
    let listener = TcpListener::bind(&addr).await.expect("bind 실패");
    tracing::info!("rhwp-server listening on {addr} (db={db_path})");

    axum::serve(listener, router(state))
        .await
        .expect("서버 종료됨");
}
