//! 외부 파일 저장소(minio) 클라이언트.
//!
//! rhwp-server가 프록시로서 minio upload/download API를 호출한다.
//! - upload: 파일 바이트(multipart `file`) → `{ "file_id": ... }`
//! - download: `GET {DOWNLOAD_URL}` (`{file_id}` 치환) → 파일 바이트
//!
//! 설정: 환경변수 `UPLOAD_URL`, `DOWNLOAD_URL`(`{file_id}` placeholder 포함).
//! 둘 중 하나라도 비어 있으면 해당 기능은 비활성(`enabled()`=false).

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

    /// 파일을 업로드하고 발급된 file_id를 반환한다.
    pub async fn upload(&self, bytes: Vec<u8>, filename: &str) -> Result<String, String> {
        if self.upload_url.is_empty() {
            return Err("UPLOAD_URL 미설정".to_string());
        }
        let part = reqwest::multipart::Part::bytes(bytes).file_name(filename.to_string());
        let form = reqwest::multipart::Form::new().part("file", part);
        let resp = self
            .client
            .post(&self.upload_url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("upload 요청 실패: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("upload 응답 상태 {}", resp.status()));
        }
        let j: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("upload 응답 파싱 실패: {e}"))?;
        j.get("file_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "upload 응답에 file_id 없음".to_string())
    }

    /// file_id로 파일을 다운로드한다.
    pub async fn download(&self, file_id: &str) -> Result<Vec<u8>, String> {
        if self.download_url.is_empty() {
            return Err("DOWNLOAD_URL 미설정".to_string());
        }
        let url = self.download_url.replace("{file_id}", file_id);
        let resp = self
            .client
            .get(&url)
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
