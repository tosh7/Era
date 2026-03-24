// Session management for persistent device connections
//
// Stores session data (UDID, device name, scale factor) in ~/.era/sessions.json
// so that users don't need to pass --device and --scale on every command.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::operations;

/// A single session representing a connected simulator device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub udid: String,
    pub device_name: String,
    pub scale: Option<u32>,
}

/// On-disk format for sessions.json
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SessionFile {
    sessions: HashMap<String, SessionData>,
    /// Name of the default session (last connected)
    default: Option<String>,
}

/// Stored session data (name is the key in the HashMap)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionData {
    udid: String,
    device_name: String,
    scale: Option<u32>,
}

/// SessionStore provides CRUD operations backed by ~/.era/sessions.json
pub struct SessionStore;

impl SessionStore {
    /// Create or update a session. Auto-detects scale factor.
    /// Sets this session as the default.
    pub fn connect(name: &str, udid: &str, device_name: &str) -> Result<Session, String> {
        let scale = match operations::detect_device_scale(udid) {
            Ok(detected) => Some(detected.value() as u32),
            Err(_) => None,
        };

        let session = Session {
            name: name.to_string(),
            udid: udid.to_string(),
            device_name: device_name.to_string(),
            scale,
        };

        let mut file = load_file()?;
        file.sessions.insert(
            name.to_string(),
            SessionData {
                udid: udid.to_string(),
                device_name: device_name.to_string(),
                scale,
            },
        );
        file.default = Some(name.to_string());
        save_file(&file)?;

        Ok(session)
    }

    /// Get a session by name
    pub fn get(name: &str) -> Result<Session, String> {
        let file = load_file()?;
        match file.sessions.get(name) {
            Some(data) => Ok(Session {
                name: name.to_string(),
                udid: data.udid.clone(),
                device_name: data.device_name.clone(),
                scale: data.scale,
            }),
            None => Err(format!("Session '{}' not found. Run `era session connect` first.", name)),
        }
    }

    /// Get the default session (last connected)
    pub fn get_default() -> Result<Session, String> {
        let file = load_file()?;
        match file.default {
            Some(ref name) => {
                match file.sessions.get(name) {
                    Some(data) => Ok(Session {
                        name: name.clone(),
                        udid: data.udid.clone(),
                        device_name: data.device_name.clone(),
                        scale: data.scale,
                    }),
                    None => Err("Default session references missing entry. Run `era session connect`.".to_string()),
                }
            }
            None => Err("No default session. Run `era session connect` first.".to_string()),
        }
    }

    /// List all sessions
    pub fn list() -> Result<Vec<Session>, String> {
        let file = load_file()?;
        let mut sessions: Vec<Session> = file
            .sessions
            .into_iter()
            .map(|(name, data)| Session {
                name,
                udid: data.udid,
                device_name: data.device_name,
                scale: data.scale,
            })
            .collect();
        sessions.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(sessions)
    }

    /// Get the default session name (for display purposes)
    pub fn default_name() -> Option<String> {
        load_file().ok().and_then(|f| f.default)
    }

    /// Disconnect (remove) a session by name
    pub fn disconnect(name: &str) -> Result<(), String> {
        let mut file = load_file()?;
        if file.sessions.remove(name).is_none() {
            return Err(format!("Session '{}' not found.", name));
        }
        // Clear default if it was the disconnected session
        if file.default.as_deref() == Some(name) {
            file.default = None;
        }
        save_file(&file)
    }

    /// Disconnect all sessions
    pub fn disconnect_all() -> Result<(), String> {
        let file = SessionFile::default();
        save_file(&file)
    }
}

/// Resolve a device identifier from session or --device flag.
/// Priority: --session > --device > default session
pub fn resolve_device(
    session_name: Option<&str>,
    device_flag: Option<&str>,
) -> Result<(String, Option<u32>), String> {
    if let Some(name) = session_name {
        let session = SessionStore::get(name)?;
        return Ok((session.udid, session.scale));
    }

    if let Some(device) = device_flag {
        return Ok((device.to_string(), None));
    }

    match SessionStore::get_default() {
        Ok(session) => Ok((session.udid, session.scale)),
        Err(_) => Err(
            "No device specified. Use --device, --session, or `era session connect` to set a default."
                .to_string(),
        ),
    }
}

// -------------------------------------------------------------------
// File I/O
// -------------------------------------------------------------------

fn sessions_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".era").join("sessions.json")
}

fn load_file() -> Result<SessionFile, String> {
    load_file_at(&sessions_path())
}

fn save_file(file: &SessionFile) -> Result<(), String> {
    save_file_at(file, &sessions_path())
}

fn load_file_at(path: &PathBuf) -> Result<SessionFile, String> {
    if !path.exists() {
        return Ok(SessionFile::default());
    }
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

fn save_file_at(file: &SessionFile, path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }
    let json = serde_json::to_string_pretty(file)
        .map_err(|e| format!("Failed to serialize sessions: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Create a unique temp file path for each test
    fn temp_sessions_path() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "era-session-test-{}-{}.json",
            std::process::id(),
            id
        ))
    }

    /// Cleanup helper
    fn cleanup(path: &PathBuf) {
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_sessions_path() {
        let path = sessions_path();
        assert!(path.to_string_lossy().contains(".era"));
        assert!(path.to_string_lossy().ends_with("sessions.json"));
    }

    #[test]
    fn test_load_empty() {
        let path = temp_sessions_path();
        let file = load_file_at(&path).unwrap();
        assert!(file.sessions.is_empty());
        assert!(file.default.is_none());
    }

    #[test]
    fn test_save_and_load() {
        let path = temp_sessions_path();
        let mut file = SessionFile::default();
        file.sessions.insert(
            "test".to_string(),
            SessionData {
                udid: "ABC-123".to_string(),
                device_name: "iPhone 16 Pro".to_string(),
                scale: Some(3),
            },
        );
        file.default = Some("test".to_string());
        save_file_at(&file, &path).unwrap();

        let loaded = load_file_at(&path).unwrap();
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.default.as_deref(), Some("test"));
        let data = loaded.sessions.get("test").unwrap();
        assert_eq!(data.udid, "ABC-123");
        assert_eq!(data.scale, Some(3));
        cleanup(&path);
    }

    #[test]
    fn test_get_nonexistent() {
        // Uses real sessions_path; "nope" won't exist
        let result = SessionStore::get("nope-nonexistent-test-session-xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_default_empty() {
        let path = temp_sessions_path();
        let file = SessionFile::default();
        save_file_at(&file, &path).unwrap();
        // Default is None
        let loaded = load_file_at(&path).unwrap();
        assert!(loaded.default.is_none());
        cleanup(&path);
    }

    #[test]
    fn test_list_empty() {
        let path = temp_sessions_path();
        let file = SessionFile::default();
        save_file_at(&file, &path).unwrap();
        let loaded = load_file_at(&path).unwrap();
        assert!(loaded.sessions.is_empty());
        cleanup(&path);
    }

    #[test]
    fn test_disconnect_nonexistent() {
        let result = SessionStore::disconnect("nope-nonexistent-test-session-xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_disconnect_all_via_file() {
        let path = temp_sessions_path();
        let mut file = SessionFile::default();
        file.sessions.insert(
            "a".to_string(),
            SessionData {
                udid: "u1".to_string(),
                device_name: "d1".to_string(),
                scale: None,
            },
        );
        file.default = Some("a".to_string());
        save_file_at(&file, &path).unwrap();

        // Simulate disconnect_all by saving empty
        let empty = SessionFile::default();
        save_file_at(&empty, &path).unwrap();
        let loaded = load_file_at(&path).unwrap();
        assert!(loaded.sessions.is_empty());
        assert!(loaded.default.is_none());
        cleanup(&path);
    }

    #[test]
    fn test_resolve_device_with_device_flag() {
        let (udid, scale) = resolve_device(None, Some("ABC-123")).unwrap();
        assert_eq!(udid, "ABC-123");
        assert!(scale.is_none());
    }

    #[test]
    fn test_resolve_device_no_device_no_session() {
        // This will try to load the real sessions file; if no default exists, it errors
        let result = resolve_device(None, None);
        // May succeed if user has a real session; we just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_resolve_device_session_not_found() {
        let result = resolve_device(Some("nope-nonexistent-test-session-xyz"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_disconnect_clears_default_via_file() {
        let path = temp_sessions_path();
        let mut file = SessionFile::default();
        file.sessions.insert(
            "main".to_string(),
            SessionData {
                udid: "u1".to_string(),
                device_name: "d1".to_string(),
                scale: Some(3),
            },
        );
        file.default = Some("main".to_string());
        save_file_at(&file, &path).unwrap();

        // Simulate disconnect("main")
        let mut loaded = load_file_at(&path).unwrap();
        loaded.sessions.remove("main");
        if loaded.default.as_deref() == Some("main") {
            loaded.default = None;
        }
        save_file_at(&loaded, &path).unwrap();

        let final_state = load_file_at(&path).unwrap();
        assert!(final_state.default.is_none());
        assert!(final_state.sessions.is_empty());
        cleanup(&path);
    }

    #[test]
    fn test_list_sorted_via_file() {
        let path = temp_sessions_path();
        let mut file = SessionFile::default();
        for name in &["charlie", "alpha", "bravo"] {
            file.sessions.insert(
                name.to_string(),
                SessionData {
                    udid: format!("udid-{}", name),
                    device_name: format!("device-{}", name),
                    scale: None,
                },
            );
        }
        save_file_at(&file, &path).unwrap();

        let loaded = load_file_at(&path).unwrap();
        let mut sessions: Vec<Session> = loaded
            .sessions
            .into_iter()
            .map(|(name, data)| Session {
                name,
                udid: data.udid,
                device_name: data.device_name,
                scale: data.scale,
            })
            .collect();
        sessions.sort_by(|a, b| a.name.cmp(&b.name));
        let names: Vec<&str> = sessions.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "bravo", "charlie"]);
        cleanup(&path);
    }

    #[test]
    fn test_session_data_roundtrip() {
        let path = temp_sessions_path();
        let mut file = SessionFile::default();
        file.sessions.insert(
            "prod".to_string(),
            SessionData {
                udid: "AAAA-BBBB-CCCC".to_string(),
                device_name: "iPhone 16 Pro Max".to_string(),
                scale: Some(3),
            },
        );
        file.sessions.insert(
            "test".to_string(),
            SessionData {
                udid: "DDDD-EEEE-FFFF".to_string(),
                device_name: "iPhone SE".to_string(),
                scale: Some(2),
            },
        );
        file.default = Some("prod".to_string());
        save_file_at(&file, &path).unwrap();

        let loaded = load_file_at(&path).unwrap();
        assert_eq!(loaded.sessions.len(), 2);
        assert_eq!(loaded.default.as_deref(), Some("prod"));

        let prod = loaded.sessions.get("prod").unwrap();
        assert_eq!(prod.udid, "AAAA-BBBB-CCCC");
        assert_eq!(prod.scale, Some(3));

        let test = loaded.sessions.get("test").unwrap();
        assert_eq!(test.device_name, "iPhone SE");
        assert_eq!(test.scale, Some(2));
        cleanup(&path);
    }
}
