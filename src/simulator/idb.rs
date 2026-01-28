// IDB (iOS Development Bridge) integration - Facebook's idb tool wrapper
// IDB is an optional dependency for advanced UI automation features
use std::process::Command;
use thiserror::Error;

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

/// Convert pixel coordinates to logical points
///
/// # Arguments
/// * `pixel` - Coordinate in pixels (from screenshot)
/// * `scale_factor` - Device scale factor (2.0 for 2x, 3.0 for 3x Retina displays)
///
/// # Returns
/// Coordinate in logical points (for IDB)
pub fn pixel_to_point(pixel: f64, scale_factor: f64) -> f64 {
    pixel / scale_factor
}

/// Tap on a specific coordinate on the simulator screen
///
/// # Arguments
/// * `udid` - The device UDID
/// * `x` - X coordinate (in logical points, NOT pixels)
/// * `y` - Y coordinate (in logical points, NOT pixels)
///
/// # Note
/// IDB uses logical points, not pixels. For coordinates from screenshots,
/// use `tap_pixel` or divide by the device's scale factor (2x or 3x).
pub fn tap(udid: &str, x: f64, y: f64) -> Result<()> {
    if x < 0.0 || y < 0.0 {
        return Err(IdbError::InvalidCoordinates(format!(
            "Coordinates must be non-negative: ({}, {})",
            x, y
        )));
    }

    run_idb(&[
        "ui",
        "tap",
        "--udid",
        udid,
        &x.to_string(),
        &y.to_string(),
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
    let x = pixel_to_point(pixel_x, scale_factor);
    let y = pixel_to_point(pixel_y, scale_factor);
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
/// IDB uses logical points, not pixels. For coordinates from screenshots,
/// use `swipe_pixel` or divide by the device's scale factor (2x or 3x).
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

    let duration_str = duration.unwrap_or(0.5).to_string();

    run_idb(&[
        "ui",
        "swipe",
        "--udid",
        udid,
        &start_x.to_string(),
        &start_y.to_string(),
        &end_x.to_string(),
        &end_y.to_string(),
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
    let start_x = pixel_to_point(start_pixel_x, scale_factor);
    let start_y = pixel_to_point(start_pixel_y, scale_factor);
    let end_x = pixel_to_point(end_pixel_x, scale_factor);
    let end_y = pixel_to_point(end_pixel_y, scale_factor);
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
/// * `x` - X coordinate
/// * `y` - Y coordinate
/// * `duration` - Press duration in seconds
pub fn long_press(udid: &str, x: f64, y: f64, duration: f64) -> Result<()> {
    if x < 0.0 || y < 0.0 {
        return Err(IdbError::InvalidCoordinates(format!(
            "Coordinates must be non-negative: ({}, {})",
            x, y
        )));
    }

    run_idb(&[
        "ui",
        "tap",
        "--udid",
        udid,
        "--duration",
        &duration.to_string(),
        &x.to_string(),
        &y.to_string(),
    ])?;
    Ok(())
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
        assert_eq!(pixel_to_point(1260.0, 3.0), 420.0);
        assert_eq!(pixel_to_point(2736.0, 3.0), 912.0);

        // iPhone SE 2x scale: 750 pixels -> 375 points
        assert_eq!(pixel_to_point(750.0, 2.0), 375.0);

        // 1x scale (no conversion)
        assert_eq!(pixel_to_point(100.0, 1.0), 100.0);
    }
}
