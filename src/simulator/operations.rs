// Simulator operations - simctl wrapper functions
use std::path::Path;
use std::process::Command;
use thiserror::Error;

use super::device::{DeviceInfo, DeviceScaleFactor, SimulatorList};

/// Errors that can occur during simulator operations
#[derive(Debug, Error)]
pub enum SimulatorError {
    #[error("Failed to execute simctl command: {0}")]
    CommandExecution(#[from] std::io::Error),

    #[error("simctl command failed with status {status}: {stderr}")]
    CommandFailed { status: i32, stderr: String },

    #[error("Failed to parse simctl JSON output: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("App not found at path: {0}")]
    AppNotFound(String),

    #[error("Screenshot failed: {0}")]
    ScreenshotFailed(String),
}

pub type Result<T> = std::result::Result<T, SimulatorError>;

/// Execute a simctl command and return the output
fn run_simctl(args: &[&str]) -> Result<String> {
    let output = Command::new("xcrun")
        .arg("simctl")
        .args(args)
        .output()?;

    if !output.status.success() {
        let status = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(SimulatorError::CommandFailed { status, stderr });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// List all simulator devices with their runtime information
pub fn list_devices() -> Result<Vec<DeviceInfo>> {
    let output = run_simctl(&["list", "-j"])?;
    let simulator_list: SimulatorList = serde_json::from_str(&output)?;

    let mut device_infos = Vec::new();

    // Create a lookup map for runtime names
    let runtime_names: std::collections::HashMap<&str, &str> = simulator_list
        .runtimes
        .iter()
        .map(|r| (r.identifier.as_str(), r.name.as_str()))
        .collect();

    // Flatten devices from all runtimes
    for (runtime_id, devices) in &simulator_list.devices {
        let runtime_name = runtime_names
            .get(runtime_id.as_str())
            .copied()
            .unwrap_or(runtime_id.as_str());

        for device in devices {
            device_infos.push(DeviceInfo::new(
                device.clone(),
                runtime_id.clone(),
                runtime_name.to_string(),
            ));
        }
    }

    Ok(device_infos)
}

/// Get the full simulator list including device types and runtimes
pub fn get_simulator_list() -> Result<SimulatorList> {
    let output = run_simctl(&["list", "-j"])?;
    let simulator_list: SimulatorList = serde_json::from_str(&output)?;
    Ok(simulator_list)
}

/// Boot a simulator device by UDID
pub fn boot(udid: &str) -> Result<()> {
    run_simctl(&["boot", udid])?;
    Ok(())
}

/// Shutdown a simulator device by UDID
pub fn shutdown(udid: &str) -> Result<()> {
    run_simctl(&["shutdown", udid])?;
    Ok(())
}

/// Shutdown all running simulators
pub fn shutdown_all() -> Result<()> {
    run_simctl(&["shutdown", "all"])?;
    Ok(())
}

/// Install an app on a simulator device
///
/// # Arguments
/// * `udid` - The device UDID (or "booted" for the currently booted device)
/// * `app_path` - Path to the .app bundle
pub fn install_app(udid: &str, app_path: &Path) -> Result<()> {
    if !app_path.exists() {
        return Err(SimulatorError::AppNotFound(
            app_path.display().to_string(),
        ));
    }

    run_simctl(&["install", udid, &app_path.display().to_string()])?;
    Ok(())
}

/// Uninstall an app from a simulator device
///
/// # Arguments
/// * `udid` - The device UDID (or "booted" for the currently booted device)
/// * `bundle_id` - The app's bundle identifier
pub fn uninstall_app(udid: &str, bundle_id: &str) -> Result<()> {
    run_simctl(&["uninstall", udid, bundle_id])?;
    Ok(())
}

/// Launch an app on a simulator device
///
/// # Arguments
/// * `udid` - The device UDID (or "booted" for the currently booted device)
/// * `bundle_id` - The app's bundle identifier
/// * `args` - Optional arguments to pass to the app
pub fn launch_app(udid: &str, bundle_id: &str, args: Option<&[&str]>) -> Result<()> {
    let mut cmd_args = vec!["launch", udid, bundle_id];
    if let Some(extra_args) = args {
        cmd_args.extend(extra_args);
    }
    run_simctl(&cmd_args)?;
    Ok(())
}

/// Terminate a running app on a simulator device
///
/// # Arguments
/// * `udid` - The device UDID (or "booted" for the currently booted device)
/// * `bundle_id` - The app's bundle identifier
pub fn terminate_app(udid: &str, bundle_id: &str) -> Result<()> {
    run_simctl(&["terminate", udid, bundle_id])?;
    Ok(())
}

/// Take a screenshot of a simulator device
///
/// # Arguments
/// * `udid` - The device UDID (or "booted" for the currently booted device)
/// * `output_path` - Path where the screenshot will be saved (PNG format)
pub fn take_screenshot(udid: &str, output_path: &Path) -> Result<()> {
    let output_str = output_path.display().to_string();

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SimulatorError::ScreenshotFailed(format!(
                    "Failed to create output directory: {}",
                    e
                ))
            })?;
        }
    }

    run_simctl(&["io", udid, "screenshot", &output_str])?;
    Ok(())
}

/// Open a URL on a simulator device
///
/// # Arguments
/// * `udid` - The device UDID (or "booted" for the currently booted device)
/// * `url` - The URL to open
pub fn open_url(udid: &str, url: &str) -> Result<()> {
    run_simctl(&["openurl", udid, url])?;
    Ok(())
}

/// Send a key event to a simulator device
///
/// **DEPRECATED**: This function requires IDB (iOS Development Bridge).
/// Use `crate::simulator::idb::send_key` or `crate::simulator::idb::press_button` instead.
///
/// simctl does not support key input directly. Install IDB with:
/// `brew install idb-companion`
///
/// # Arguments
/// * `_udid` - The device UDID (unused, for API compatibility)
/// * `_key` - The key to send (unused, for API compatibility)
#[deprecated(
    since = "0.1.0",
    note = "simctl does not support key input. Use idb::send_key or idb::press_button instead."
)]
pub fn send_key(_udid: &str, _key: &str) -> Result<()> {
    Err(SimulatorError::CommandFailed {
        status: 1,
        stderr: "Key input requires IDB. Install with: brew install idb-companion. Use idb::send_key or idb::press_button instead.".to_string(),
    })
}

/// Enumerate I/O devices available on a simulator
///
/// # Arguments
/// * `udid` - The device UDID (or "booted" for the currently booted device)
///
/// # Returns
/// Raw output from simctl io enumerate command
pub fn enumerate_devices(udid: &str) -> Result<String> {
    run_simctl(&["io", udid, "enumerate"])
}

/// Get the booted device UDID, if any
pub fn get_booted_device() -> Result<Option<DeviceInfo>> {
    let devices = list_devices()?;
    Ok(devices.into_iter().find(|d| d.device.is_booted()))
}

/// Detect the scale factor for a device by its UDID
///
/// Looks up the device in the simulator list, finds its device type,
/// and infers the scale factor from the device type name.
/// Falls back to 2x if the device or device type cannot be determined.
pub fn detect_device_scale(udid: &str) -> Result<DeviceScaleFactor> {
    let sim_list = get_simulator_list()?;

    // Find the device and its device_type_identifier
    let device_type_id = sim_list
        .devices
        .values()
        .flatten()
        .find(|d| d.udid == udid)
        .and_then(|d| d.device_type_identifier.clone());

    let Some(type_id) = device_type_id else {
        log::warn!(
            "Could not find device type for UDID {}. Defaulting to 2x scale.",
            udid
        );
        return Ok(DeviceScaleFactor::X2);
    };

    // Look up the device type name from devicetypes
    let device_type_name = sim_list
        .devicetypes
        .iter()
        .find(|dt| dt.identifier == type_id)
        .map(|dt| dt.name.as_str());

    match device_type_name {
        Some(name) => {
            let scale = DeviceScaleFactor::from_device_name(name);
            log::info!(
                "Detected device type '{}' for UDID {} -> scale {}",
                name,
                udid,
                scale
            );
            Ok(scale)
        }
        None => {
            log::warn!(
                "Device type identifier '{}' not found in device types. Defaulting to 2x scale.",
                type_id
            );
            Ok(DeviceScaleFactor::X2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices() {
        // This test requires Xcode to be installed
        let result = list_devices();
        assert!(result.is_ok(), "list_devices should succeed: {:?}", result.err());
    }

    #[test]
    fn test_get_simulator_list() {
        let result = get_simulator_list();
        assert!(result.is_ok(), "get_simulator_list should succeed");

        let list = result.unwrap();
        assert!(!list.devicetypes.is_empty(), "Should have device types");
        assert!(!list.runtimes.is_empty(), "Should have runtimes");
    }
}
