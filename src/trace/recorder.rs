// Trace recorder — records each operation step during a trace session
//
// Uses a global singleton backed by a file lock so that both CLI invocations
// and MCP server calls can append steps to the active trace.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::store;

/// A single recorded step in a trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    pub seq: u32,
    pub timestamp: String,
    pub action: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_after: Option<String>,
}

/// Device info captured at trace start
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceDevice {
    pub udid: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<u32>,
}

/// Assertion summary within a trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionSummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
}

/// Summary statistics for a completed trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummary {
    pub total_steps: u32,
    pub total_duration_ms: u64,
    pub assertions: AssertionSummary,
    pub retries: u32,
}

/// Complete trace data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceData {
    pub trace_id: String,
    pub name: String,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<TraceDevice>,
    pub steps: Vec<TraceStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<TraceSummary>,
}

/// Active trace state persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActiveTrace {
    trace_id: String,
    name: String,
    trace_dir: String,
}

impl TraceData {
    pub fn new(name: &str) -> Self {
        let now = now_iso8601();
        let ts_suffix = now_compact();
        let trace_id = format!("{}-{}", name, ts_suffix);

        Self {
            trace_id,
            name: name.to_string(),
            started_at: now,
            ended_at: None,
            device: None,
            steps: Vec::new(),
            summary: None,
        }
    }

    /// Compute summary from recorded steps
    pub fn compute_summary(&mut self) {
        let total_steps = self.steps.len() as u32;
        let total_duration_ms: u64 = self.steps.iter().map(|s| s.duration_ms).sum();

        let mut assert_total = 0u32;
        let mut assert_passed = 0u32;
        let mut assert_failed = 0u32;
        let mut retries = 0u32;

        for step in &self.steps {
            if step.action == "assert" {
                assert_total += 1;
                if step.output.get("pass").and_then(|v| v.as_bool()).unwrap_or(false) {
                    assert_passed += 1;
                } else {
                    assert_failed += 1;
                }
            }
            if let Some(r) = step.output.get("retries").and_then(|v| v.as_u64()) {
                retries += r as u32;
            }
        }

        self.summary = Some(TraceSummary {
            total_steps,
            total_duration_ms,
            assertions: AssertionSummary {
                total: assert_total,
                passed: assert_passed,
                failed: assert_failed,
            },
            retries,
        });
    }
}

// ---------------------------------------------------------------------------
// Global active trace management
// ---------------------------------------------------------------------------

fn active_trace_path() -> PathBuf {
    store::era_dir().join("active-trace.json")
}

/// Start a new trace
pub fn start_trace(name: &str) -> Result<TraceData, String> {
    // Check if a trace is already active
    if get_active().is_some() {
        return Err("A trace is already active. Run `era trace stop` first.".to_string());
    }

    let trace = TraceData::new(name);
    let trace_dir = store::trace_dir(&trace.trace_id);

    // Create trace directory
    fs::create_dir_all(&trace_dir)
        .map_err(|e| format!("Failed to create trace directory: {}", e))?;

    // Save initial trace data
    store::save_trace(&trace)?;

    // Mark as active
    let active = ActiveTrace {
        trace_id: trace.trace_id.clone(),
        name: trace.name.clone(),
        trace_dir: trace_dir.to_string_lossy().to_string(),
    };
    let json = serde_json::to_string_pretty(&active)
        .map_err(|e| format!("Failed to serialize active trace: {}", e))?;

    let path = active_trace_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    fs::write(&path, json).map_err(|e| format!("Failed to write active trace: {}", e))?;

    Ok(trace)
}

/// Stop the active trace and finalize it
pub fn stop_trace() -> Result<TraceData, String> {
    let active = get_active()
        .ok_or_else(|| "No active trace. Run `era trace start` first.".to_string())?;

    let mut trace = store::load_trace(&active.trace_id)?;
    trace.ended_at = Some(now_iso8601());
    trace.compute_summary();

    // Save finalized trace
    store::save_trace(&trace)?;

    // Remove active marker
    let _ = fs::remove_file(active_trace_path());

    Ok(trace)
}

/// Record a step to the active trace (if any)
pub fn record_step(
    action: &str,
    input: serde_json::Value,
    output: serde_json::Value,
    duration_ms: u64,
) -> Result<(), String> {
    let active = match get_active() {
        Some(a) => a,
        None => return Ok(()), // No active trace, silently skip
    };

    let mut trace = store::load_trace(&active.trace_id)?;

    let seq = trace.steps.len() as u32 + 1;
    let step = TraceStep {
        seq,
        timestamp: now_iso8601(),
        action: action.to_string(),
        input,
        output,
        duration_ms,
        screenshot_before: None,
        screenshot_after: None,
    };

    trace.steps.push(step);
    store::save_trace(&trace)?;

    Ok(())
}

/// Check if a trace is currently active
pub fn is_active() -> bool {
    get_active().is_some()
}

/// Get the active trace info (if any)
fn get_active() -> Option<ActiveTrace> {
    let path = active_trace_path();
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

// ---------------------------------------------------------------------------
// Time helpers
// ---------------------------------------------------------------------------

fn now_iso8601() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Simple UTC timestamp without external chrono dependency
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Calculate year/month/day from days since epoch
    let (year, month, day) = days_to_ymd(days);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn now_compact() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;
    let days = secs / 86400;
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        year, month, day, hours, minutes, seconds
    )
}

fn days_to_ymd(days_since_epoch: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days_since_epoch + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_trace_data_new() {
        let trace = TraceData::new("login-flow");
        assert_eq!(trace.name, "login-flow");
        assert!(trace.trace_id.starts_with("login-flow-"));
        assert!(trace.ended_at.is_none());
        assert!(trace.steps.is_empty());
        assert!(trace.summary.is_none());
    }

    #[test]
    fn test_compute_summary_empty() {
        let mut trace = TraceData::new("test");
        trace.compute_summary();
        let summary = trace.summary.unwrap();
        assert_eq!(summary.total_steps, 0);
        assert_eq!(summary.total_duration_ms, 0);
        assert_eq!(summary.assertions.total, 0);
    }

    #[test]
    fn test_compute_summary_with_steps() {
        let mut trace = TraceData::new("test");
        trace.steps.push(TraceStep {
            seq: 1,
            timestamp: "2026-03-24T14:00:00Z".to_string(),
            action: "tap".to_string(),
            input: json!({"ref": 5}),
            output: json!({"retries": 1}),
            duration_ms: 500,
            screenshot_before: None,
            screenshot_after: None,
        });
        trace.steps.push(TraceStep {
            seq: 2,
            timestamp: "2026-03-24T14:00:01Z".to_string(),
            action: "assert".to_string(),
            input: json!({"visible": ["Login"]}),
            output: json!({"pass": true}),
            duration_ms: 200,
            screenshot_before: None,
            screenshot_after: None,
        });
        trace.steps.push(TraceStep {
            seq: 3,
            timestamp: "2026-03-24T14:00:02Z".to_string(),
            action: "assert".to_string(),
            input: json!({"visible": ["Missing"]}),
            output: json!({"pass": false}),
            duration_ms: 150,
            screenshot_before: None,
            screenshot_after: None,
        });

        trace.compute_summary();
        let summary = trace.summary.unwrap();
        assert_eq!(summary.total_steps, 3);
        assert_eq!(summary.total_duration_ms, 850);
        assert_eq!(summary.assertions.total, 2);
        assert_eq!(summary.assertions.passed, 1);
        assert_eq!(summary.assertions.failed, 1);
        assert_eq!(summary.retries, 1);
    }

    #[test]
    fn test_now_iso8601_format() {
        let ts = now_iso8601();
        // Should be like 2026-03-24T14:30:52Z
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
    }

    #[test]
    fn test_now_compact_format() {
        let ts = now_compact();
        // Should be like 20260324-143052
        assert_eq!(ts.len(), 15);
        assert_eq!(&ts[8..9], "-");
    }

    #[test]
    fn test_days_to_ymd_epoch() {
        let (y, m, d) = days_to_ymd(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_ymd_known_date() {
        // 2026-03-24 is day 20536 since epoch
        let (y, m, d) = days_to_ymd(20536);
        assert_eq!((y, m, d), (2026, 3, 24));
    }

    #[test]
    fn test_trace_step_serialization() {
        let step = TraceStep {
            seq: 1,
            timestamp: "2026-03-24T14:00:00Z".to_string(),
            action: "tap".to_string(),
            input: json!({"ref": 5}),
            output: json!({"tapped": true}),
            duration_ms: 300,
            screenshot_before: None,
            screenshot_after: Some("step001-after.png".to_string()),
        };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"action\":\"tap\""));
        assert!(!json.contains("screenshot_before")); // None should be skipped
        assert!(json.contains("screenshot_after"));
    }

    #[test]
    fn test_trace_data_serialization_roundtrip() {
        let mut trace = TraceData::new("test-roundtrip");
        trace.device = Some(TraceDevice {
            udid: "ABC-123".to_string(),
            name: "iPhone 16 Pro".to_string(),
            runtime: Some("iOS 26.0".to_string()),
            scale: Some(3),
        });
        trace.steps.push(TraceStep {
            seq: 1,
            timestamp: "2026-03-24T14:00:00Z".to_string(),
            action: "snapshot".to_string(),
            input: json!({}),
            output: json!({"element_count": 24}),
            duration_ms: 320,
            screenshot_before: None,
            screenshot_after: None,
        });

        let json = serde_json::to_string_pretty(&trace).unwrap();
        let parsed: TraceData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test-roundtrip");
        assert_eq!(parsed.steps.len(), 1);
        assert_eq!(parsed.device.unwrap().scale, Some(3));
    }
}
