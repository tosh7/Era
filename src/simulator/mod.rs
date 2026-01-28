// Simulator module - iOS Simulator operations using xcrun simctl

pub mod device;
pub mod operations;

// Re-export commonly used types
pub use device::{Device, DeviceInfo, DeviceState, DeviceType, Runtime, SimulatorList};
pub use operations::{
    boot, get_booted_device, get_simulator_list, install_app, launch_app, list_devices, shutdown,
    shutdown_all, take_screenshot, terminate_app, uninstall_app, open_url, Result, SimulatorError,
};
