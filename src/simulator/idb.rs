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

/// Tap on a specific coordinate on the simulator screen
///
/// # Arguments
/// * `udid` - The device UDID
/// * `x` - X coordinate (in points)
/// * `y` - Y coordinate (in points)
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

/// Perform a swipe gesture on the simulator screen
///
/// # Arguments
/// * `udid` - The device UDID
/// * `start_x` - Starting X coordinate
/// * `start_y` - Starting Y coordinate
/// * `end_x` - Ending X coordinate
/// * `end_y` - Ending Y coordinate
/// * `duration` - Optional duration in seconds (default: 0.5)
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
}
