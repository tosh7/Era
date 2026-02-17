// Screenshot capture and observation policy for debugging tap operations

use clap::ValueEnum;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Controls when screenshots are captured during tap operations
#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum ObservationPolicy {
    /// Capture screenshots at every step
    Always,
    /// Capture screenshots only when retry is triggered (default)
    #[value(name = "on-failure")]
    OnFailure,
    /// No screenshot capture
    Never,
}

/// Configuration for screenshot capture behavior
pub struct CaptureConfig {
    /// When to capture screenshots
    pub policy: ObservationPolicy,
    /// Whether to save screenshots to disk
    pub debug_capture: bool,
    /// Directory for debug screenshots
    pub debug_dir: String,
}

impl CaptureConfig {
    pub fn new(policy: ObservationPolicy, debug_capture: bool, debug_dir: String) -> Self {
        Self {
            policy,
            debug_capture,
            debug_dir,
        }
    }
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            policy: ObservationPolicy::OnFailure,
            debug_capture: false,
            debug_dir: "/tmp/era-debug/".to_string(),
        }
    }
}

/// Capture a screenshot and save to disk if debug_capture is enabled.
/// This is a no-op if debug_capture is false.
/// Failures are logged to stderr but do not propagate errors.
pub fn observe(udid: &str, config: &CaptureConfig, label: &str) {
    if !config.debug_capture {
        return;
    }

    match capture_screenshot(udid) {
        Ok(bytes) => {
            if let Err(e) = save_debug_screenshot(&config.debug_dir, &bytes, label) {
                eprintln!("[era] Warning: failed to save debug screenshot: {}", e);
            }
        }
        Err(e) => {
            eprintln!("[era] Warning: failed to capture screenshot: {}", e);
        }
    }
}

/// Capture a screenshot to memory as PNG bytes.
///
/// Tries `xcrun simctl io <udid> screenshot --type=png -` (stdout) first.
/// Falls back to a temp file if stdout capture is not supported.
fn capture_screenshot(udid: &str) -> Result<Vec<u8>, String> {
    // Try stdout capture first
    let output = Command::new("xcrun")
        .args(["simctl", "io", udid, "screenshot", "--type=png", "-"])
        .output()
        .map_err(|e| format!("Failed to run simctl: {}", e))?;

    if output.status.success() && !output.stdout.is_empty() {
        return Ok(output.stdout);
    }

    // Fallback: write to temp file, read bytes, delete
    let temp_path = format!("/tmp/era_temp_{}.png", std::process::id());
    let output = Command::new("xcrun")
        .args(["simctl", "io", udid, "screenshot", &temp_path])
        .output()
        .map_err(|e| format!("Failed to run simctl: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("simctl screenshot failed: {}", stderr));
    }

    let bytes = fs::read(&temp_path).map_err(|e| format!("Failed to read temp file: {}", e))?;
    let _ = fs::remove_file(&temp_path);
    Ok(bytes)
}

/// Save screenshot bytes to disk with a timestamped filename.
///
/// # Returns
/// The path of the saved file.
fn save_debug_screenshot(dir: &str, data: &[u8], label: &str) -> Result<String, String> {
    let dir_path = Path::new(dir);
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)
            .map_err(|e| format!("Failed to create debug dir: {}", e))?;
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let filename = format!("tap_{}_{}.png", ts, label);
    let filepath = dir_path.join(&filename);

    fs::write(&filepath, data).map_err(|e| format!("Failed to write screenshot: {}", e))?;

    eprintln!("[era] Debug screenshot saved: {}", filepath.display());
    Ok(filepath.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CaptureConfig::default();
        assert_eq!(config.policy, ObservationPolicy::OnFailure);
        assert!(!config.debug_capture);
        assert_eq!(config.debug_dir, "/tmp/era-debug/");
    }

    #[test]
    fn test_observe_noop_when_disabled() {
        let config = CaptureConfig::default();
        // Should not panic or attempt capture when debug_capture is false
        observe("fake-udid", &config, "test");
    }

    #[test]
    fn test_observation_policy_variants() {
        let _ = ObservationPolicy::Always;
        let _ = ObservationPolicy::OnFailure;
        let _ = ObservationPolicy::Never;
    }
}
