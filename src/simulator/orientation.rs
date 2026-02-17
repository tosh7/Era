// Screen orientation detection and coordinate transformation for iOS Simulator

use serde::Deserialize;
use std::process::Command;

/// Screen orientation of the iOS Simulator
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
    Portrait,
    LandscapeLeft,
    LandscapeRight,
    UpsideDown,
}

/// Orientation detection result with screen dimensions (Portrait basis)
pub struct OrientationInfo {
    pub orientation: Orientation,
    /// Screen width in Portrait mode (logical points)
    pub screen_width: f64,
    /// Screen height in Portrait mode (logical points)
    pub screen_height: f64,
}

/// Detect the current screen orientation of a simulator device.
/// Falls back to Portrait (no transformation) if detection fails.
///
/// # Limitations
/// - The current implementation cannot distinguish between `LandscapeLeft` and
///   `LandscapeRight`. When landscape is detected (width > height), it is
///   always reported as `LandscapeLeft`.
/// - `UpsideDown` is never detected by the current heuristic.
/// - These limitations may be addressed in a future version if a reliable
///   detection method becomes available.
pub fn detect_orientation(udid: &str) -> OrientationInfo {
    match try_detect_via_idb(udid) {
        Ok(info) => info,
        Err(e) => {
            eprintln!(
                "[era] Warning: orientation detection failed: {}. Falling back to Portrait.",
                e
            );
            // Fallback uses 0.0 for screen dimensions. This is safe because
            // Portrait orientation returns (x, y) unchanged in
            // transform_coordinates, so screen_width/screen_height are never
            // used in the calculation.
            OrientationInfo {
                orientation: Orientation::Portrait,
                screen_width: 0.0,
                screen_height: 0.0,
            }
        }
    }
}

/// Represents the frame dimensions from idb's UI element tree.
/// Only `width` and `height` are used; `x` and `y` fields present in the
/// JSON output are intentionally ignored as they are not needed for
/// orientation detection.
#[derive(Deserialize)]
struct Frame {
    width: f64,
    height: f64,
}

fn try_detect_via_idb(udid: &str) -> Result<OrientationInfo, String> {
    let output = Command::new("idb")
        .args(["ui", "describe-all", "--udid", udid])
        .output()
        .map_err(|e| format!("Failed to run idb: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("idb command failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let root: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse idb output: {}", e))?;

    let frame = extract_root_frame(&root)?;

    let (orientation, portrait_width, portrait_height) = if frame.width > frame.height {
        // Landscape detected: swap to get portrait dimensions
        (Orientation::LandscapeLeft, frame.height, frame.width)
    } else {
        (Orientation::Portrait, frame.width, frame.height)
    };

    Ok(OrientationInfo {
        orientation,
        screen_width: portrait_width,
        screen_height: portrait_height,
    })
}

fn extract_root_frame(value: &serde_json::Value) -> Result<Frame, String> {
    let root = if value.is_array() {
        value.get(0).ok_or("Empty UI tree")?
    } else {
        value
    };

    let frame_val = root.get("frame").ok_or("No frame in root element")?;
    let frame: Frame =
        serde_json::from_value(frame_val.clone()).map_err(|e| format!("Invalid frame: {}", e))?;

    if frame.width <= 0.0 || frame.height <= 0.0 {
        return Err(format!(
            "Invalid frame dimensions: {}x{}",
            frame.width, frame.height
        ));
    }

    Ok(frame)
}

/// Transform coordinates based on screen orientation.
///
/// Input coordinates are in the current orientation's coordinate system.
/// Output coordinates are in idb's expected coordinate system (Portrait).
///
/// # Arguments
/// * `x` - X coordinate in current orientation
/// * `y` - Y coordinate in current orientation
/// * `screen_width` - Screen width in Portrait mode
/// * `screen_height` - Screen height in Portrait mode
/// * `orientation` - Current screen orientation
pub fn transform_coordinates(
    x: f64,
    y: f64,
    screen_width: f64,
    screen_height: f64,
    orientation: Orientation,
) -> (f64, f64) {
    match orientation {
        Orientation::Portrait => (x, y),
        Orientation::LandscapeLeft => (y, screen_width - x),
        Orientation::LandscapeRight => (screen_height - y, x),
        Orientation::UpsideDown => (screen_width - x, screen_height - y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_portrait() {
        let (x, y) = transform_coordinates(100.0, 200.0, 390.0, 844.0, Orientation::Portrait);
        assert_eq!(x, 100.0);
        assert_eq!(y, 200.0);
    }

    #[test]
    fn test_transform_landscape_left() {
        let (x, y) = transform_coordinates(100.0, 200.0, 390.0, 844.0, Orientation::LandscapeLeft);
        assert_eq!(x, 200.0);
        assert_eq!(y, 290.0); // 390 - 100
    }

    #[test]
    fn test_transform_landscape_right() {
        let (x, y) =
            transform_coordinates(100.0, 200.0, 390.0, 844.0, Orientation::LandscapeRight);
        assert_eq!(x, 644.0); // 844 - 200
        assert_eq!(y, 100.0);
    }

    #[test]
    fn test_transform_upside_down() {
        let (x, y) = transform_coordinates(100.0, 200.0, 390.0, 844.0, Orientation::UpsideDown);
        assert_eq!(x, 290.0); // 390 - 100
        assert_eq!(y, 644.0); // 844 - 200
    }

    #[test]
    fn test_detect_orientation_fallback() {
        // With a fake UDID, detection should fail and fall back to Portrait
        let info = detect_orientation("fake-udid-for-test");
        assert_eq!(info.orientation, Orientation::Portrait);
    }

    #[test]
    fn test_extract_root_frame_object() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"frame": {"x": 0, "y": 0, "width": 393, "height": 852}}"#,
        )
        .unwrap();
        let frame = extract_root_frame(&json).unwrap();
        assert_eq!(frame.width, 393.0);
        assert_eq!(frame.height, 852.0);
    }

    #[test]
    fn test_extract_root_frame_array() {
        let json: serde_json::Value = serde_json::from_str(
            r#"[{"frame": {"x": 0, "y": 0, "width": 852, "height": 393}}]"#,
        )
        .unwrap();
        let frame = extract_root_frame(&json).unwrap();
        assert_eq!(frame.width, 852.0);
        assert_eq!(frame.height, 393.0);
    }

    #[test]
    fn test_extract_root_frame_invalid() {
        let json: serde_json::Value = serde_json::from_str(r#"{"no_frame": true}"#).unwrap();
        assert!(extract_root_frame(&json).is_err());
    }
}
