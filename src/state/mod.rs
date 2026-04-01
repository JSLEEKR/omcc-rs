//! State persistence: file-based state under `.omc/state/`, session isolation, atomic writes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateError {
    #[error("state I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("state serialization error: {0}")]
    SerializeError(#[from] serde_json::Error),
    #[error("state directory not initialized")]
    NotInitialized,
    #[error("session not found: {0}")]
    SessionNotFound(String),
}

/// A key-value state entry with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: String,
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
}

/// Notepad entry that survives compaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotepadEntry {
    pub id: String,
    pub content: String,
    pub created_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Session state container.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
    pub session_id: String,
    pub entries: HashMap<String, StateEntry>,
    pub notepad: Vec<NotepadEntry>,
    pub created_at: String,
    #[serde(default)]
    pub mode: Option<String>,
}

/// File-based state manager.
#[derive(Debug)]
pub struct StateManager {
    base_dir: PathBuf,
    current_session: Option<String>,
}

impl StateManager {
    /// Create a new state manager with a base directory.
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
            current_session: None,
        }
    }

    /// Initialize the state directory structure.
    pub fn init(&self) -> Result<(), StateError> {
        std::fs::create_dir_all(&self.base_dir)?;
        std::fs::create_dir_all(self.base_dir.join("sessions"))?;
        std::fs::create_dir_all(self.base_dir.join("notepad"))?;
        Ok(())
    }

    /// Start or resume a session.
    pub fn start_session(&mut self, session_id: &str) -> Result<SessionState, StateError> {
        self.init()?;
        self.current_session = Some(session_id.to_string());

        let session_file = self.session_path(session_id);
        if session_file.exists() {
            let content = std::fs::read_to_string(&session_file)?;
            let state: SessionState = serde_json::from_str(&content)?;
            Ok(state)
        } else {
            let now = chrono::Utc::now().to_rfc3339();
            let state = SessionState {
                session_id: session_id.to_string(),
                entries: HashMap::new(),
                notepad: Vec::new(),
                created_at: now,
                mode: None,
            };
            self.save_session(&state)?;
            Ok(state)
        }
    }

    /// Save session state atomically (write to temp, then rename).
    pub fn save_session(&self, state: &SessionState) -> Result<(), StateError> {
        let session_file = self.session_path(&state.session_id);
        let tmp_file = session_file.with_extension("tmp");
        let content = serde_json::to_string_pretty(state)?;
        std::fs::write(&tmp_file, &content)?;
        std::fs::rename(&tmp_file, &session_file)?;
        Ok(())
    }

    /// Get the current session ID.
    pub fn current_session_id(&self) -> Option<&str> {
        self.current_session.as_deref()
    }

    /// Set a key-value pair in the current session.
    pub fn set(
        &self,
        state: &mut SessionState,
        key: &str,
        value: serde_json::Value,
        ttl: Option<u64>,
    ) {
        let now = chrono::Utc::now().to_rfc3339();
        state.entries.insert(
            key.to_string(),
            StateEntry {
                key: key.to_string(),
                value,
                updated_at: now,
                ttl_seconds: ttl,
            },
        );
    }

    /// Get a value from the current session.
    pub fn get<'a>(&self, state: &'a SessionState, key: &str) -> Option<&'a serde_json::Value> {
        state.entries.get(key).map(|e| &e.value)
    }

    /// Remove a key from the session.
    pub fn remove(&self, state: &mut SessionState, key: &str) -> bool {
        state.entries.remove(key).is_some()
    }

    /// Add a notepad entry.
    pub fn add_note(
        &self,
        state: &mut SessionState,
        id: &str,
        content: &str,
        tags: Vec<String>,
    ) {
        let now = chrono::Utc::now().to_rfc3339();
        state.notepad.push(NotepadEntry {
            id: id.to_string(),
            content: content.to_string(),
            created_at: now,
            tags,
        });
    }

    /// Get all notepad entries.
    pub fn notes<'a>(&self, state: &'a SessionState) -> &'a [NotepadEntry] {
        &state.notepad
    }

    /// List all session IDs.
    pub fn list_sessions(&self) -> Result<Vec<String>, StateError> {
        let sessions_dir = self.base_dir.join("sessions");
        if !sessions_dir.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    ids.push(stem.to_string_lossy().to_string());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Delete a session.
    pub fn delete_session(&self, session_id: &str) -> Result<(), StateError> {
        let path = self.session_path(session_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Clean up expired entries from a session.
    pub fn cleanup_expired(&self, state: &mut SessionState) {
        let now = chrono::Utc::now();
        state.entries.retain(|_, entry| {
            if let Some(ttl) = entry.ttl_seconds {
                if let Ok(updated) = chrono::DateTime::parse_from_rfc3339(&entry.updated_at) {
                    let expiry =
                        updated + chrono::Duration::try_seconds(ttl as i64).unwrap_or_default();
                    return now < expiry;
                }
            }
            true
        });
    }

    fn session_path(&self, session_id: &str) -> PathBuf {
        // Sanitize session_id to prevent path traversal
        let sanitized: String = session_id
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        self.base_dir.join("sessions").join(format!("{sanitized}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, StateManager) {
        let dir = TempDir::new().unwrap();
        let mgr = StateManager::new(dir.path());
        (dir, mgr)
    }

    #[test]
    fn test_init_creates_directories() {
        let (dir, mgr) = setup();
        mgr.init().unwrap();
        assert!(dir.path().join("sessions").exists());
        assert!(dir.path().join("notepad").exists());
    }

    #[test]
    fn test_start_new_session() {
        let (_dir, mut mgr) = setup();
        let state = mgr.start_session("test-001").unwrap();
        assert_eq!(state.session_id, "test-001");
        assert!(state.entries.is_empty());
        assert_eq!(mgr.current_session_id(), Some("test-001"));
    }

    #[test]
    fn test_set_and_get() {
        let (_dir, mut mgr) = setup();
        let mut state = mgr.start_session("s1").unwrap();
        mgr.set(&mut state, "mode", serde_json::json!("autopilot"), None);
        let val = mgr.get(&state, "mode").unwrap();
        assert_eq!(val, &serde_json::json!("autopilot"));
    }

    #[test]
    fn test_get_missing_key() {
        let (_dir, mut mgr) = setup();
        let state = mgr.start_session("s2").unwrap();
        assert!(mgr.get(&state, "nonexistent").is_none());
    }

    #[test]
    fn test_remove_key() {
        let (_dir, mut mgr) = setup();
        let mut state = mgr.start_session("s3").unwrap();
        mgr.set(&mut state, "key1", serde_json::json!(42), None);
        assert!(mgr.remove(&mut state, "key1"));
        assert!(!mgr.remove(&mut state, "key1"));
    }

    #[test]
    fn test_save_and_restore_session() {
        let (_dir, mut mgr) = setup();
        let mut state = mgr.start_session("s4").unwrap();
        mgr.set(&mut state, "data", serde_json::json!({"x": 1}), None);
        mgr.save_session(&state).unwrap();

        // Restore
        let restored = mgr.start_session("s4").unwrap();
        let val = mgr.get(&restored, "data").unwrap();
        assert_eq!(val, &serde_json::json!({"x": 1}));
    }

    #[test]
    fn test_notepad() {
        let (_dir, mut mgr) = setup();
        let mut state = mgr.start_session("s5").unwrap();
        mgr.add_note(&mut state, "n1", "Remember this", vec!["important".to_string()]);
        mgr.add_note(&mut state, "n2", "Also this", vec![]);
        let notes = mgr.notes(&state);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].content, "Remember this");
        assert_eq!(notes[0].tags, vec!["important"]);
    }

    #[test]
    fn test_list_sessions() {
        let (_dir, mut mgr) = setup();
        mgr.start_session("alpha").unwrap();
        mgr.start_session("beta").unwrap();
        mgr.start_session("gamma").unwrap();
        let sessions = mgr.list_sessions().unwrap();
        assert_eq!(sessions, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn test_delete_session() {
        let (_dir, mut mgr) = setup();
        mgr.start_session("deleteme").unwrap();
        mgr.delete_session("deleteme").unwrap();
        let sessions = mgr.list_sessions().unwrap();
        assert!(!sessions.contains(&"deleteme".to_string()));
    }

    #[test]
    fn test_session_mode() {
        let (_dir, mut mgr) = setup();
        let mut state = mgr.start_session("s6").unwrap();
        state.mode = Some("ralph".to_string());
        mgr.save_session(&state).unwrap();
        let restored = mgr.start_session("s6").unwrap();
        assert_eq!(restored.mode, Some("ralph".to_string()));
    }

    #[test]
    fn test_path_traversal_prevention() {
        let (_dir, mut mgr) = setup();
        // A malicious session_id with path traversal should be sanitized
        let state = mgr.start_session("../../etc/passwd").unwrap();
        // The session_id in the returned state should match what was requested,
        // but the file path should be sanitized
        assert_eq!(state.session_id, "../../etc/passwd");
        // Verify the actual file is in the correct directory
        let sessions = mgr.list_sessions().unwrap();
        assert!(sessions.iter().all(|s| !s.contains("..")));
    }

    #[test]
    fn test_cleanup_expired() {
        let (_dir, mut mgr) = setup();
        let mut state = mgr.start_session("s7").unwrap();
        // Set an entry with 0-second TTL (already expired)
        state.entries.insert(
            "expired".to_string(),
            StateEntry {
                key: "expired".to_string(),
                value: serde_json::json!("old"),
                updated_at: "2020-01-01T00:00:00+00:00".to_string(),
                ttl_seconds: Some(1),
            },
        );
        mgr.set(&mut state, "fresh", serde_json::json!("new"), None);
        mgr.cleanup_expired(&mut state);
        assert!(mgr.get(&state, "expired").is_none());
        assert!(mgr.get(&state, "fresh").is_some());
    }
}
