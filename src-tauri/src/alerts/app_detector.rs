use std::sync::Arc;
use dashmap::DashSet;
use tauri::{AppHandle, Emitter};
use log::{info, warn};

use crate::models::NewAppAlert;
use crate::storage::db::Database;

pub struct AppDetector {
    db: Arc<Database>,
    alerted_paths: DashSet<String>,
    app_handle: parking_lot::RwLock<Option<AppHandle>>,
}

impl AppDetector {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            alerted_paths: DashSet::new(),
            app_handle: parking_lot::RwLock::new(None),
        }
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        *self.app_handle.write() = Some(handle);
    }

    pub fn check_new_app(
        &self,
        pid: u32,
        name: &str,
        path: &str,
        remote_ip: &str,
        remote_port: u16,
        protocol: &str,
    ) {
        if path.is_empty() || path == "Unknown" {
            return;
        }

        // Avoid repeated checks if already alerted in this session
        if self.alerted_paths.contains(path) {
            return;
        }

        // Check if new in database
        let is_new = self.db.register_app_if_new(path, name);
        self.alerted_paths.insert(path.to_string());
        if is_new {
            info!("New application network connection detected: {} ({})", name, path);

            let alert = NewAppAlert {
                pid,
                name: name.to_string(),
                path: path.to_string(),
                remote_ip: remote_ip.to_string(),
                remote_port,
                protocol: protocol.to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };

            // Emit to frontend
            if let Some(handle) = self.app_handle.read().as_ref() {
                let _ = handle.emit("new-app-detected", &alert);
            }
        } else {
            self.alerted_paths.insert(path.to_string());
        }
    }
}
