// Simulator module - iOS Simulator operations using xcrun simctl

pub mod device;
pub mod idb;
pub mod operations;
pub mod orientation;
pub mod session;
pub mod snapshot;
pub mod ui_tree;
pub mod wait;

// Re-export commonly used types
pub use device::{Device, DeviceInfo, DeviceScaleFactor, DeviceState, DeviceType, Runtime, SimulatorList};
pub use operations::{
    boot, detect_device_scale, enumerate_devices, get_booted_device, get_simulator_list,
    install_app, launch_app, list_devices, open_url, send_key, shutdown, shutdown_all,
    take_screenshot, terminate_app, uninstall_app, Result, SimulatorError,
};
