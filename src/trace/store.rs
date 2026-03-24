// Trace store — file I/O for trace data in ~/.era/traces/

use std::fs;
use std::path::PathBuf;

use super::recorder::TraceData;

/// Base directory for Era data: ~/.era/
pub fn era_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".era")
}

/// Base directory for traces: ~/.era/traces/
fn traces_dir() -> PathBuf {
    era_dir().join("traces")
}

/// Directory for a specific trace: ~/.era/traces/{trace_id}/
pub fn trace_dir(trace_id: &str) -> PathBuf {
    traces_dir().join(trace_id)
}

/// Path to the trace JSON file
fn trace_json_path(trace_id: &str) -> PathBuf {
    trace_dir(trace_id).join("trace.json")
}

/// Save trace data to disk
pub fn save_trace(trace: &TraceData) -> Result<(), String> {
    let dir = trace_dir(&trace.trace_id);
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create trace dir {}: {}", dir.display(), e))?;

    let path = trace_json_path(&trace.trace_id);
    let json = serde_json::to_string_pretty(trace)
        .map_err(|e| format!("Failed to serialize trace: {}", e))?;
    fs::write(&path, json)
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;

    Ok(())
}

/// Load trace data from disk
pub fn load_trace(trace_id: &str) -> Result<TraceData, String> {
    let path = trace_json_path(trace_id);
    if !path.exists() {
        return Err(format!("Trace not found: {}", trace_id));
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// List all traces (sorted by name descending = most recent first)
pub fn list_traces() -> Result<Vec<TraceData>, String> {
    let dir = traces_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut traces = Vec::new();
    let entries = fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read traces directory: {}", e))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.path().is_dir() {
            continue;
        }

        let trace_id = entry
            .file_name()
            .to_string_lossy()
            .to_string();

        let json_path = entry.path().join("trace.json");
        if !json_path.exists() {
            continue;
        }

        match load_trace(&trace_id) {
            Ok(trace) => traces.push(trace),
            Err(_) => continue,
        }
    }

    // Sort by trace_id descending (most recent first due to timestamp suffix)
    traces.sort_by(|a, b| b.trace_id.cmp(&a.trace_id));

    Ok(traces)
}

/// Delete a trace and its directory
pub fn delete_trace(trace_id: &str) -> Result<(), String> {
    let dir = trace_dir(trace_id);
    if !dir.exists() {
        return Err(format!("Trace not found: {}", trace_id));
    }
    fs::remove_dir_all(&dir)
        .map_err(|e| format!("Failed to delete trace {}: {}", trace_id, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::recorder::{TraceStep, TraceDevice};
    use serde_json::json;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn unique_trace_id() -> String {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("test-trace-{}-{}", std::process::id(), id)
    }

    #[test]
    fn test_era_dir() {
        let dir = era_dir();
        assert!(dir.to_string_lossy().contains(".era"));
    }

    #[test]
    fn test_trace_dir_path() {
        let dir = trace_dir("my-trace-123");
        assert!(dir.to_string_lossy().contains("traces"));
        assert!(dir.to_string_lossy().ends_with("my-trace-123"));
    }

    #[test]
    fn test_save_and_load_trace() {
        let trace_id = unique_trace_id();
        let mut trace = TraceData::new("test");
        trace.trace_id = trace_id.clone();
        trace.device = Some(TraceDevice {
            udid: "ABC-123".to_string(),
            name: "iPhone 16 Pro".to_string(),
            runtime: Some("iOS 26.0".to_string()),
            scale: Some(3),
        });
        trace.steps.push(TraceStep {
            seq: 1,
            timestamp: "2026-03-24T14:00:00Z".to_string(),
            action: "tap".to_string(),
            input: json!({"ref": 5}),
            output: json!({"tapped": true}),
            duration_ms: 300,
            screenshot_before: None,
            screenshot_after: None,
        });

        save_trace(&trace).unwrap();
        let loaded = load_trace(&trace_id).unwrap();
        assert_eq!(loaded.trace_id, trace_id);
        assert_eq!(loaded.steps.len(), 1);
        assert_eq!(loaded.device.unwrap().name, "iPhone 16 Pro");

        // Cleanup
        let _ = delete_trace(&trace_id);
    }

    #[test]
    fn test_load_nonexistent_trace() {
        let result = load_trace("nonexistent-trace-xyz-999");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_trace() {
        let trace_id = unique_trace_id();
        let mut trace = TraceData::new("delete-test");
        trace.trace_id = trace_id.clone();
        save_trace(&trace).unwrap();

        assert!(trace_dir(&trace_id).exists());
        delete_trace(&trace_id).unwrap();
        assert!(!trace_dir(&trace_id).exists());
    }

    #[test]
    fn test_delete_nonexistent_trace() {
        let result = delete_trace("nonexistent-trace-xyz-999");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_traces_empty() {
        // Should not panic even if traces dir doesn't exist
        let _ = list_traces();
    }
}
