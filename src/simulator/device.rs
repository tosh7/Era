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
