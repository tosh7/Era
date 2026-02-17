// IDB (iOS Development Bridge) integration - Facebook's idb tool wrapper
// IDB is an optional dependency for advanced UI automation features
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::process::Command;
use std::thread;
use std::time::Duration;
use thiserror::Error;

use crate::capture::{self, CaptureConfig, ObservationPolicy};

/// Errors that can occur during IDB operations
#[derive(Debug, Error)]
pub enum IdbError {
    #[error("IDB is not installed. Install with: brew install idb-companion")]
    NotInstalled,

    #[error("Failed to execute idb command: {0}")]
    CommandExecution(#[from] std::io::Error),

    #[error("IDB command failed with status {status}: {stderr}")]
    CommandFailed { status: i32, stderr: String },

    #[error("Invalid coordinates: {0}")]
    InvalidCoordinates(String),
}

pub type Result<T> = std::result::Result<T, IdbError>;

/// Check if IDB (idb) is installed and available
pub fn is_available() -> bool {
    Command::new("which")
        .arg("idb")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Execute an idb command and return the output
fn run_idb(args: &[&str]) -> Result<String> {
    if !is_available() {
        return Err(IdbError::NotInstalled);
    }

    let output = Command::new("idb").args(args).output()?;

    if !output.status.success() {
        let status = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(IdbError::CommandFailed { status, stderr });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Common scale factors for iOS devices
pub mod scale {
    /// iPhone 6/7/8, iPhone SE (2nd/3rd gen): 2x
    pub const SCALE_2X: f64 = 2.0;
    /// iPhone 6/7/8 Plus, iPhone X and later Pro models: 3x
    pub const SCALE_3X: f64 = 3.0;
}

/// Convert pixel coordinates to logical points (rounded to integer)
///
/// # Arguments
/// * `pixel` - Coordinate in pixels (from screenshot)
/// * `scale_factor` - Device scale factor (2.0 for 2x, 3.0 for 3x Retina displays)
///
/// # Returns
/// Coordinate in logical points as integer (for IDB)
///
/// # Note
/// IDB requires integer coordinates. This function rounds to nearest integer.
pub fn pixel_to_point(pixel: f64, scale_factor: f64) -> i64 {
    (pixel / scale_factor).round() as i64
}

/// Tap on a specific coordinate on the simulator screen
///
/// # Arguments
/// * `udid` - The device UDID
/// * `x` - X coordinate (in logical points, NOT pixels)
/// * `y` - Y coordinate (in logical points, NOT pixels)
///
/// # Note
/// IDB uses logical points, not pixels. Coordinates are rounded to integers.
/// For coordinates from screenshots, use `tap_pixel` with scale factor.
pub fn tap(udid: &str, x: f64, y: f64) -> Result<()> {
    if x < 0.0 || y < 0.0 {
        return Err(IdbError::InvalidCoordinates(format!(
            "Coordinates must be non-negative: ({}, {})",
            x, y
        )));
    }

    let info = super::orientation::detect_orientation(udid);
    let (tx, ty) = super::orientation::transform_coordinates(
        x, y, info.screen_width, info.screen_height, info.orientation,
    );

    tap_raw(udid, tx, ty)
}

/// Internal tap that sends already-transformed coordinates to idb.
/// Skips orientation detection — caller is responsible for transformation.
fn tap_raw(udid: &str, x: f64, y: f64) -> Result<()> {
    let x_int = x.round() as i64;
    let y_int = y.round() as i64;

    run_idb(&[
        "ui",
        "tap",
        "--udid",
        udid,
        &x_int.to_string(),
        &y_int.to_string(),
    ])?;
    Ok(())
}

/// Tap using pixel coordinates (from screenshots)
///
/// # Arguments
/// * `udid` - The device UDID
/// * `pixel_x` - X coordinate in pixels (from screenshot)
/// * `pixel_y` - Y coordinate in pixels (from screenshot)
/// * `scale_factor` - Device scale factor (e.g., 3.0 for iPhone Pro models)
///
/// # Example
/// ```ignore
/// // iPhone 16 Pro screenshot shows button at (630, 1368) pixels
/// // Use scale factor 3.0 to convert to logical points (210, 456)
/// tap_pixel(udid, 630.0, 1368.0, 3.0)?;
/// ```
pub fn tap_pixel(udid: &str, pixel_x: f64, pixel_y: f64, scale_factor: f64) -> Result<()> {
    let x = pixel_to_point(pixel_x, scale_factor) as f64;
    let y = pixel_to_point(pixel_y, scale_factor) as f64;
    tap(udid, x, y)
}

/// Perform a swipe gesture on the simulator screen
///
/// # Arguments
/// * `udid` - The device UDID
/// * `start_x` - Starting X coordinate (in logical points)
/// * `start_y` - Starting Y coordinate (in logical points)
/// * `end_x` - Ending X coordinate (in logical points)
/// * `end_y` - Ending Y coordinate (in logical points)
/// * `duration` - Optional duration in seconds (default: 0.5)
///
/// # Note
/// IDB uses logical points, not pixels. Coordinates are rounded to integers.
/// For coordinates from screenshots, use `swipe_pixel` with scale factor.
pub fn swipe(
    udid: &str,
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    duration: Option<f64>,
) -> Result<()> {
    if start_x < 0.0 || start_y < 0.0 || end_x < 0.0 || end_y < 0.0 {
        return Err(IdbError::InvalidCoordinates(format!(
            "Coordinates must be non-negative: ({}, {}) -> ({}, {})",
            start_x, start_y, end_x, end_y
        )));
    }

    let info = super::orientation::detect_orientation(udid);
    let (tx_start, ty_start) = super::orientation::transform_coordinates(
        start_x, start_y, info.screen_width, info.screen_height, info.orientation,
    );
    let (tx_end, ty_end) = super::orientation::transform_coordinates(
        end_x, end_y, info.screen_width, info.screen_height, info.orientation,
    );

    swipe_raw(udid, tx_start, ty_start, tx_end, ty_end, duration)
}

/// Internal swipe that sends already-transformed coordinates to idb.
/// Skips orientation detection — caller is responsible for transformation.
fn swipe_raw(
    udid: &str,
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    duration: Option<f64>,
) -> Result<()> {
    let start_x_int = start_x.round() as i64;
    let start_y_int = start_y.round() as i64;
    let end_x_int = end_x.round() as i64;
    let end_y_int = end_y.round() as i64;

    let duration_str = duration.unwrap_or(0.5).to_string();

    run_idb(&[
        "ui",
        "swipe",
        "--udid",
        udid,
        &start_x_int.to_string(),
        &start_y_int.to_string(),
        &end_x_int.to_string(),
        &end_y_int.to_string(),
        "--duration",
        &duration_str,
    ])?;
    Ok(())
}

/// Swipe using pixel coordinates (from screenshots)
///
/// # Arguments
/// * `udid` - The device UDID
/// * `start_pixel_x` - Starting X coordinate in pixels
/// * `start_pixel_y` - Starting Y coordinate in pixels
/// * `end_pixel_x` - Ending X coordinate in pixels
/// * `end_pixel_y` - Ending Y coordinate in pixels
/// * `scale_factor` - Device scale factor (e.g., 3.0 for iPhone Pro models)
/// * `duration` - Optional duration in seconds (default: 0.5)
pub fn swipe_pixel(
    udid: &str,
    start_pixel_x: f64,
    start_pixel_y: f64,
    end_pixel_x: f64,
    end_pixel_y: f64,
    scale_factor: f64,
    duration: Option<f64>,
) -> Result<()> {
    let start_x = pixel_to_point(start_pixel_x, scale_factor) as f64;
    let start_y = pixel_to_point(start_pixel_y, scale_factor) as f64;
    let end_x = pixel_to_point(end_pixel_x, scale_factor) as f64;
    let end_y = pixel_to_point(end_pixel_y, scale_factor) as f64;
    swipe(udid, start_x, start_y, end_x, end_y, duration)
}

/// Input text into the focused text field
///
/// # Arguments
/// * `udid` - The device UDID
/// * `text` - The text to input
pub fn text_input(udid: &str, text: &str) -> Result<()> {
    run_idb(&["ui", "text", "--udid", udid, text])?;
    Ok(())
}

/// Press a hardware button
///
/// # Arguments
/// * `udid` - The device UDID
/// * `button` - The button name (e.g., "HOME", "LOCK", "SIRI", "APPLE_PAY")
pub fn press_button(udid: &str, button: &str) -> Result<()> {
    run_idb(&["ui", "button", "--udid", udid, button])?;
    Ok(())
}

/// Send a key event to a simulator device
///
/// # Arguments
/// * `udid` - The device UDID
/// * `key` - The key code or key name to send
///
/// # Supported Keys
/// Key codes (integers) or special key names supported by IDB.
/// Use `press_button` for hardware buttons like HOME, LOCK.
pub fn send_key(udid: &str, key: &str) -> Result<()> {
    run_idb(&["ui", "key", "--udid", udid, key])?;
    Ok(())
}

/// Long press on a specific coordinate
///
/// # Arguments
/// * `udid` - The device UDID
/// * `x` - X coordinate (in logical points)
/// * `y` - Y coordinate (in logical points)
/// * `duration` - Press duration in seconds
pub fn long_press(udid: &str, x: f64, y: f64, duration: f64) -> Result<()> {
    if x < 0.0 || y < 0.0 {
        return Err(IdbError::InvalidCoordinates(format!(
            "Coordinates must be non-negative: ({}, {})",
            x, y
        )));
    }

    // Apply orientation-based coordinate transformation
    let info = super::orientation::detect_orientation(udid);
    let (tx, ty) = super::orientation::transform_coordinates(
        x, y, info.screen_width, info.screen_height, info.orientation,
    );

    let x_int = tx.round() as i64;
    let y_int = ty.round() as i64;

    run_idb(&[
        "ui",
        "tap",
        "--udid",
        udid,
        "--duration",
        &duration.to_string(),
        &x_int.to_string(),
        &y_int.to_string(),
    ])?;
    Ok(())
}

/// Retrieve the UI element tree from the simulator using `idb ui describe-all`
///
/// # Arguments
/// * `udid` - The device UDID
///
/// # Returns
/// JSON string of the UI element tree
pub fn describe_all(udid: &str) -> Result<String> {
    run_idb(&["ui", "describe-all", "--udid", udid])
}

/// Compute a lightweight hash of the given string data
///
/// Uses `DefaultHasher` to avoid adding external crate dependencies.
pub fn compute_state_hash(data: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

/// Fixed jitter offsets for retry attempts (x_offset, y_offset)
pub const JITTER_OFFSETS: [(f64, f64); 3] = [(3.0, 0.0), (0.0, 3.0), (-3.0, -3.0)];

/// Maximum number of retry attempts when tap doesn't cause UI change
pub const MAX_RETRIES: usize = 3;

/// Tap with automatic retry if the UI state doesn't change
///
/// Compares UI state (via `idb ui describe-all`) before and after the tap.
/// If no change is detected, retries with small coordinate jitter.
/// Screenshots are captured based on the observation policy in `config`.
///
/// # Arguments
/// * `udid` - The device UDID
/// * `x` - X coordinate (in logical points)
/// * `y` - Y coordinate (in logical points)
/// * `config` - Capture configuration controlling screenshot observation
pub fn tap_with_retry(udid: &str, x: f64, y: f64, config: &CaptureConfig) -> Result<()> {
    if x < 0.0 || y < 0.0 {
        return Err(IdbError::InvalidCoordinates(format!(
            "Coordinates must be non-negative: ({}, {})",
            x, y
        )));
    }

    // Detect orientation once and reuse for all attempts
    let info = super::orientation::detect_orientation(udid);
    let (tx, ty) = super::orientation::transform_coordinates(
        x, y, info.screen_width, info.screen_height, info.orientation,
    );

    // Capture pre-tap screenshot (Always policy only)
    if config.policy == ObservationPolicy::Always {
        capture::observe(udid, config, "pre");
    }

    // Get pre-tap UI state hash
    let pre_hash = match describe_all(udid) {
        Ok(output) => Some(compute_state_hash(&output)),
        Err(e) => {
            eprintln!(
                "[tap_with_retry] Warning: failed to get pre-tap UI state: {}. Falling back to simple tap.",
                e
            );
            return tap_raw(udid, tx, ty);
        }
    };

    // First tap attempt at transformed coordinates
    eprintln!(
        "[tap_with_retry] Attempt 1/{}: tap at ({}, {})",
        MAX_RETRIES + 1,
        x,
        y
    );
    tap_raw(udid, tx, ty)?;

    // Brief pause for UI to settle
    thread::sleep(Duration::from_millis(300));

    // Get post-tap UI state hash
    let post_hash = match describe_all(udid) {
        Ok(output) => compute_state_hash(&output),
        Err(e) => {
            eprintln!(
                "[tap_with_retry] Warning: failed to get post-tap UI state: {}. Assuming success.",
                e
            );
            return Ok(());
        }
    };

    if pre_hash != Some(post_hash) {
        eprintln!("[tap_with_retry] UI state changed after attempt 1. Success.");
        if config.policy == ObservationPolicy::Always {
            capture::observe(udid, config, "success_1");
        }
        return Ok(());
    }

    eprintln!("[tap_with_retry] UI state unchanged after attempt 1. Retrying with jitter...");

    // Capture screenshot on failure (OnFailure or Always)
    if config.policy != ObservationPolicy::Never {
        capture::observe(udid, config, "fail_1");
    }

    // Retry with jitter offsets (applied to already-transformed coordinates)
    let mut last_hash = post_hash;
    for (i, (dx, dy)) in JITTER_OFFSETS.iter().enumerate() {
        let jittered_x = (tx + dx).max(0.0);
        let jittered_y = (ty + dy).max(0.0);

        eprintln!(
            "[tap_with_retry] Attempt {}/{}: tap at ({}, {}) [jitter: ({:+}, {:+})]",
            i + 2,
            MAX_RETRIES + 1,
            jittered_x,
            jittered_y,
            dx,
            dy
        );

        tap_raw(udid, jittered_x, jittered_y)?;
        thread::sleep(Duration::from_millis(300));

        match describe_all(udid) {
            Ok(output) => {
                let new_hash = compute_state_hash(&output);
                if new_hash != last_hash {
                    eprintln!(
                        "[tap_with_retry] UI state changed after attempt {}. Success.",
                        i + 2
                    );
                    if config.policy == ObservationPolicy::Always {
                        capture::observe(udid, config, &format!("success_{}", i + 2));
                    }
                    return Ok(());
                }
                last_hash = new_hash;
            }
            Err(e) => {
                eprintln!(
                    "[tap_with_retry] Warning: failed to get UI state on retry {}: {}. Assuming success.",
                    i + 2,
                    e
                );
                return Ok(());
            }
        }

        eprintln!(
            "[tap_with_retry] UI state unchanged after attempt {}.",
            i + 2
        );

        // Capture screenshot on each failed retry (OnFailure or Always)
        if config.policy != ObservationPolicy::Never {
            capture::observe(udid, config, &format!("fail_{}", i + 2));
        }
    }

    eprintln!(
        "[tap_with_retry] All {} attempts exhausted. UI state did not change.",
        MAX_RETRIES + 1
    );
    Ok(())
}

/// Deterministic jitter offset as fraction of region dimensions (within ±10%)
const REGION_JITTER: (f64, f64) = (0.05, -0.03);

/// Tap within a rectangular region on the simulator screen
///
/// Computes the center of the region and applies a small deterministic jitter
/// (within 10% of region dimensions) to simulate more natural tap behavior.
/// The tap point is clamped to stay within the region bounds.
///
/// # Arguments
/// * `udid` - The device UDID
/// * `x` - Left edge X coordinate (in logical points)
/// * `y` - Top edge Y coordinate (in logical points)
/// * `width` - Region width (in logical points)
/// * `height` - Region height (in logical points)
/// * `no_retry` - If true, perform a single tap; if false, use tap_with_retry
/// * `config` - Capture configuration for screenshot observation
pub fn tap_region(
    udid: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    no_retry: bool,
    config: &CaptureConfig,
) -> Result<()> {
    if x < 0.0 || y < 0.0 || width <= 0.0 || height <= 0.0 {
        return Err(IdbError::InvalidCoordinates(format!(
            "Region must have non-negative origin and positive dimensions: ({}, {}, {}x{})",
            x, y, width, height
        )));
    }

    // Compute center of region
    let center_x = x + width / 2.0;
    let center_y = y + height / 2.0;

    // Apply deterministic jitter within 10% of region dimensions
    let jitter_x = width * REGION_JITTER.0;
    let jitter_y = height * REGION_JITTER.1;

    // Clamp to region bounds
    let tap_x = (center_x + jitter_x).clamp(x, x + width);
    let tap_y = (center_y + jitter_y).clamp(y, y + height);

    eprintln!(
        "[tap_region] Region ({}, {}, {}x{}), center ({}, {}), jittered tap at ({:.1}, {:.1})",
        x, y, width, height, center_x, center_y, tap_x, tap_y
    );

    if no_retry {
        tap(udid, tap_x, tap_y)
    } else {
        tap_with_retry(udid, tap_x, tap_y, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available() {
        // Just check that the function runs without panicking
        let _ = is_available();
    }

    #[test]
    fn test_invalid_coordinates() {
        // These should fail with InvalidCoordinates, not attempt to run idb
        let result = tap("fake-udid", -1.0, 100.0);
        assert!(matches!(result, Err(IdbError::InvalidCoordinates(_))));

        let result = swipe("fake-udid", -1.0, 0.0, 100.0, 100.0, None);
        assert!(matches!(result, Err(IdbError::InvalidCoordinates(_))));
    }

    #[test]
    fn test_pixel_to_point_conversion() {
        // iPhone Pro 3x scale: 1260 pixels -> 420 points
        assert_eq!(pixel_to_point(1260.0, 3.0), 420);
        assert_eq!(pixel_to_point(2736.0, 3.0), 912);

        // iPhone SE 2x scale: 750 pixels -> 375 points
        assert_eq!(pixel_to_point(750.0, 2.0), 375);

        // 1x scale (no conversion)
        assert_eq!(pixel_to_point(100.0, 1.0), 100);

        // Test rounding: 455 / 3 = 151.666... -> 152
        assert_eq!(pixel_to_point(455.0, 3.0), 152);

        // Test rounding: 454 / 3 = 151.333... -> 151
        assert_eq!(pixel_to_point(454.0, 3.0), 151);
    }

    #[test]
    fn test_compute_state_hash_deterministic() {
        let hash1 = compute_state_hash("hello world");
        let hash2 = compute_state_hash("hello world");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_state_hash_different_inputs() {
        let hash1 = compute_state_hash("state A");
        let hash2 = compute_state_hash("state B");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_jitter_offsets_count() {
        assert_eq!(JITTER_OFFSETS.len(), MAX_RETRIES);
    }

    #[test]
    fn test_region_center_calculation() {
        let x = 100.0;
        let y = 200.0;
        let width = 50.0;
        let height = 40.0;

        let center_x = x + width / 2.0;
        let center_y = y + height / 2.0;
        assert_eq!(center_x, 125.0);
        assert_eq!(center_y, 220.0);

        // Jittered point should stay within region
        let jitter_x = width * REGION_JITTER.0;
        let jitter_y = height * REGION_JITTER.1;
        let tap_x = (center_x + jitter_x).clamp(x, x + width);
        let tap_y = (center_y + jitter_y).clamp(y, y + height);

        assert!(tap_x >= x && tap_x <= x + width);
        assert!(tap_y >= y && tap_y <= y + height);
    }

    #[test]
    fn test_region_jitter_within_bounds() {
        // Even with various region sizes, jitter should stay within bounds
        for &(w, h) in &[(1.0, 1.0), (1000.0, 1000.0), (10.0, 5.0)] {
            let x = 0.0;
            let y = 0.0;
            let center_x = x + w / 2.0;
            let center_y = y + h / 2.0;
            let jitter_x = w * REGION_JITTER.0;
            let jitter_y = h * REGION_JITTER.1;
            let tap_x = (center_x + jitter_x).clamp(x, x + w);
            let tap_y = (center_y + jitter_y).clamp(y, y + h);

            assert!(tap_x >= x && tap_x <= x + w, "x out of bounds for {}x{}", w, h);
            assert!(tap_y >= y && tap_y <= y + h, "y out of bounds for {}x{}", w, h);
        }
    }

    #[test]
    fn test_region_invalid_dimensions() {
        let config = CaptureConfig::new(ObservationPolicy::Never, false, String::new());

        // Negative origin
        let result = tap_region("fake-udid", -1.0, 0.0, 10.0, 10.0, true, &config);
        assert!(matches!(result, Err(IdbError::InvalidCoordinates(_))));

        // Zero width
        let result = tap_region("fake-udid", 0.0, 0.0, 0.0, 10.0, true, &config);
        assert!(matches!(result, Err(IdbError::InvalidCoordinates(_))));

        // Negative height
        let result = tap_region("fake-udid", 0.0, 0.0, 10.0, -5.0, true, &config);
        assert!(matches!(result, Err(IdbError::InvalidCoordinates(_))));
    }

    #[test]
    fn test_region_jitter_is_within_10_percent() {
        // REGION_JITTER offsets should each be within ±10% (0.1)
        assert!(REGION_JITTER.0.abs() <= 0.1);
        assert!(REGION_JITTER.1.abs() <= 0.1);
    }
}
