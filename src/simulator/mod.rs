// Simulator module - iOS Simulator operations using xcrun simctl

pub mod device;
pub mod idb;
pub mod operations;

// Re-export commonly used types
pub use device::{Device, DeviceInfo, DeviceState, DeviceType, Runtime, SimulatorList};
pub use operations::{
    boot, enumerate_devices, get_booted_device, get_simulator_list, install_app, launch_app,
    list_devices, open_url, send_key, shutdown, shutdown_all, take_screenshot, terminate_app,
    uninstall_app, Result, SimulatorError,
};
