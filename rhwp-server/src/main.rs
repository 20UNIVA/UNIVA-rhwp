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
mod ir_compact;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, Query, State},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;

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
    /// 브라우저 (rhwp-studio WASM) 가 자기 화면 paginate 결과를 *역공급* 한 page → para 매핑.
    /// 측정기 격차 (native EmbeddedTextMeasurer ↔ WASM Canvas) 로 native paginator 가
    /// 브라우저 화면과 다른 페이지 경계를 그릴 때, ir-slice 가 *사용자가 본 화면* 을
    /// 진실로 삼게 해 주는 우회 경로. file_id 단위 보관.
    pub(crate) page_maps: Arc<Mutex<HashMap<String, ClientPageMap>>>,
}

/// 클라이언트 (브라우저 WASM) 가 paginate 후 POST 한 page → (sec, para_start, para_end) 묶음.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ClientPageEntry {
    /// 1-based 페이지 번호 (사용자·모델 직관 정합).
    pub(crate) page: u32,
    pub(crate) sec: usize,
    pub(crate) para_start: usize,
    pub(crate) para_end: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ClientPageMap {
    pub(crate) entries: Vec<ClientPageEntry>,
    pub(crate) total_pages: u32,
    /// 브라우저가 이 map 을 만들 때 *마지막으로 적용한* op seq. 응답 staleness 판단용
    /// (현재는 항상 사용 — 미세 stale 가 측정기 격차보다 작음).
    pub(crate) seq: i64,
}

/// 메모리에 보유되는 단일 세션.
pub(crate) struct Session {
    pub(crate) core: DocumentCore,
    pub(crate) format: String,
    /// 저장(덮어쓰기) 시 사용할 파일명.
    pub(crate) filename: String,
    /// 다음에 부여할 op/snapshot seq.
    pub(crate) next_seq: i64,
    /// 이 세션을 만든 사용자 식별자. storage 호출 시 vfinder 의 `X-Vfinder-User`
    /// 헤더로 박는다. 누락된 자리에서 만들어진 세션은 storage 동작이 실패한다.
    pub(crate) user_id: String,
}

/// format 기반 기본 파일명.
fn default_filename(file_id: &str, format: &str) -> String {
    format!("{file_id}.{format}")
}

/// 요청에서 추출되는 사용자 식별자 — `X-Rhwp-User` 헤더, 누락 시 `RHWP_DEFAULT_USER`
/// 환경변수 폴백. 둘 다 비면 400.
///
/// 폴백은 *agent 부모창 연동 전까지* 의 임시 자리. agent 가 모든 요청에 헤더를 박는
/// 약속이 정착되면 폴백을 제거해 강결합으로 굳힌다 (`docs/13-rhwp-vfinder-storage-integration.md` §10).
pub(crate) struct RhwpUser(pub(crate) String);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for RhwpUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(v) = parts.headers.get("X-Rhwp-User") {
            if let Ok(s) = v.to_str() {
                if !s.is_empty() {
                    return Ok(RhwpUser(s.to_string()));
                }
            }
        }
        if let Ok(s) = std::env::var("RHWP_DEFAULT_USER") {
            if !s.is_empty() {
                return Ok(RhwpUser(s));
            }
        }
        Err(AppError::bad_request(
            "사용자 식별자 누락 — X-Rhwp-User 헤더 또는 RHWP_DEFAULT_USER 환경변수 필요",
        ))
    }
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
#[serde(rename_all = "camelCase")]
struct CreateBlankReq {
    /// 새 문서 파일명. `보고서.hwp` / `초안.hwpx` 등. 확장자가 format 결정에 사용된다.
    /// 미명시 시 `format` 인자 (또는 기본 `"hwp"`) 로 `document.<ext>` 가 자동 생성.
    filename: Option<String>,
    /// `"hwp"` / `"hwpx"`. 미명시 시 filename 확장자로 결정, 그것도 부재면 `"hwp"`.
    format: Option<String>,
}

#[derive(Deserialize)]
struct ExportQuery {
    /// "hwp" | "hwpx". 미지정 시 세션 생성 시 포맷을 따른다.
    fmt: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveAsReq {
    /// vfinder 의 부모 폴더 (root 기준 절대경로). save-as iframe 에서 사용자가 고른 자리.
    path: String,
    /// 새 파일명 (확장자 포함). save-as iframe 의 이름 입력 칸 자리.
    name: String,
    /// 이름 충돌 시 *덮어쓰기* 선택 자리. false 면 vfinder 가 `(1)` suffix 정책으로 신규 저장.
    #[serde(default)]
    overwrite: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveAsResp {
    /// vfinder 가 새로 발급한 file_id.
    file_id: String,
    /// 실제로 저장된 경로 (suffix 가 박혔으면 그 모양 그대로).
    path: String,
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
    /// Sub-4: 편집 연산 적용 전후 IR 스냅샷. ops 분기에서만 채워지며 passthrough/undo/
    /// complete 에서는 None. 모델이 응답만 보고 *정말 바뀌었는지* 와 *어떻게 바뀌었는지* 를
    /// 즉시 확인할 수 있다.
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<ir_compact::PatchDiff>,
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
    pub(crate) fn conflict(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, msg)
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
    // [Plan A.1] DocumentCore::from_bytes 단일 진입점 사용 — wasm 빌드(studio)와
    // 동일한 후처리 (reflow_zero_height_paragraphs / normalize_hwpx_paragraphs /
    // clear_initial_field_texts) 를 거치도록 통일. 종전 set_document 경로는 이 4단계를
    // 건너뛰어 TAC 표 paragraph 의 line_height 가 표 높이 미반영 (1000 유지) 상태로
    // server 메모리에 박혀, wasm paginate(4페이지) 와 server paginate(3페이지) 격차의
    // root cause 가 되었다.
    DocumentCore::from_bytes(bytes)
        .map_err(|e| AppError::unprocessable(format!("문서 파싱 실패: {e}")))
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

/// 메모리 세션을 얻거나, 없으면 sqlite → (그래도 없으면) 외부 저장소 download 순으로
/// 복원하여 등록한다.
///
/// `user_id` — 외부 저장소(vfinder) 의 `X-Vfinder-User` 헤더로 박는 자리. 메모리/sqlite
/// 캐시 hit 자리에서는 *기존 세션 user* 를 그대로 쓰고, 미스 자리에서만 인자 user 가 세션
/// 의 user 자리로 박힌다.
pub(crate) async fn get_or_restore(
    state: &AppState,
    file_id: &str,
    user_id: &str,
) -> Result<Arc<Mutex<Session>>, AppError> {
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
            user_id: user_id.to_string(),
        }));
        state
            .sessions
            .lock()
            .unwrap()
            .insert(file_id.to_string(), session.clone());
        return Ok(session);
    }
    // 3) 외부 저장소 download 폴백 (외부가 fileId만 지정하고 진입한 경우)
    if state.storage.enabled() {
        let bytes = state
            .storage
            .download(file_id, user_id)
            .await
            .map_err(|e| AppError::not_found(format!("세션·저장소 모두 없음: {file_id} ({e})")))?;
        let core = build_core(&bytes)?;
        state.store.create_session(file_id, "hwp", &bytes)?;
        let session = Arc::new(Mutex::new(Session {
            core,
            format: "hwp".to_string(),
            filename: default_filename(file_id, "hwp"),
            next_seq: 1,
            user_id: user_id.to_string(),
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
    RhwpUser(user_id): RhwpUser,
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
        user_id,
    };
    let info = session_info(&req.file_id, &session);
    state
        .sessions
        .lock()
        .unwrap()
        .insert(req.file_id.clone(), Arc::new(Mutex::new(session)));
    Ok(Json(info))
}

/// 파일을 외부 저장소에 업로드하여 file_id를 발급받고, 그 file_id로 세션을 생성한다.
/// fileId가 없는 신규 문서(빈 문서 포함)의 진입점.
async fn create_document(
    State(state): State<AppState>,
    RhwpUser(user_id): RhwpUser,
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

    // 2) 외부 저장소 upload → file_id (신규: file_id 미지정, target_path 도 미지정 — 서버 기본 자리)
    let file_id = state
        .storage
        .upload(bytes.clone(), &filename, None, None, false, &user_id)
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
        user_id,
    };
    let info = session_info(&file_id, &session);
    state
        .sessions
        .lock()
        .unwrap()
        .insert(file_id.clone(), Arc::new(Mutex::new(session)));
    Ok(Json(info))
}

/// 빈 hwp/hwpx 문서를 *서버 자체* 가 생성해 외부 저장소에 업로드한다. rhwp-studio 의
/// "새로 만들기" 흐름과 동등 — Rust core 의 `create_blank_document_native()` 가 메모리
/// IR 빈 문서를 만들고, 그것을 직렬화해 minio 에 올려 file_id 를 발급받는다. 호출자가
/// 빈 바이트를 base64 로 보낼 필요 없음 — `filename` (옵션) 과 `format` (옵션) 만 박는다.
async fn create_blank_document(
    State(state): State<AppState>,
    RhwpUser(user_id): RhwpUser,
    Json(req): Json<CreateBlankReq>,
) -> Result<Json<SessionInfo>, AppError> {
    // 1) format 결정: 명시 인자 > filename 확장자 > 기본 "hwp"
    let format = req
        .format
        .as_deref()
        .map(str::to_lowercase)
        .filter(|f| f == "hwp" || f == "hwpx")
        .or_else(|| {
            req.filename.as_deref().and_then(|f| {
                let lower = f.to_lowercase();
                if lower.ends_with(".hwpx") {
                    Some("hwpx".to_string())
                } else if lower.ends_with(".hwp") {
                    Some("hwp".to_string())
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| "hwp".to_string());

    let filename = req
        .filename
        .clone()
        .unwrap_or_else(|| format!("document.{format}"));

    // 2) Rust core 자체 함수로 빈 문서 IR 생성 (studio 의 createBlankDocument 와 동등).
    let mut core = rhwp::document_core::DocumentCore::new_empty();
    core.create_blank_document_native()
        .map_err(|e| AppError::internal(format!("빈 문서 생성 실패: {e}")))?;

    // 3) format 에 따라 hwp / hwpx 직렬화.
    let doc = core.document();
    let bytes = match format.as_str() {
        "hwpx" => rhwp::serializer::serialize_hwpx(doc)
            .map_err(|e| AppError::internal(format!("hwpx 직렬화 실패: {e:?}")))?,
        _ => rhwp::serialize_document(doc)
            .map_err(|e| AppError::internal(format!("hwp 직렬화 실패: {e:?}")))?,
    };

    // 4) 외부 저장소 upload → file_id 발급.
    let file_id = state
        .storage
        .upload(bytes.clone(), &filename, None, None, false, &user_id)
        .await
        .map_err(|e| AppError::internal(format!("저장소 업로드 실패: {e}")))?
        .file_id;

    // 5) 발급된 file_id 로 세션 생성 (메모리 + sqlite 양쪽).
    state.store.create_session(&file_id, &format, &bytes)?;
    let session = Session {
        core,
        format,
        filename,
        next_seq: 1,
        user_id,
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
    RhwpUser(user_id): RhwpUser,
    Path(file_id): Path<String>,
    Json(ops): Json<Vec<serde_json::Value>>,
) -> Result<Json<SessionInfo>, AppError> {
    let session = get_or_restore(&state, &file_id, &user_id).await?;
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
                // HTTP 경로 — 외부 호출이라 발신자 식별 없음.
                origin_client_id: None,
            },
        );
    }
    Ok(Json(session_info(&file_id, &s)))
}

async fn put_snapshot(
    State(state): State<AppState>,
    RhwpUser(user_id): RhwpUser,
    Path(file_id): Path<String>,
    Json(req): Json<SnapshotReq>,
) -> Result<Json<SessionInfo>, AppError> {
    let session = get_or_restore(&state, &file_id, &user_id).await?;
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
    RhwpUser(user_id): RhwpUser,
    Path(file_id): Path<String>,
    Query(q): Query<IrQuery>,
) -> Result<Response, AppError> {
    let session = get_or_restore(&state, &file_id, &user_id).await?;
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
    RhwpUser(user_id): RhwpUser,
    Path(file_id): Path<String>,
    Query(q): Query<ExportQuery>,
) -> Result<Response, AppError> {
    let session = get_or_restore(&state, &file_id, &user_id).await?;
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
    RhwpUser(user_id): RhwpUser,
    Path(file_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let session = get_or_restore(&state, &file_id, &user_id).await?;
    // lock 안에서 동기 직렬화 → guard drop 후 await upload (MutexGuard는 await 경계를 넘지 않음).
    // session 의 user 를 함께 추출 — *세션 주인* 의 user 로 storage 호출.
    let (bytes, filename, session_user) = {
        let s = session.lock().unwrap();
        let doc = s.core.document();
        let bytes = match s.format.as_str() {
            "hwpx" => rhwp::serializer::serialize_hwpx(doc)
                .map_err(|e| AppError::internal(format!("hwpx 직렬화 실패: {e:?}")))?,
            _ => rhwp::serialize_document(doc)
                .map_err(|e| AppError::internal(format!("hwp 직렬화 실패: {e:?}")))?,
        };
        (bytes, s.filename.clone(), s.user_id.clone())
    };
    let res = state
        .storage
        .upload(bytes, &filename, Some(&file_id), None, false, &session_user)
        .await
        .map_err(|e| AppError::internal(format!("저장(덮어쓰기) 실패: {e}")))?;
    Ok(Json(json!({
        "fileId": res.file_id,
        "minioKey": res.minio_key,
        "updated": res.updated,
    })))
}

/// 다른 이름으로 저장 — 현재 세션 문서를 *vfinder 의 새 자리* 에 *신규 file_id* 로 저장.
/// 호출자는 save-as iframe 결과(`path` 폴더 + `name` 이름 + `overwrite` 옵션)를 그대로 박는다.
/// 응답의 새 `fileId` 로 클라이언트가 `?fileId=` URL 을 갱신 — 그 시점부터 같은 세션이
/// *새 file_id* 자리에 묶인다.
async fn save_as(
    State(state): State<AppState>,
    RhwpUser(user_id): RhwpUser,
    Path(file_id): Path<String>,
    Json(req): Json<SaveAsReq>,
) -> Result<Json<SaveAsResp>, AppError> {
    let session = get_or_restore(&state, &file_id, &user_id).await?;
    let (bytes, session_user) = {
        let s = session.lock().unwrap();
        let doc = s.core.document();
        let bytes = match s.format.as_str() {
            "hwpx" => rhwp::serializer::serialize_hwpx(doc)
                .map_err(|e| AppError::internal(format!("hwpx 직렬화 실패: {e:?}")))?,
            _ => rhwp::serialize_document(doc)
                .map_err(|e| AppError::internal(format!("hwp 직렬화 실패: {e:?}")))?,
        };
        (bytes, s.user_id.clone())
    };

    // file_id=None → 신규 발급. target_path 자리에 사용자가 고른 폴더. overwrite 자리에 선택값.
    let res = state
        .storage
        .upload(bytes, &req.name, None, Some(&req.path), req.overwrite, &session_user)
        .await
        .map_err(|e| AppError::internal(format!("save-as 실패: {e}")))?;

    // 응답 path 자리 — *vfinder 가 응답에 minio_key 자리에 실 저장 path 를 박을 수도* 있고
    // 아닐 수도 있다. 둘 다 대비 — vfinder 응답의 minio_key 가 비면 호출 인자로 조립.
    let stored_path = res
        .minio_key
        .unwrap_or_else(|| format!("{}/{}", req.path.trim_end_matches('/'), req.name));
    Ok(Json(SaveAsResp {
        file_id: res.file_id,
        path: stored_path,
    }))
}

/// 단일 EditOperation 을 적용하면서 sqlite op_stash 영속 + broadcast 한 묶음.
/// 1. core.export_hwpx_native() → before_blob
/// 2. core.apply_edit_op(&op)
/// 3. store.append_op_stash(file_id, seq, op_json, before_blob)
/// 4. events.publish(ServerEvent::Ops { seq, ops: [op], origin_client_id })
///
/// `origin_client_id` — 그 op 의 *원래 발신자 브라우저 식별자*. WS 경로면 클라가 보낸
/// `client_id` 를 그대로 흘려보내고, HTTP `/workbench` 같은 외부 경로는 `None` 으로
/// 호출한다. 브라우저가 broadcast 의 self echo 를 식별·skip 하는 데 쓰인다.
pub(crate) async fn apply_op_with_stash(
    state: &AppState,
    file_id: &str,
    session: Arc<Mutex<Session>>,
    op: rhwp::document_core::EditOperation,
    origin_client_id: Option<String>,
) -> Result<(i64, Option<ir_compact::PatchDiff>), AppError> {
    // affected_range — apply 전후 IR 슬라이스 캡처 범위.
    let range = op.affected_range();

    // op tag — EditOperation 의 `op` 태그 (snake_case). 직렬화 시 "op" 필드를 가져와 사용.
    let op_json = serde_json::to_value(&op)
        .map_err(|e| AppError::internal(format!("op 직렬화: {e}")))?;
    let op_tag = op_json
        .get("op")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let op_json_str = op_json.to_string();

    // before 캡처 + export_hwpx (snapshot stash 용) — 같은 lock 안에서 처리.
    let (before_blob, before_target) = {
        let s = session.lock().unwrap();
        let blob = s
            .core
            .export_hwpx_native()
            .map_err(|e| AppError::internal(format!("export_hwpx_native: {e}")))?;
        let target = ir_compact::capture_before_target(&s.core, &range);
        (blob, target)
    };

    // 적용 + after 캡처. apply 가 실패하면 PatchDiff 없이 그대로 오류 전파.
    let (seq, after_target) = {
        let mut s = session.lock().unwrap();
        s.core
            .apply_edit_op(&op)
            .map_err(|e| AppError::unprocessable(format!("apply_edit_op: {e}")))?;
        let seq = s.next_seq;
        s.next_seq += 1;
        let target = ir_compact::capture_after_target(&s.core, &range);
        (seq, target)
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
            origin_client_id: origin_client_id.clone(),
        },
    );

    let diff = ir_compact::build_patch_diff(&op_tag, &range, before_target, after_target);
    Ok((seq, Some(diff)))
}

async fn workbench(
    State(state): State<AppState>,
    RhwpUser(user_id): RhwpUser,
    Path(file_id): Path<String>,
    Json(req): Json<WorkbenchReq>,
) -> Result<Json<WorkbenchResp>, AppError> {
    let session = get_or_restore(&state, &file_id, &user_id).await?;

    match req.action.as_str() {
        "insert_text" => {
            let section = req
                .payload
                .get("section")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| AppError::bad_request("payload.section 누락"))? as usize;
            let para = req
                .payload
                .get("para")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| AppError::bad_request("payload.para 누락"))? as usize;
            let offset = req
                .payload
                .get("offset")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| AppError::bad_request("payload.offset 누락"))? as usize;
            let text = req
                .payload
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::bad_request("payload.text 누락"))?
                .to_string();

            // [4-2 fix] insert_text 도 op_stash 적재 — POST /undo 가 사용자 키 입력도 되돌림.
            let op = rhwp::document_core::EditOperation::InsertText {
                section,
                para,
                offset,
                text,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            let info = {
                let s = session.lock().unwrap();
                session_info(&file_id, &s)
            };
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: Some(info),
                diff,
            }))
        }
        "replace_runs" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                para: usize,
                runs: Vec<rhwp::document_core::RunSpec>,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            let op = rhwp::document_core::EditOperation::ReplaceRuns {
                section: payload.section,
                para: payload.para,
                runs: payload.runs,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "set_paragraph_style" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                para: usize,
                style: rhwp::document_core::PartialParagraphStyle,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            let op = rhwp::document_core::EditOperation::SetParagraphStyle {
                section: payload.section,
                para: payload.para,
                style: payload.style,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "delete_range" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                para_start: usize,
                char_start: usize,
                para_end: usize,
                char_end: usize,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            let op = rhwp::document_core::EditOperation::DeleteRange {
                section: payload.section,
                para_start: payload.para_start,
                char_start: payload.char_start,
                para_end: payload.para_end,
                char_end: payload.char_end,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "insert_paragraph" => {
            fn one() -> usize { 1 }
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                after_para: usize,
                #[serde(default = "one")]
                count: usize,
                #[serde(default)]
                style: Option<rhwp::document_core::PartialParagraphStyle>,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            let op = rhwp::document_core::EditOperation::InsertParagraph {
                section: payload.section,
                after_para: payload.after_para,
                count: payload.count,
                style: payload.style,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "insert_page_break" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                para: usize,
                offset: usize,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            let op = rhwp::document_core::EditOperation::InsertPageBreak {
                section: payload.section,
                para: payload.para,
                offset: payload.offset,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "delete_element" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                para: usize,
                element_type: rhwp::document_core::ElementType,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            let op = rhwp::document_core::EditOperation::DeleteElement {
                section: payload.section,
                para: payload.para,
                element_type: payload.element_type,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "insert_table" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                insert_after_para: usize,
                rows: u16,
                cols: u16,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            let op = rhwp::document_core::EditOperation::InsertTable {
                section: payload.section,
                insert_after_para: payload.insert_after_para,
                rows: payload.rows,
                cols: payload.cols,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "set_cell_style" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                table_para: usize,
                row: usize,
                col: usize,
                style: rhwp::document_core::PartialCellStyle,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            // [4-4 fix] cell_idx + ctrl_idx 미리 변환해 broadcast 페이로드에 포함 — 클라 재계산 제거.
            // ctrl_idx 는 paragraph 첫 Table 위치 (section_def/column_def 가 앞에 동거할 때 0 이 아님).
            let (cell_idx, ctrl_idx) = {
                let s = session.lock().unwrap();
                let ctrl = s.core
                    .find_table_ctrl_idx(payload.section, payload.table_para)
                    .map_err(|e| AppError::unprocessable(format!("find_table_ctrl_idx: {e}")))?;
                let cell = s.core
                    .find_cell_idx(payload.section, payload.table_para, ctrl, payload.row as u16, payload.col as u16)
                    .map_err(|e| AppError::unprocessable(format!("find_cell_idx: {e}")))?;
                (cell, ctrl)
            };
            let op = rhwp::document_core::EditOperation::SetCellStyle {
                section: payload.section,
                table_para: payload.table_para,
                row: payload.row,
                col: payload.col,
                cell_idx: Some(cell_idx),
                ctrl_idx: Some(ctrl_idx),
                style: payload.style,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "merge_cells" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                table_para: usize,
                row_start: usize,
                col_start: usize,
                row_end: usize,
                col_end: usize,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            let ctrl_idx = {
                let s = session.lock().unwrap();
                s.core
                    .find_table_ctrl_idx(payload.section, payload.table_para)
                    .map_err(|e| AppError::unprocessable(format!("find_table_ctrl_idx: {e}")))?
            };
            let op = rhwp::document_core::EditOperation::MergeCells {
                section: payload.section,
                table_para: payload.table_para,
                row_start: payload.row_start,
                col_start: payload.col_start,
                row_end: payload.row_end,
                col_end: payload.col_end,
                ctrl_idx: Some(ctrl_idx),
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "replace_cell_runs" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                table_para: usize,
                row: usize,
                col: usize,
                cell_para: usize,
                runs: Vec<rhwp::document_core::RunSpec>,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            // [4-4 fix] cell_idx + ctrl_idx 미리 변환해 broadcast 페이로드에 포함.
            let (cell_idx, ctrl_idx) = {
                let s = session.lock().unwrap();
                let ctrl = s.core
                    .find_table_ctrl_idx(payload.section, payload.table_para)
                    .map_err(|e| AppError::unprocessable(format!("find_table_ctrl_idx: {e}")))?;
                let cell = s.core
                    .find_cell_idx(payload.section, payload.table_para, ctrl, payload.row as u16, payload.col as u16)
                    .map_err(|e| AppError::unprocessable(format!("find_cell_idx: {e}")))?;
                (cell, ctrl)
            };
            let op = rhwp::document_core::EditOperation::ReplaceCellRuns {
                section: payload.section,
                table_para: payload.table_para,
                row: payload.row,
                col: payload.col,
                cell_idx: Some(cell_idx),
                ctrl_idx: Some(ctrl_idx),
                cell_para: payload.cell_para,
                runs: payload.runs,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "insert_text_in_cell" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                table_para: usize,
                row: usize,
                col: usize,
                cell_para: usize,
                offset: usize,
                text: String,
                #[serde(default)]
                style: Option<rhwp::document_core::PartialRunStyle>,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            // [4-4 fix] cell_idx + ctrl_idx 미리 변환해 broadcast 페이로드에 포함.
            let (cell_idx, ctrl_idx) = {
                let s = session.lock().unwrap();
                let ctrl = s.core
                    .find_table_ctrl_idx(payload.section, payload.table_para)
                    .map_err(|e| AppError::unprocessable(format!("find_table_ctrl_idx: {e}")))?;
                let cell = s.core
                    .find_cell_idx(payload.section, payload.table_para, ctrl, payload.row as u16, payload.col as u16)
                    .map_err(|e| AppError::unprocessable(format!("find_cell_idx: {e}")))?;
                (cell, ctrl)
            };
            let op = rhwp::document_core::EditOperation::InsertTextInCell {
                section: payload.section,
                table_para: payload.table_para,
                row: payload.row,
                col: payload.col,
                cell_idx: Some(cell_idx),
                ctrl_idx: Some(ctrl_idx),
                cell_para: payload.cell_para,
                offset: payload.offset,
                text: payload.text,
                style: payload.style,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "delete_range_in_cell" => {
            #[derive(serde::Deserialize)]
            struct Payload {
                section: usize,
                table_para: usize,
                row: usize,
                col: usize,
                cell_para_start: usize,
                char_start: usize,
                cell_para_end: usize,
                char_end: usize,
            }
            let payload: Payload = serde_json::from_value(req.payload.clone())
                .map_err(|e| AppError::bad_request(format!("INVALID_PAYLOAD: {e}")))?;
            // [4-4 fix] cell_idx + ctrl_idx 미리 변환해 broadcast 페이로드에 포함.
            let (cell_idx, ctrl_idx) = {
                let s = session.lock().unwrap();
                let ctrl = s.core
                    .find_table_ctrl_idx(payload.section, payload.table_para)
                    .map_err(|e| AppError::unprocessable(format!("find_table_ctrl_idx: {e}")))?;
                let cell = s.core
                    .find_cell_idx(payload.section, payload.table_para, ctrl, payload.row as u16, payload.col as u16)
                    .map_err(|e| AppError::unprocessable(format!("find_cell_idx: {e}")))?;
                (cell, ctrl)
            };
            let op = rhwp::document_core::EditOperation::DeleteRangeInCell {
                section: payload.section,
                table_para: payload.table_para,
                row: payload.row,
                col: payload.col,
                cell_idx: Some(cell_idx),
                ctrl_idx: Some(ctrl_idx),
                cell_para_start: payload.cell_para_start,
                char_start: payload.char_start,
                cell_para_end: payload.cell_para_end,
                char_end: payload.char_end,
            };
            let (seq, diff) = apply_op_with_stash(&state, &file_id, session.clone(), op, None).await?;
            Ok(Json(WorkbenchResp {
                seq,
                applied: "ops".to_string(),
                info: None,
                diff,
            }))
        }
        "complete" => {
            let blob = {
                let s = session.lock().unwrap();
                s.core
                    .export_hwpx_native()
                    .map_err(|e| AppError::internal(format!("export_hwpx: {e}")))?
            };
            let seq = {
                let mut s = session.lock().unwrap();
                let cur = s.next_seq;
                s.next_seq += 1;
                cur
            };
            state
                .store
                .save_final_snapshot(&file_id, seq, &blob)
                .map_err(|e| AppError::internal(format!("save_final_snapshot: {e}")))?;
            state
                .events
                .publish(&file_id, events::ServerEvent::Complete { seq });
            Ok(Json(WorkbenchResp {
                seq,
                applied: "complete".to_string(),
                info: None,
                diff: None,
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
                diff: None,
            }))
        }
    }
}

#[derive(Deserialize)]
struct AuditQuery {
    seq_from: i64,
    seq_to: i64,
}

#[derive(Serialize)]
struct AuditRow {
    seq: i64,
    op: serde_json::Value,
}

/// op_stash 범위 조회. seq_from..=seq_to 사이 op 들을 op_json 파싱하여 반환.
async fn audit_handler(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditRow>>, AppError> {
    let rows = state
        .store
        .list_op_stash_range(&file_id, q.seq_from, q.seq_to)
        .map_err(|e| AppError::internal(format!("list_op_stash_range: {e}")))?;

    let result: Vec<AuditRow> = rows
        .into_iter()
        .map(|r| {
            let op_value: serde_json::Value = serde_json::from_str(&r.op_json)
                .unwrap_or_else(|_| serde_json::Value::String(r.op_json.clone()));
            AuditRow {
                seq: r.seq,
                op: op_value,
            }
        })
        .collect();

    Ok(Json(result))
}

fn default_ir_slice_mode() -> String {
    "auto".to_string()
}

#[derive(Deserialize)]
struct IrSliceQuery {
    #[serde(default)]
    sec: Option<usize>,
    #[serde(default)]
    para_start: Option<usize>,
    #[serde(default)]
    para_end: Option<usize>,
    /// Sub-3 v2 — 문서 전체 0-based 페이지 번호. 지정 시 paginator 결과로
    /// sec/para_start/para_end 가 *덮어써짐*. raw 모드에는 영향 없음.
    #[serde(default)]
    page: Option<u32>,
    #[serde(default = "default_ir_slice_mode")]
    mode: String,
}

/// 섹션 일부 paragraph 만 IR JSON 으로 반환.
/// mode: "raw" — paragraph 의 상세 필드, "compact" — 텍스트+모양 id, "auto" — 5000자 미만 raw, 이상 compact.
/// 브라우저 (rhwp-studio WASM) 가 자기 paginate 결과를 *역공급* 하는 요청.
///
/// 본문: `{ "seq": <i64>, "total_pages": <u32>, "pages": [{page, sec, para_start, para_end}, ...] }`
/// — 이미 적용된 마지막 op seq 와 함께 페이지별 (sec, para_start, para_end) 매핑을 동봉.
///
/// `seq` 는 현재 staleness 판정에 *직접 쓰이지 않는다* — 측정기 격차 (native ↔ WASM) 가
/// 통상 한두 단락 어긋남이라 *살짝 stale 한 map* 이라도 native paginator 보다 항상 가깝다.
/// 멀티 클라이언트 / 편집 race 가 본격 문제가 되면 그 때 seq 비교 가드를 도입한다.
#[derive(Deserialize)]
struct PageMapReq {
    seq: i64,
    total_pages: u32,
    pages: Vec<ClientPageEntry>,
}

async fn put_page_map(
    State(_state): State<AppState>,
    Path(_file_id): Path<String>,
    Json(_req): Json<PageMapReq>,
) -> StatusCode {
    // [Plan A] page_maps 폐기 — wasm 가 보내는 매핑을 *보관하지 않는다*.
    // ir-slice 가 server 자기 paginate 결과만 사용하므로 매핑 저장이 무용.
    // 호환성을 위해 endpoint 와 응답 코드는 유지 (wasm 측 fetch 실패 회피).
    // 종전 race: wasm 의 paginate 진행 중 부분 매핑이 600ms debounce 로 POST 되어
    // server 에 영구 박힘 → ir-slice 가 *잘린 IR* 반환 → 모델이 paragraph 일부만 받음.
    StatusCode::NO_CONTENT
}

async fn ir_slice_handler(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(q): Query<IrSliceQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let session = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(&file_id)
            .ok_or_else(|| AppError::not_found(format!("세션 없음: {file_id}")))?
            .clone()
    };
    let s = session.lock().unwrap();

    let sec = q.sec.unwrap_or(0);
    let total = s
        .core
        .document()
        .sections
        .get(sec)
        .ok_or_else(|| AppError::bad_request(format!("section {} 없음", sec)))?
        .paragraphs
        .len();
    let para_start = q.para_start.unwrap_or(0);
    let para_end = q.para_end.unwrap_or(total).min(total);

    let resolved_mode = match q.mode.as_str() {
        "raw" => "raw",
        _ => "compact",
    };

    if resolved_mode == "compact" {
        // [Plan A] page_maps (브라우저 wasm 역공급 매핑) 폐기 — 항상 server 자기 paginate
        // 결과만 단일 진실 소스로 사용한다.
        //
        // 종전: 측정기 격차 우회용으로 wasm 매핑을 우선 사용. 다만 wasm 의 paginate 가
        // *진행 중 부분 결과*를 600ms debounce 로 POST 하는 race 가 있어 *부분 매핑* 이
        // server 에 영구 박힐 위험이 있었다. 한 paragraph 가 매핑에서 누락되면 ir-slice
        // 응답이 *잘려서* 모델이 paragraph 일부만 받는 사고가 일어났다.
        //
        // 현재 [text_measurement.rs PR #1026] 이후 wasm 측정도 *내장 메트릭 + 휴리스틱*
        // 만 사용 (Canvas measureText 호출 자리 0건) — native EmbeddedTextMeasurer 와
        // 사실상 동일. 측정기 격차가 사실상 사라졌으므로 page_maps 우회가 불필요.
        let (page_override_range, total_pages_override): (Option<(usize, usize, usize)>, Option<u32>) =
            (None, None);
        let opts = ir_compact::BuildOptions {
            sec,
            para_start,
            para_end: Some(para_end),
            edit_session_id: Some(format!("cli_{}", file_id)),
            // Sub-3 v2 — page query 지정 시 paginator 결과로 sec/start/end 가 덮어써짐.
            // m400 sub-2 — page 인자 1-based 정합. 사용자·모델 직관 (1 페이지 = page 1) 정합.
            // page = 1 → 첫 페이지 (내부 0-based 인덱스 0). page = 0 또는 미지정 → 전체.
            page: q.page.and_then(|p| if p >= 1 { Some(p - 1) } else { None }),
            page_override_range,
            total_pages_override,
        };
        let slice = ir_compact::build_compact_ir_slice(&s.core, &opts);
        // anchor 값은 page 매핑 후의 *실제* sec/para_start/para_end — top-level 호환 필드도
        // 이 값으로 채워야 옛 client 가 일관되게 인식. move 전에 사본을 떠둔다.
        let anchor_sec = slice.doc_meta.anchor.sec;
        let anchor_start = slice.doc_meta.anchor.para_start;
        let anchor_end = slice.doc_meta.anchor.para_end;
        let mut v = serde_json::to_value(&slice).unwrap_or(serde_json::Value::Null);
        if let serde_json::Value::Object(ref mut m) = v {
            m.insert("section".into(), serde_json::json!(anchor_sec));
            m.insert("para_start".into(), serde_json::json!(anchor_start));
            m.insert("para_end".into(), serde_json::json!(anchor_end));
            m.insert("mode".into(), serde_json::json!("compact"));
        }
        return Ok(Json(v));
    }

    let paragraphs: Vec<serde_json::Value> = (para_start..para_end)
        .map(|p| {
            let para = &s.core.document().sections[sec].paragraphs[p];
            // Paragraph 의 Serialize derive 로 직접 직렬화. `controls`,
            // `ctrl_data_records` 는 #[serde(skip)] 되어 제외 — Control enum
            // 이 Serialize 미구현이라 raw 에서 빠진다. 컨트롤 목록은 별도로
            // /sessions/{id}/ir 의 ParagraphView::controls 로 조회.
            let mut v = serde_json::to_value(para).unwrap_or(serde_json::Value::Null);
            if let serde_json::Value::Object(ref mut map) = v {
                map.insert("para".into(), serde_json::Value::from(p));
                // 컨트롤은 raw 직렬화에서 빠지므로 길이만 보강.
                map.insert(
                    "controls_len".into(),
                    serde_json::Value::from(para.controls.len()),
                );
            }
            v
        })
        .collect();

    Ok(Json(serde_json::json!({
        "section": sec,
        "para_start": para_start,
        "para_end": para_end,
        "mode": resolved_mode,
        "paragraphs": paragraphs,
    })))
}

#[derive(Deserialize)]
struct DiffQuery {
    seq: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiffResponse {
    seq: i64,
    op: serde_json::Value,
    before_paragraphs: Vec<String>,
    after_paragraphs: Vec<String>,
    chars_added: i64,
    chars_removed: i64,
}

/// 지정한 seq 의 before/after blob 두 개를 임시 코어로 비교한다.
/// after blob 은 다음 seq 의 before_blob 또는 (다음이 없으면) 현재 세션 상태.
async fn diff_handler(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(q): Query<DiffQuery>,
) -> Result<Json<DiffResponse>, AppError> {
    let target = state
        .store
        .get_op_stash_by_seq(&file_id, q.seq)
        .map_err(|e| AppError::internal(format!("get_op_stash_by_seq: {e}")))?
        .ok_or_else(|| AppError::not_found(format!("seq {} op_stash 없음", q.seq)))?;

    let before_core = rhwp::document_core::DocumentCore::from_bytes(&target.before_blob)
        .map_err(|e| AppError::internal(format!("before from_bytes: {e}")))?;

    let after_blob = match state
        .store
        .get_op_stash_by_seq(&file_id, q.seq + 1)
        .map_err(|e| AppError::internal(format!("get next: {e}")))?
    {
        Some(next) => next.before_blob,
        None => {
            let session = {
                let sessions = state.sessions.lock().unwrap();
                sessions
                    .get(&file_id)
                    .ok_or_else(|| AppError::not_found(format!("세션 없음: {file_id}")))?
                    .clone()
            };
            let s = session.lock().unwrap();
            s.core
                .export_hwpx_native()
                .map_err(|e| AppError::internal(format!("export after: {e}")))?
        }
    };
    let after_core = rhwp::document_core::DocumentCore::from_bytes(&after_blob)
        .map_err(|e| AppError::internal(format!("after from_bytes: {e}")))?;

    let before_paragraphs: Vec<String> = before_core.document().sections[0]
        .paragraphs
        .iter()
        .map(|p| p.text.clone())
        .collect();
    let after_paragraphs: Vec<String> = after_core.document().sections[0]
        .paragraphs
        .iter()
        .map(|p| p.text.clone())
        .collect();

    let before_total: usize = before_paragraphs.iter().map(|s| s.chars().count()).sum();
    let after_total: usize = after_paragraphs.iter().map(|s| s.chars().count()).sum();
    let chars_added = (after_total as i64 - before_total as i64).max(0);
    let chars_removed = (before_total as i64 - after_total as i64).max(0);

    let op_value: serde_json::Value =
        serde_json::from_str(&target.op_json).unwrap_or(serde_json::Value::Null);

    Ok(Json(DiffResponse {
        seq: q.seq,
        op: op_value,
        before_paragraphs,
        after_paragraphs,
        chars_added,
        chars_removed,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UndoResponse {
    seq_reverted: i64,
    applied: &'static str,
}

/// 가장 최근 op_stash 항목을 pop 하여 before_blob 으로 세션 코어를 복원한다.
/// 빈 stash 면 409 NO_UNDO_AVAILABLE.
async fn undo_handler(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<Json<UndoResponse>, AppError> {
    let session = {
        let sessions = state.sessions.lock().unwrap();
        sessions
            .get(&file_id)
            .ok_or_else(|| AppError::not_found(format!("세션 없음: {file_id}")))?
            .clone()
    };

    let row = state
        .store
        .pop_op_stash(&file_id)
        .map_err(|e| AppError::internal(format!("pop_op_stash: {e}")))?
        .ok_or_else(|| AppError::conflict("NO_UNDO_AVAILABLE"))?;

    let new_core = rhwp::document_core::DocumentCore::from_bytes(&row.before_blob)
        .map_err(|e| AppError::internal(format!("from_bytes: {e}")))?;

    let seq = {
        let mut s = session.lock().unwrap();
        s.core = new_core;
        let cur = s.next_seq;
        s.next_seq += 1;
        cur
    };

    let snapshot_base64 = STANDARD.encode(&row.before_blob);
    state.events.publish(
        &file_id,
        events::ServerEvent::SnapshotRestored {
            seq,
            snapshot_base64,
        },
    );

    Ok(Json(UndoResponse {
        seq_reverted: row.seq,
        applied: "undo",
    }))
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
        .route("/documents/blank", post(create_blank_document))
        .route("/sessions/:id/ops", post(apply_ops))
        .route("/sessions/:id/snapshot", put(put_snapshot))
        .route("/sessions/:id/ir", get(get_ir))
        .route("/sessions/:id/export", get(export))
        .route("/sessions/:id/save", post(save_document))
        .route("/sessions/:id/save-as", post(save_as))
        .route("/sessions/:id/workbench", post(workbench))
        .route("/sessions/:id/undo", post(undo_handler))
        .route("/sessions/:id/audit", get(audit_handler))
        .route("/sessions/:id/diff", get(diff_handler))
        .route("/sessions/:id/ir-slice", get(ir_slice_handler))
        .route("/sessions/:id/page-map", post(put_page_map))
        .route("/sessions/:id/ws", get(ws::ws_upgrade))
        .route("/sessions/:id", delete(delete_session))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // RHWP_STUDIO_DIR 가 지정되면 studio 정적 자산(dist)도 같은 포트에서 서빙한다.
    // → single-origin 배포(별도 웹서버/CORS 불필요). 미지정 시 API 전용.
    //
    // SPA deep-link 복원 — 새로고침 시 `?fileId=...` 가 정적 파일로 매칭되지 않아도
    // index.html 로 폴백해 클라이언트 라우팅이 처리하도록 ServeDir.fallback(ServeFile).
    let index_path = std::env::var("RHWP_STUDIO_DIR").ok().and_then(|dir| {
        if dir.is_empty() {
            None
        } else {
            let p = std::path::PathBuf::from(&dir).join("index.html");
            Some((dir, p))
        }
    });
    if let Some((dir, idx)) = index_path.clone() {
        tracing::info!("studio 정적 서빙: {dir}");
        app = app.fallback_service(
            tower_http::services::ServeDir::new(dir)
                .append_index_html_on_directories(true)
                .fallback(tower_http::services::ServeFile::new(idx)),
        );
    }

    // ── /hwp prefix 일괄 적용 ──
    // 모든 경로(API + 정적 자산)가 `/hwp/` 아래로 들어간다. iframe / 모델 호출 / 헬스체크
    // 모두 prefix 명시 — 단일 진입점.
    //
    // nest 사각지대: axum 0.7 의 `nest("/hwp")` 는 `/hwp`(exact) 와 `/hwp/{*rest}`(≥1 세그먼트)
    // 만 매칭해 정확히 `/hwp/` (trailing slash) 가 빠진다. iframe 진입이 보통 `/hwp/` 형태라
    // index.html 을 trailing-slash 경로에 명시 라우팅한다.
    let mut root = Router::new().nest("/hwp", app);
    if let Some((_, idx)) = index_path {
        root = root.route_service("/hwp/", tower_http::services::ServeFile::new(idx));
    }
    root
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
        page_maps: Arc::new(Mutex::new(HashMap::new())),
    };

    let addr = std::env::var("RHWP_SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:7710".to_string());
    let listener = TcpListener::bind(&addr).await.expect("bind 실패");

    // iframe 부모창 origin 화이트리스트 → CSP frame-ancestors 디렉티브.
    // 우선순위: RHWP_FRAME_ANCESTORS (raw, BC) > RHWP_ALLOWED_PARENT_ORIGIN (CSV/공백 분리)
    // 비어 있으면 layer 미적용 (= 기존 permissive 동작 유지).
    let frame_ancestors = resolve_frame_ancestors();
    tracing::info!(
        "rhwp-server listening on {addr} (db={db_path}, frame_ancestors={:?})",
        frame_ancestors
    );

    let mut svc = router(state);
    if let Some(fa) = frame_ancestors {
        let csp = format!("frame-ancestors {fa}");
        if let Ok(value) = axum::http::HeaderValue::from_str(&csp) {
            svc = svc.layer(SetResponseHeaderLayer::if_not_present(
                axum::http::header::CONTENT_SECURITY_POLICY,
                value,
            ));
        }
    }

    axum::serve(listener, svc).await.expect("서버 종료됨");
}

/// `RHWP_ALLOWED_PARENT_ORIGIN` (콤마/공백 분리 CSV) 또는 `RHWP_FRAME_ANCESTORS` (raw CSP)
/// 를 읽어 CSP `frame-ancestors` 디렉티브로 환산한다. rcode/vfinder 와 동등 패턴.
fn resolve_frame_ancestors() -> Option<String> {
    if let Ok(raw) = std::env::var("RHWP_FRAME_ANCESTORS") {
        let t = raw.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if let Ok(list) = std::env::var("RHWP_ALLOWED_PARENT_ORIGIN") {
        let tokens: Vec<&str> = list
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if !tokens.is_empty() {
            return Some(tokens.join(" "));
        }
    }
    None
}
