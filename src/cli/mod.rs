// CLI module - Sub1担当

pub mod commands;

use std::path::Path;

use clap::Parser;
use commands::{Cli, Commands, KeyType};

use crate::simulator::{idb, operations};

pub fn run() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::List { booted } => handle_list(booted),
        Commands::Boot { device } => handle_boot(&device),
        Commands::Shutdown { device } => handle_shutdown(&device),
        Commands::Install { device, app_path } => handle_install(&device, &app_path),
        Commands::Launch { device, bundle_id } => handle_launch(&device, &bundle_id),
        Commands::Screenshot { device, output } => handle_screenshot(&device, &output),
        Commands::Input { device, key } => handle_input(&device, key),
        Commands::Openurl { device, url } => handle_openurl(&device, &url),
        Commands::Tap { device, x, y, scale } => handle_tap(&device, x, y, scale),
        Commands::Swipe {
            device,
            start_x,
            start_y,
            end_x,
            end_y,
            scale,
        } => handle_swipe(&device, start_x, start_y, end_x, end_y, scale),
        Commands::Enumerate { device } => handle_enumerate(&device),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn handle_list(booted_only: bool) -> Result<(), Box<dyn std::error::Error>> {
    let devices = operations::list_devices()?;

    let filtered: Vec<_> = if booted_only {
        devices.into_iter().filter(|d| d.device.is_booted()).collect()
    } else {
        devices
    };

    if filtered.is_empty() {
        if booted_only {
            println!("No booted simulators found.");
        } else {
            println!("No simulators found.");
        }
        return Ok(());
    }

    for device_info in filtered {
        let status = if device_info.device.is_booted() {
            "Booted"
        } else {
            "Shutdown"
        };
        println!(
            "{} - {} ({}) [{}]",
            device_info.device.udid, device_info.device.name, device_info.runtime_name, status
        );
    }

    Ok(())
}

fn handle_boot(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    operations::boot(device)?;
    println!("Booted simulator: {}", device);
    Ok(())
}

fn handle_shutdown(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    if device == "all" {
        operations::shutdown_all()?;
        println!("Shutdown all simulators.");
    } else {
        operations::shutdown(device)?;
        println!("Shutdown simulator: {}", device);
    }
    Ok(())
}

fn handle_install(device: &str, app_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(app_path);
    operations::install_app(device, path)?;
    println!("Installed {} on {}", app_path, device);
    Ok(())
}

fn handle_launch(device: &str, bundle_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    operations::launch_app(device, bundle_id, None)?;
    println!("Launched {} on {}", bundle_id, device);
    Ok(())
}

fn handle_screenshot(device: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(output);
    operations::take_screenshot(device, path)?;
    println!("Screenshot saved to: {}", output);
    Ok(())
}

fn handle_input(device: &str, key: KeyType) -> Result<(), Box<dyn std::error::Error>> {
    let key_str = match key {
        KeyType::Home => "HOME",
        KeyType::Lock => "LOCK",
        KeyType::Return => "RETURN",
        KeyType::VolumeUp => "VOLUME_UP",
        KeyType::VolumeDown => "VOLUME_DOWN",
        KeyType::Shake => "SHAKE",
    };

    idb::press_button(device, key_str)?;
    println!("Sent {} key to {}", key_str, device);
    Ok(())
}

fn handle_openurl(device: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    operations::open_url(device, url)?;
    println!("Opened URL: {}", url);
    Ok(())
}

fn handle_tap(device: &str, x: u32, y: u32, scale: Option<u32>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(scale_factor) = scale {
        // Pixel coordinates - convert to logical points
        idb::tap_pixel(device, f64::from(x), f64::from(y), f64::from(scale_factor))?;
        let point_x = f64::from(x) / f64::from(scale_factor);
        let point_y = f64::from(y) / f64::from(scale_factor);
        println!(
            "Tapped at pixel ({}, {}) -> point ({:.1}, {:.1}) on {} (scale: {}x)",
            x, y, point_x, point_y, device, scale_factor
        );
    } else {
        // Logical point coordinates
        idb::tap(device, f64::from(x), f64::from(y))?;
        println!("Tapped at point ({}, {}) on {}", x, y, device);
    }
    Ok(())
}

fn handle_swipe(
    device: &str,
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    scale: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(scale_factor) = scale {
        // Pixel coordinates - convert to logical points
        idb::swipe_pixel(
            device,
            f64::from(start_x),
            f64::from(start_y),
            f64::from(end_x),
            f64::from(end_y),
            f64::from(scale_factor),
            None,
        )?;
        println!(
            "Swiped from pixel ({}, {}) to ({}, {}) on {} (scale: {}x)",
            start_x, start_y, end_x, end_y, device, scale_factor
        );
    } else {
        // Logical point coordinates
        idb::swipe(
            device,
            f64::from(start_x),
            f64::from(start_y),
            f64::from(end_x),
            f64::from(end_y),
            None,
        )?;
        println!(
            "Swiped from point ({}, {}) to ({}, {}) on {}",
            start_x, start_y, end_x, end_y, device
        );
    }
    Ok(())
}

fn handle_enumerate(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = operations::enumerate_devices(device)?;
    println!("{}", output);
    Ok(())
}
