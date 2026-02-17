// Device model definitions for simctl JSON output
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root structure for `xcrun simctl list -j` output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorList {
    #[serde(default)]
    pub devicetypes: Vec<DeviceType>,
    #[serde(default)]
    pub runtimes: Vec<Runtime>,
    #[serde(default)]
    pub devices: HashMap<String, Vec<Device>>,
}

/// Device type definition (e.g., iPhone 16 Pro, iPad Air)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceType {
    pub identifier: String,
    pub name: String,
    #[serde(default)]
    pub product_family: Option<String>,
    #[serde(default)]
    pub bundle_path: Option<String>,
    #[serde(default)]
    pub model_identifier: Option<String>,
    #[serde(default)]
    pub min_runtime_version_string: Option<String>,
    #[serde(default)]
    pub max_runtime_version_string: Option<String>,
}

/// Runtime definition (e.g., iOS 18.0, tvOS 18.0)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Runtime {
    pub identifier: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub buildversion: Option<String>,
    #[serde(default)]
    pub is_available: bool,
    #[serde(default)]
    pub is_internal: bool,
    #[serde(default)]
    pub bundle_path: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
}

/// Simulator device state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceState {
    Shutdown,
    Booted,
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for DeviceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceState::Shutdown => write!(f, "Shutdown"),
            DeviceState::Booted => write!(f, "Booted"),
            DeviceState::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Scale factor for iOS devices
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceScaleFactor {
    /// 2x Retina (iPhone SE, iPhone 8, etc.)
    X2,
    /// 3x Retina (iPhone Plus/Max/Pro Max, etc.)
    X3,
}

impl DeviceScaleFactor {
    /// Get the numeric scale value
    pub fn value(self) -> f64 {
        match self {
            DeviceScaleFactor::X2 => 2.0,
            DeviceScaleFactor::X3 => 3.0,
        }
    }

    /// Detect scale factor from device type name
    ///
    /// iPhone Plus/Max/Pro Max models use 3x, others default to 2x.
    /// Known 2x exceptions: iPhone XR, iPad Pro (all generations).
    pub fn from_device_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        let is_iphone = lower.contains("iphone");

        // Explicit 2x exceptions
        if lower.contains("iphone xr") {
            return DeviceScaleFactor::X2;
        }

        // 3x iPhones: Plus, Max, Pro, X/XS, and standard models from iPhone 11+
        if is_iphone
            && (lower.contains("plus")
                || lower.contains("max")
                || lower.contains("pro")
                || lower.contains("iphone xs")
                || lower.contains("iphone x ")
                || lower.ends_with("iphone x")
                || lower.contains("iphone 1")) // iPhone 11, 12, 13, 14, 15, 16...
        {
            DeviceScaleFactor::X3
        } else {
            DeviceScaleFactor::X2
        }
    }
}

impl std::fmt::Display for DeviceScaleFactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceScaleFactor::X2 => write!(f, "2x"),
            DeviceScaleFactor::X3 => write!(f, "3x"),
        }
    }
}

/// Individual simulator device
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub udid: String,
    pub name: String,
    pub state: DeviceState,
    #[serde(default)]
    pub is_available: bool,
    #[serde(default)]
    pub device_type_identifier: Option<String>,
    #[serde(default)]
    pub data_path: Option<String>,
    #[serde(default)]
    pub log_path: Option<String>,
    #[serde(default)]
    pub data_path_size: Option<u64>,
}

impl Device {
    /// Check if the device is currently booted
    pub fn is_booted(&self) -> bool {
        self.state == DeviceState::Booted
    }

    /// Check if the device is available for use
    pub fn is_usable(&self) -> bool {
        self.is_available
    }
}

/// Simplified device info with runtime context
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device: Device,
    pub runtime_identifier: String,
    pub runtime_name: String,
}

impl DeviceInfo {
    pub fn new(device: Device, runtime_identifier: String, runtime_name: String) -> Self {
        Self {
            device,
            runtime_identifier,
            runtime_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_factor_3x_devices() {
        // Pro models
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 16 Pro"), DeviceScaleFactor::X3);
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 16 Pro Max"), DeviceScaleFactor::X3);
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 15 Pro"), DeviceScaleFactor::X3);

        // Plus models
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 15 Plus"), DeviceScaleFactor::X3);
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 14 Plus"), DeviceScaleFactor::X3);

        // iPhone X series
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone X"), DeviceScaleFactor::X3);
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone XS"), DeviceScaleFactor::X3);
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone XS Max"), DeviceScaleFactor::X3);

        // Standard iPhone 11+ (3x)
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 11"), DeviceScaleFactor::X3);
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 12"), DeviceScaleFactor::X3);
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 13"), DeviceScaleFactor::X3);
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 14"), DeviceScaleFactor::X3);
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 15"), DeviceScaleFactor::X3);
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 16"), DeviceScaleFactor::X3);
    }

    #[test]
    fn test_scale_factor_2x_devices() {
        // SE models
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone SE (3rd generation)"), DeviceScaleFactor::X2);
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone SE (2nd generation)"), DeviceScaleFactor::X2);

        // iPhone XR (2x exception)
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone XR"), DeviceScaleFactor::X2);

        // Older models
        assert_eq!(DeviceScaleFactor::from_device_name("iPhone 8"), DeviceScaleFactor::X2);

        // iPad Pro (2x, not 3x despite "Pro" in name)
        assert_eq!(DeviceScaleFactor::from_device_name("iPad Pro (12.9-inch) (6th generation)"), DeviceScaleFactor::X2);
        assert_eq!(DeviceScaleFactor::from_device_name("iPad Pro (11-inch) (4th generation)"), DeviceScaleFactor::X2);

        // Other iPads (default 2x)
        assert_eq!(DeviceScaleFactor::from_device_name("iPad Air (5th generation)"), DeviceScaleFactor::X2);
        assert_eq!(DeviceScaleFactor::from_device_name("iPad mini (6th generation)"), DeviceScaleFactor::X2);
    }

    #[test]
    fn test_scale_factor_value() {
        assert_eq!(DeviceScaleFactor::X2.value(), 2.0);
        assert_eq!(DeviceScaleFactor::X3.value(), 3.0);
    }

    #[test]
    fn test_scale_factor_display() {
        assert_eq!(format!("{}", DeviceScaleFactor::X2), "2x");
        assert_eq!(format!("{}", DeviceScaleFactor::X3), "3x");
    }
}
