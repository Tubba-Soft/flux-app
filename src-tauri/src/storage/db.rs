use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use rusqlite::{params, Connection, Result as SqlResult};
use log::{info, warn, error};

use crate::models::{AppRule, StreamRule};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn init() -> Result<Self, String> {
        let db_path = match dirs_or_local() {
            Some(mut dir) => {
                let _ = std::fs::create_dir_all(&dir);
                dir.push("flux.db");
                dir
            }
            None => PathBuf::from("flux.db"),
        };

        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open SQLite database at {:?}: {}", db_path, e))?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS known_apps (
                path TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                first_seen INTEGER NOT NULL,
                is_blocked INTEGER NOT NULL DEFAULT 0,
                down_limit_kbps INTEGER,
                up_limit_kbps INTEGER,
                priority TEXT NOT NULL DEFAULT 'Normal'
            );

            CREATE TABLE IF NOT EXISTS stream_rules (
                stream_id TEXT PRIMARY KEY,
                is_blocked INTEGER NOT NULL DEFAULT 0,
                down_limit_kbps INTEGER,
                up_limit_kbps INTEGER
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS daily_traffic (
                date_str TEXT PRIMARY KEY,
                down_bytes INTEGER NOT NULL DEFAULT 0,
                up_bytes INTEGER NOT NULL DEFAULT 0
            );
            ",
        ).map_err(|e| format!("Failed to create tables: {}", e))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn add_daily_traffic(&self, down_bytes: u64, up_bytes: u64) -> Result<(), String> {
        let date_str = get_today_date_string();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO daily_traffic (date_str, down_bytes, up_bytes)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(date_str) DO UPDATE SET
                down_bytes = down_bytes + ?2,
                up_bytes = up_bytes + ?3",
            params![date_str, down_bytes as i64, up_bytes as i64],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_today_traffic(&self) -> (u64, u64) {
        let date_str = get_today_date_string();
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare("SELECT down_bytes, up_bytes FROM daily_traffic WHERE date_str = ?1") {
            Ok(s) => s,
            Err(_) => return (0, 0),
        };
        stmt.query_row(params![date_str], |row| {
            let d: i64 = row.get(0)?;
            let u: i64 = row.get(1)?;
            Ok((d.max(0) as u64, u.max(0) as u64))
        }).unwrap_or((0, 0))
    }

    pub fn get_setting(&self, key: &str, default_val: &str) -> String {
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare("SELECT value FROM settings WHERE key = ?1") {
            Ok(s) => s,
            Err(_) => return default_val.to_string(),
        };
        stmt.query_row(params![key], |row| row.get(0)).unwrap_or_else(|_| default_val.to_string())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Check if application path is known; if not, registers it
    pub fn register_app_if_new(&self, path: &str, name: &str) -> bool {
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare("SELECT COUNT(*) FROM known_apps WHERE path = ?1") {
            Ok(s) => s,
            Err(_) => return false,
        };

        let count: i64 = stmt.query_row(params![path], |row| row.get(0)).unwrap_or(0);
        if count == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let _ = conn.execute(
                "INSERT INTO known_apps (path, name, first_seen, is_blocked, priority) VALUES (?1, ?2, ?3, 0, 'Normal')",
                params![path, name, now],
            );
            true // Was newly added!
        } else {
            false // Already known
        }
    }

    /// Get all stored application rules
    pub fn get_all_app_rules(&self) -> Vec<AppRule> {
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare("SELECT path, name, is_blocked, down_limit_kbps, up_limit_kbps, priority FROM known_apps") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows = stmt.query_map([], |row| {
            let is_blocked_int: i32 = row.get(2)?;
            let down_limit: Option<i64> = row.get(3)?;
            let up_limit: Option<i64> = row.get(4)?;

            Ok(AppRule {
                path: row.get(0)?,
                name: row.get(1)?,
                is_blocked: is_blocked_int != 0,
                down_limit_kbps: down_limit.map(|v| v as u64),
                up_limit_kbps: up_limit.map(|v| v as u64),
                priority: row.get(5)?,
            })
        });

        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Save or update an application rule
    pub fn set_app_rule(&self, rule: &AppRule) -> Result<(), String> {
        let conn = self.conn.lock();
        let is_blocked_int = if rule.is_blocked { 1 } else { 0 };
        let down_limit = rule.down_limit_kbps.map(|v| v as i64);
        let up_limit = rule.up_limit_kbps.map(|v| v as i64);

        conn.execute(
            "INSERT INTO known_apps (path, name, first_seen, is_blocked, down_limit_kbps, up_limit_kbps, priority)
             VALUES (?1, ?2, unixepoch(), ?3, ?4, ?5, ?6)
             ON CONFLICT(path) DO UPDATE SET
                is_blocked = excluded.is_blocked,
                down_limit_kbps = excluded.down_limit_kbps,
                up_limit_kbps = excluded.up_limit_kbps,
                priority = excluded.priority;",
            params![rule.path, rule.name, is_blocked_int, down_limit, up_limit, rule.priority],
        ).map_err(|e| format!("SQLite set_app_rule error: {}", e))?;

        Ok(())
    }

    /// Save stream-level rule
    pub fn set_stream_rule(&self, rule: &StreamRule) -> Result<(), String> {
        let conn = self.conn.lock();
        let is_blocked_int = if rule.is_blocked { 1 } else { 0 };
        let down_limit = rule.down_limit_kbps.map(|v| v as i64);
        let up_limit = rule.up_limit_kbps.map(|v| v as i64);

        conn.execute(
            "INSERT INTO stream_rules (stream_id, is_blocked, down_limit_kbps, up_limit_kbps)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(stream_id) DO UPDATE SET
                is_blocked = excluded.is_blocked,
                down_limit_kbps = excluded.down_limit_kbps,
                up_limit_kbps = excluded.up_limit_kbps;",
            params![rule.stream_id, is_blocked_int, down_limit, up_limit],
        ).map_err(|e| format!("SQLite set_stream_rule error: {}", e))?;

        Ok(())
    }

    /// Get all stored stream rules
    pub fn get_all_stream_rules(&self) -> Vec<StreamRule> {
        let conn = self.conn.lock();
        let mut stmt = match conn.prepare("SELECT stream_id, is_blocked, down_limit_kbps, up_limit_kbps FROM stream_rules") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows = stmt.query_map([], |row| {
            let is_blocked_int: i32 = row.get(1)?;
            let down_limit: Option<i64> = row.get(2)?;
            let up_limit: Option<i64> = row.get(3)?;

            Ok(StreamRule {
                stream_id: row.get(0)?,
                is_blocked: is_blocked_int != 0,
                down_limit_kbps: down_limit.map(|v| v as u64),
                up_limit_kbps: up_limit.map(|v| v as u64),
            })
        });

        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }
}

fn dirs_or_local() -> Option<PathBuf> {
    if let Ok(app_data) = std::env::var("APPDATA") {
        let flux_dir = PathBuf::from(&app_data).join("Flux");
        let old_dir = PathBuf::from(&app_data).join("NetFlowStudio");
        let old_db = old_dir.join("netflow_studio.db");
        let new_db = flux_dir.join("flux.db");
        if old_db.exists() && !new_db.exists() {
            let _ = std::fs::create_dir_all(&flux_dir);
            let _ = std::fs::copy(&old_db, &new_db);
        }
        Some(flux_dir)
    } else {
        None
    }
}

pub fn get_today_date_string() -> String {
    #[repr(C)]
    struct WinSysTime {
        w_year: u16,
        w_month: u16,
        w_day_of_week: u16,
        w_day: u16,
        w_hour: u16,
        w_minute: u16,
        w_second: u16,
        w_milliseconds: u16,
    }
    extern "system" {
        fn GetLocalTime(lpSystemTime: *mut WinSysTime);
    }
    let mut st = WinSysTime {
        w_year: 0,
        w_month: 0,
        w_day_of_week: 0,
        w_day: 0,
        w_hour: 0,
        w_minute: 0,
        w_second: 0,
        w_milliseconds: 0,
    };
    unsafe { GetLocalTime(&mut st); }
    format!("{:04}-{:02}-{:02}", st.w_year, st.w_month, st.w_day)
}
