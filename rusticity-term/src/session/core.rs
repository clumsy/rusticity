use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub timestamp: String,
    pub profile: String,
    pub region: String,
    pub account_id: String,
    pub role_arn: String,
    pub tabs: Vec<SessionTab>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTab {
    pub service: String,
    pub title: String,
    pub breadcrumb: String,
    pub filter: Option<String>,
    pub selected_item: Option<String>,
}

impl Session {
    pub fn new(profile: String, region: String, account_id: String, role_arn: String) -> Self {
        let timestamp = chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        let id = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();

        Self {
            id,
            timestamp,
            profile,
            region,
            account_id,
            role_arn,
            tabs: Vec::new(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let session_dir = Self::session_dir()?;
        std::fs::create_dir_all(&session_dir)?;

        let file_path = session_dir.join(format!("{}.json", self.id));
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(file_path, json)?;

        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        let session_dir = Self::session_dir()?;
        let file_path = session_dir.join(format!("{}.json", self.id));
        if file_path.exists() {
            std::fs::remove_file(file_path)?;
        }
        Ok(())
    }

    pub fn load(id: &str) -> Result<Self> {
        let session_dir = Self::session_dir()?;
        let file_path = session_dir.join(format!("{}.json", id));
        let json = std::fs::read_to_string(file_path)?;
        let session = serde_json::from_str(&json)?;
        Ok(session)
    }

    pub fn list_all() -> Result<Vec<Session>> {
        let session_dir = Self::session_dir()?;
        if !session_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(session_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(session) = serde_json::from_str::<Session>(&json) {
                        sessions.push(session);
                    }
                }
            }
        }

        // Sort by timestamp descending (newest first)
        sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(sessions)
    }

    fn session_dir() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        Ok(home.join(".rusticity").join("sessions"))
    }
}
