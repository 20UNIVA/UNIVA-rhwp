//! 외부 파일 저장소 클라이언트 — minio 또는 vfinder 와 호환.
//!
//! rhwp-server가 프록시로서 외부 upload/download API를 호출한다.
//! - upload: 파일 바이트(multipart `file`) → `{ "file_id": ... }`
//! - download: `GET {DOWNLOAD_URL}` (`{file_id}` 치환) → 파일 바이트
//!
//! 설정: 환경변수 `UPLOAD_URL`, `DOWNLOAD_URL`(`{file_id}` placeholder 포함).
//! 둘 중 하나라도 비어 있으면 해당 기능은 비활성(`enabled()`=false).
//!
//! ## 외부 저장소별 약속
//!
//! - **minio** (기존): form 의 `file_id` text 만 본다. 쿼리·헤더는 무시.
//! - **vfinder** (`docs/13-rhwp-vfinder-storage-integration.md`):
//!   - `?file_id=<id>` 쿼리 → 그 id 의 path 자리 덮어쓰기 (form text 의 id 는 무시).
//!   - `?path=<folder>` 쿼리 → 신규 — 그 폴더에 저장.
//!   - `X-Vfinder-User` 헤더 → user 식별. 누락 시 거절.
//!
//! 같은 클라이언트가 두 약속을 모두 만족하도록 — *쿼리·form·헤더 셋 다* 박는다.
//! minio 는 쿼리·헤더를 무시하므로 호환, vfinder 는 쿼리·헤더를 보므로 정합.

/// upload 응답.
pub struct UploadResult {
    pub file_id: String,
    /// 실 저장 경로. minio 응답의 `minio_key` 또는 vfinder 응답의 `path` 자리 — 둘 다
    /// *외부 저장소 안의 자리* 라는 같은 의미를 가져 한 필드로 통합한다.
    pub minio_key: Option<String>,
    /// 기존 file_id 덮어쓰기였으면 true.
    pub updated: bool,
}

pub struct Storage {
    client: reqwest::Client,
    upload_url: String,
    download_url: String,
}

impl Storage {
    pub fn from_env() -> Self {
        Storage {
            client: reqwest::Client::new(),
            upload_url: std::env::var("UPLOAD_URL").unwrap_or_default(),
            download_url: std::env::var("DOWNLOAD_URL").unwrap_or_default(),
        }
    }

    /// upload/download 설정이 모두 갖춰졌는지.
    pub fn enabled(&self) -> bool {
        !self.upload_url.is_empty() && !self.download_url.is_empty()
    }

    /// 파일을 업로드한다.
    ///
    /// 모드 — 인자 조합으로 갈림:
    /// - `file_id = Some(id)` : 기존 자리 *덮어쓰기*. id 가 없으면 서버가 404.
    /// - `file_id = None`, `target_path = Some(path)` : 신규 — 그 폴더에 저장.
    /// - `file_id = None`, `target_path = None` : 신규 — vfinder 의 기본 폴더 또는
    ///   minio 의 기본 자리 (서버 측 정책에 위임).
    ///
    /// `user_id` 는 vfinder 의 `X-Vfinder-User` 헤더로 박는다. minio 는 무시.
    pub async fn upload(
        &self,
        bytes: Vec<u8>,
        filename: &str,
        file_id: Option<&str>,
        target_path: Option<&str>,
        overwrite: bool,
        user_id: &str,
    ) -> Result<UploadResult, String> {
        if self.upload_url.is_empty() {
            return Err("UPLOAD_URL 미설정".to_string());
        }

        // 쿼리 자리 — file_id, target_path, overwrite 중 박힌 자리만 추가.
        let url = compose_upload_url(&self.upload_url, file_id, target_path, overwrite);

        let part = reqwest::multipart::Part::bytes(bytes).file_name(filename.to_string());
        let mut form = reqwest::multipart::Form::new().part("file", part);
        // form text 의 file_id 는 *minio 호환* 자리. vfinder 는 쿼리만 본다.
        if let Some(id) = file_id {
            form = form.text("file_id", id.to_string());
        }

        let resp = self
            .client
            .post(&url)
            .header("X-Vfinder-User", user_id)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("upload 요청 실패: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            // 404 = 덮어쓰기 대상 file_id 없음
            return Err(format!("upload 응답 상태 {status}"));
        }
        let j: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("upload 응답 파싱 실패: {e}"))?;
        let fid = j
            .get("file_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "upload 응답에 file_id 없음".to_string())?;
        // `minio_key` (minio) 또는 `path` (vfinder) 어느 쪽이 박혀 와도 같은 자리.
        let stored = j
            .get("minio_key")
            .or_else(|| j.get("path"))
            .and_then(|v| v.as_str())
            .map(String::from);
        Ok(UploadResult {
            file_id: fid,
            minio_key: stored,
            updated: j.get("updated").and_then(|v| v.as_bool()).unwrap_or(false),
        })
    }

    /// file_id 로 파일을 다운로드한다. user_id 는 vfinder 의 `X-Vfinder-User` 헤더.
    pub async fn download(&self, file_id: &str, user_id: &str) -> Result<Vec<u8>, String> {
        if self.download_url.is_empty() {
            return Err("DOWNLOAD_URL 미설정".to_string());
        }
        let url = self.download_url.replace("{file_id}", file_id);
        let resp = self
            .client
            .get(&url)
            .header("X-Vfinder-User", user_id)
            .send()
            .await
            .map_err(|e| format!("download 요청 실패: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("download 응답 상태 {}", resp.status()));
        }
        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("download 본문 읽기 실패: {e}"))
    }
}

/// `base?file_id=...&path=...&overwrite=true` 형태로 쿼리 자리 조립.
/// base 에 이미 `?` 가 박혀 있으면 `&` 로 이어 붙임.
fn compose_upload_url(
    base: &str,
    file_id: Option<&str>,
    target_path: Option<&str>,
    overwrite: bool,
) -> String {
    let mut params: Vec<(String, String)> = Vec::new();
    if let Some(id) = file_id {
        params.push(("file_id".to_string(), id.to_string()));
    }
    if let Some(p) = target_path {
        params.push(("path".to_string(), p.to_string()));
    }
    if overwrite {
        params.push(("overwrite".to_string(), "true".to_string()));
    }
    if params.is_empty() {
        return base.to_string();
    }
    let sep = if base.contains('?') { '&' } else { '?' };
    let qs = params
        .into_iter()
        .map(|(k, v)| format!("{k}={}", url_encode(&v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{base}{sep}{qs}")
}

/// 최소 percent-encoding — 영숫자·`-._~/` 외 모두 %xx.
/// `/` 는 path 자리에서 의미 보존이라 그대로 둔다.
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_upload_url_appends_query() {
        let base = "http://host/upload";
        assert_eq!(compose_upload_url(base, None, None, false), "http://host/upload");
        assert_eq!(
            compose_upload_url(base, Some("abc"), None, false),
            "http://host/upload?file_id=abc"
        );
        assert_eq!(
            compose_upload_url(base, None, Some("/folder/sub"), false),
            "http://host/upload?path=/folder/sub"
        );
        assert_eq!(
            compose_upload_url(base, Some("abc"), Some("/folder"), false),
            "http://host/upload?file_id=abc&path=/folder"
        );
        assert_eq!(
            compose_upload_url(base, None, Some("/folder"), true),
            "http://host/upload?path=/folder&overwrite=true"
        );
    }

    #[test]
    fn compose_upload_url_existing_query_uses_amp() {
        let base = "http://host/upload?token=x";
        assert_eq!(
            compose_upload_url(base, Some("abc"), None, false),
            "http://host/upload?token=x&file_id=abc"
        );
    }

    #[test]
    fn url_encode_keeps_slash_and_basics() {
        assert_eq!(url_encode("/내 작업/"), "/%EB%82%B4%20%EC%9E%91%EC%97%85/");
    }
}
