//! sqlite 기반 세션 영속 저장소.
//!
//! 작업 중 세션 상태(VM 로컬)를 sqlite에 보관하여 서버 재시작 후에도 복원한다.
//! - `sessions`  : 세션별 원본 문서(base) + 포맷
//! - `ops`       : 적용된 EditOperation 로그 (seq 순)
//! - `snapshots` : 스냅샷형 동기화 시점의 전체 문서 (seq 순)
//!
//! 복원은 "가장 최근 snapshot(없으면 base) + 그 이후 ops 재적용"으로 수행한다.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

/// 복원용으로 적재된 세션 데이터.
pub struct PersistedSession {
    pub format: String,
    /// 복원 기준 문서 바이트 (최신 snapshot 또는 base).
    pub base_blob: Vec<u8>,
    /// `base_blob` 이후 재적용할 op (seq, op_json).
    pub ops: Vec<(i64, String)>,
    /// 마지막으로 사용된 seq (다음 seq = last_seq + 1).
    pub last_seq: i64,
}

pub struct Store {
    conn: Mutex<Connection>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OpStashRow {
    pub seq: i64,
    pub op_json: String,
    pub before_blob: Vec<u8>,
}

impl Store {
    /// sqlite 파일을 열고 스키마를 보장한다. `:memory:` 도 허용.
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                file_id   TEXT PRIMARY KEY,
                format    TEXT NOT NULL,
                base_blob BLOB NOT NULL,
                created   INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS ops (
                file_id TEXT NOT NULL,
                seq     INTEGER NOT NULL,
                op_json TEXT NOT NULL,
                PRIMARY KEY (file_id, seq)
             );
             CREATE TABLE IF NOT EXISTS snapshots (
                file_id TEXT NOT NULL,
                seq     INTEGER NOT NULL,
                blob    BLOB NOT NULL,
                PRIMARY KEY (file_id, seq)
             );
             CREATE TABLE IF NOT EXISTS op_stash (
                seq         INTEGER NOT NULL,
                file_id     TEXT NOT NULL,
                op_json     TEXT NOT NULL,
                before_blob BLOB NOT NULL,
                created_at  INTEGER NOT NULL,
                PRIMARY KEY (file_id, seq)
             );
             CREATE INDEX IF NOT EXISTS idx_op_stash_file_seq ON op_stash(file_id, seq);
             CREATE TABLE IF NOT EXISTS final_snapshots (
                file_id    TEXT PRIMARY KEY,
                seq        INTEGER NOT NULL,
                blob       BLOB NOT NULL,
                created_at INTEGER NOT NULL
             );",
        )?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    /// 세션을 생성(또는 재생성)한다. 기존 ops/snapshots는 초기화된다.
    pub fn create_session(
        &self,
        file_id: &str,
        format: &str,
        base_blob: &[u8],
    ) -> rusqlite::Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sessions (file_id, format, base_blob, created)
             VALUES (?1, ?2, ?3, ?4)",
            params![file_id, format, base_blob, now],
        )?;
        conn.execute("DELETE FROM ops WHERE file_id = ?1", params![file_id])?;
        conn.execute("DELETE FROM snapshots WHERE file_id = ?1", params![file_id])?;
        Ok(())
    }

    /// op 로그를 추가한다.
    pub fn append_op(&self, file_id: &str, seq: i64, op_json: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO ops (file_id, seq, op_json) VALUES (?1, ?2, ?3)",
            params![file_id, seq, op_json],
        )?;
        Ok(())
    }

    /// 스냅샷을 추가한다.
    pub fn append_snapshot(&self, file_id: &str, seq: i64, blob: &[u8]) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO snapshots (file_id, seq, blob) VALUES (?1, ?2, ?3)",
            params![file_id, seq, blob],
        )?;
        Ok(())
    }

    pub fn append_op_stash(
        &self,
        file_id: &str,
        seq: i64,
        op_json: &str,
        before_blob: &[u8],
    ) -> rusqlite::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO op_stash(seq, file_id, op_json, before_blob, created_at) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![seq, file_id, op_json, before_blob, now],
        )?;
        // 세션당 100 entry 정책 — 초과 row 제거
        conn.execute(
            "DELETE FROM op_stash WHERE file_id = ?1 AND seq IN (
                SELECT seq FROM op_stash WHERE file_id = ?1 ORDER BY seq DESC LIMIT -1 OFFSET 100
             )",
            rusqlite::params![file_id],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn pop_op_stash(&self, file_id: &str) -> rusqlite::Result<Option<OpStashRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT seq, op_json, before_blob FROM op_stash WHERE file_id = ?1 ORDER BY seq DESC LIMIT 1",
        )?;
        let row_opt = stmt
            .query_row(rusqlite::params![file_id], |row| {
                Ok(OpStashRow {
                    seq: row.get(0)?,
                    op_json: row.get(1)?,
                    before_blob: row.get(2)?,
                })
            })
            .optional()?;

        if let Some(row) = &row_opt {
            conn.execute(
                "DELETE FROM op_stash WHERE file_id = ?1 AND seq = ?2",
                rusqlite::params![file_id, row.seq],
            )?;
        }
        Ok(row_opt)
    }

    #[allow(dead_code)]
    pub fn count_op_stash(&self, file_id: &str) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM op_stash WHERE file_id = ?1",
            rusqlite::params![file_id],
            |row| row.get(0),
        )
    }

    #[allow(dead_code)]
    pub fn list_op_stash_range(
        &self,
        file_id: &str,
        seq_from: i64,
        seq_to: i64,
    ) -> rusqlite::Result<Vec<OpStashRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT seq, op_json, before_blob FROM op_stash WHERE file_id = ?1 AND seq BETWEEN ?2 AND ?3 ORDER BY seq ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![file_id, seq_from, seq_to], |row| {
            Ok(OpStashRow {
                seq: row.get(0)?,
                op_json: row.get(1)?,
                before_blob: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    #[allow(dead_code)]
    pub fn get_op_stash_by_seq(
        &self,
        file_id: &str,
        seq: i64,
    ) -> rusqlite::Result<Option<OpStashRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT seq, op_json, before_blob FROM op_stash WHERE file_id = ?1 AND seq = ?2",
        )?;
        stmt.query_row(rusqlite::params![file_id, seq], |row| {
            Ok(OpStashRow {
                seq: row.get(0)?,
                op_json: row.get(1)?,
                before_blob: row.get(2)?,
            })
        })
        .optional()
    }

    #[allow(dead_code)]
    pub fn save_final_snapshot(
        &self,
        file_id: &str,
        seq: i64,
        blob: &[u8],
    ) -> rusqlite::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO final_snapshots(file_id, seq, blob, created_at) VALUES (?, ?, ?, ?)",
            rusqlite::params![file_id, seq, blob, now],
        )?;
        Ok(())
    }

    /// 세션 존재 여부.
    #[allow(dead_code)]
    pub fn exists(&self, file_id: &str) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE file_id = ?1",
            params![file_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// 복원용 데이터를 적재한다. 세션이 없으면 `None`.
    pub fn load(&self, file_id: &str) -> rusqlite::Result<Option<PersistedSession>> {
        let conn = self.conn.lock().unwrap();

        let row: Option<(String, Vec<u8>)> = conn
            .query_row(
                "SELECT format, base_blob FROM sessions WHERE file_id = ?1",
                params![file_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (format, base_blob) = match row {
            Some(v) => v,
            None => return Ok(None),
        };

        // 가장 최근 snapshot (있으면 복원 기준점으로 사용).
        let snapshot: Option<(i64, Vec<u8>)> = conn
            .query_row(
                "SELECT seq, blob FROM snapshots WHERE file_id = ?1 ORDER BY seq DESC LIMIT 1",
                params![file_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;

        let (base_seq, base) = match snapshot {
            Some((seq, blob)) => (seq, blob),
            None => (0, base_blob),
        };

        // base_seq 이후의 op만 재적용 대상.
        let mut stmt = conn.prepare(
            "SELECT seq, op_json FROM ops WHERE file_id = ?1 AND seq > ?2 ORDER BY seq ASC",
        )?;
        let ops: Vec<(i64, String)> = stmt
            .query_map(params![file_id, base_seq], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;

        let last_op_seq = ops.last().map(|(s, _)| *s).unwrap_or(base_seq);
        let last_seq = base_seq.max(last_op_seq);

        Ok(Some(PersistedSession {
            format,
            base_blob: base,
            ops,
            last_seq,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_load() {
        let store = Store::open(":memory:").unwrap();
        store.create_session("f1", "hwpx", b"BASE").unwrap();
        store.append_op("f1", 1, r#"{"op":"insert_text"}"#).unwrap();
        store.append_op("f1", 2, r#"{"op":"delete_text"}"#).unwrap();

        assert!(store.exists("f1").unwrap());
        let p = store.load("f1").unwrap().unwrap();
        assert_eq!(p.format, "hwpx");
        assert_eq!(p.base_blob, b"BASE");
        assert_eq!(p.ops.len(), 2);
        assert_eq!(p.last_seq, 2);
    }

    #[test]
    fn test_snapshot_supersedes_base() {
        let store = Store::open(":memory:").unwrap();
        store.create_session("f1", "hwp", b"BASE").unwrap();
        store.append_op("f1", 1, "{}").unwrap();
        store.append_snapshot("f1", 2, b"SNAP").unwrap();
        store.append_op("f1", 3, "{}").unwrap();

        let p = store.load("f1").unwrap().unwrap();
        // snapshot(seq=2) 이후로 복원 → base는 SNAP, op는 seq=3 하나만.
        assert_eq!(p.base_blob, b"SNAP");
        assert_eq!(p.ops.len(), 1);
        assert_eq!(p.ops[0].0, 3);
        assert_eq!(p.last_seq, 3);
    }

    #[test]
    fn test_load_missing() {
        let store = Store::open(":memory:").unwrap();
        assert!(store.load("nope").unwrap().is_none());
    }

    #[test]
    fn test_op_stash_append_and_pop() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("x.db").to_str().unwrap()).unwrap();
        store.create_session("file-1", "hwpx", &[]).unwrap();

        store
            .append_op_stash("file-1", 1, r#"{"op":"insert_text"}"#, b"BLOBA")
            .unwrap();
        store
            .append_op_stash("file-1", 2, r#"{"op":"insert_text"}"#, b"BLOBB")
            .unwrap();

        let popped = store.pop_op_stash("file-1").unwrap().unwrap();
        assert_eq!(popped.seq, 2);
        assert_eq!(popped.before_blob, b"BLOBB");

        let popped2 = store.pop_op_stash("file-1").unwrap().unwrap();
        assert_eq!(popped2.seq, 1);

        let empty = store.pop_op_stash("file-1").unwrap();
        assert!(empty.is_none());
    }

    #[test]
    fn test_op_stash_100_entry_limit_per_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("x.db").to_str().unwrap()).unwrap();
        store.create_session("file-A", "hwpx", &[]).unwrap();
        for i in 1..=105 {
            store.append_op_stash("file-A", i, r#"{}"#, &[]).unwrap();
        }
        let count = store.count_op_stash("file-A").unwrap();
        assert_eq!(count, 100, "세션당 마지막 100개만 보관");
    }

    #[test]
    fn test_op_stash_list_range() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("x.db").to_str().unwrap()).unwrap();
        store.create_session("file-2", "hwpx", &[]).unwrap();
        for i in 1..=10 {
            let op_json = format!(r#"{{"op":"test","seq":{}}}"#, i);
            store.append_op_stash("file-2", i, &op_json, &[]).unwrap();
        }
        let rows = store.list_op_stash_range("file-2", 3, 7).unwrap();
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].seq, 3);
        assert_eq!(rows[4].seq, 7);
    }
}
