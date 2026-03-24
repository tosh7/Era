// CLI module - Sub1担当

pub mod commands;

use std::path::Path;

use clap::Parser;
use commands::{Cli, Commands, KeyType};
use log::{debug, info};

use crate::capture::CaptureConfig;
use crate::simulator::{idb, operations, snapshot, ui_tree};

/// Initialize the logger based on verbosity level
fn init_logger(verbose: u8) {
    let filter = match verbose {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        _ => log::LevelFilter::Debug,
    };

    env_logger::Builder::new()
        .filter_level(filter)
        .format_timestamp_millis()
        .init();
}

/// Resolve the effective scale factor.
/// If `--scale` was provided, use it. Otherwise, auto-detect from the device.
fn resolve_scale(device: &str, explicit_scale: Option<u32>) -> Option<u32> {
    if let Some(s) = explicit_scale {
        info!("Using explicit scale factor: {}x", s);
        return Some(s);
    }

    match operations::detect_device_scale(device) {
        Ok(detected) => {
            info!("Auto-detected scale factor: {} for device {}", detected, device);
            Some(detected.value() as u32)
        }
        Err(e) => {
            log::warn!("Failed to auto-detect scale for device {}: {}. Treating coordinates as logical points.", device, e);
            None
        }
    }
}

pub fn run() {
    let cli = Cli::parse();
    init_logger(cli.verbose);

    let debug_capture = cli.debug_capture;
    let debug_dir = cli.debug_dir.clone();

    let result = match cli.command {
        Commands::List { booted } => handle_list(booted),
        Commands::Boot { device } => handle_boot(&device),
        Commands::Shutdown { device } => handle_shutdown(&device),
        Commands::Install { device, app_path } => handle_install(&device, &app_path),
        Commands::Launch { device, bundle_id } => handle_launch(&device, &bundle_id),
        Commands::Screenshot { device, output } => handle_screenshot(&device, &output),
        Commands::Input { device, key } => handle_input(&device, key),
        Commands::Openurl { device, url } => handle_openurl(&device, &url),
        Commands::Snapshot {
            device,
            show_frames,
            interactive,
            filter,
        } => handle_snapshot(&device, show_frames, interactive, filter),
        Commands::Tap {
            device,
            x,
            y,
            ref_id,
            scale,
            no_retry,
            observe,
        } => {
            let config = CaptureConfig::new(observe, debug_capture, debug_dir.clone());
            handle_tap(&device, x, y, ref_id, scale, no_retry, &config)
        }
        Commands::Fill {
            device,
            ref_id,
            text,
            clear,
            no_retry,
            observe,
        } => {
            let config = CaptureConfig::new(observe, debug_capture, debug_dir.clone());
            handle_fill(&device, ref_id, &text, clear, no_retry, &config)
        }
        Commands::TapRegion {
            device,
            x,
            y,
            width,
            height,
            scale,
            no_retry,
            observe,
        } => {
            let config = CaptureConfig::new(observe, debug_capture, debug_dir.clone());
            handle_tap_region(&device, x, y, width, height, scale, no_retry, &config)
        }
        Commands::Swipe {
            device,
            start_x,
            start_y,
            end_x,
            end_y,
            scale,
        } => handle_swipe(&device, start_x, start_y, end_x, end_y, scale),
        Commands::Enumerate { device } => handle_enumerate(&device),
        Commands::Text { device, text } => handle_text(&device, &text),
        Commands::Describe { device } => handle_describe(&device),
        Commands::Longpress {
            device,
            x,
            y,
            duration,
            scale,
        } => handle_longpress(&device, x, y, duration, scale),
        Commands::Key { device, key } => handle_key(&device, &key),
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

fn handle_snapshot(
    device: &str,
    show_frames: bool,
    interactive: bool,
    filter: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Taking snapshot of device: {}", device);

    let json_output = idb::describe_all(device)?;
    let elements = ui_tree::parse(&json_output)?;

    let options = snapshot::SnapshotOptions {
        verbose: show_frames,
        interactive_only: interactive,
        filter_type: filter,
    };

    let (output, ref_map) = snapshot::build_snapshot(&elements, &options);
    snapshot::save_ref_map(device, &ref_map)?;

    let count = ref_map.entries.len();
    print!("{}", output);
    eprintln!(
        "[era] Snapshot: {} elements indexed. Ref map saved to {}",
        count,
        snapshot::ref_map_path(device)
    );

    Ok(())
}

fn handle_tap(
    device: &str,
    x: Option<u32>,
    y: Option<u32>,
    ref_id: Option<u32>,
    scale: Option<u32>,
    no_retry: bool,
    config: &CaptureConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ref-based tap
    if let Some(ref_id) = ref_id {
        let (point_x, point_y) = snapshot::resolve_ref(device, ref_id)?;

        if no_retry {
            idb::tap(device, point_x, point_y)?;
        } else {
            idb::tap_with_retry(device, point_x, point_y, config)?;
        }

        info!(
            "Tap success: ref [{}] -> point ({:.1}, {:.1}), retry={}",
            ref_id, point_x, point_y, !no_retry
        );
        println!(
            "Tapped ref [{}] at point ({:.1}, {:.1}) on {}{}",
            ref_id,
            point_x,
            point_y,
            device,
            if no_retry { "" } else { " (retry enabled)" }
        );
        return Ok(());
    }

    // Coordinate-based tap (original behavior)
    let x = x.expect("x is required when --ref is not set");
    let y = y.expect("y is required when --ref is not set");
    let effective_scale = resolve_scale(device, scale);

    let (point_x, point_y) = if let Some(scale_factor) = effective_scale {
        let px = f64::from(x) / f64::from(scale_factor);
        let py = f64::from(y) / f64::from(scale_factor);
        debug!(
            "Tap: pixel ({}, {}), scale {}x -> point ({:.1}, {:.1}), device {}",
            x, y, scale_factor, px, py, device
        );
        (px, py)
    } else {
        debug!("Tap: point ({}, {}), device {}", x, y, device);
        (f64::from(x), f64::from(y))
    };

    if no_retry {
        idb::tap(device, point_x, point_y)?;
    } else {
        idb::tap_with_retry(device, point_x, point_y, config)?;
    }

    // Print result
    if let Some(scale_factor) = effective_scale {
        info!(
            "Tap success: pixel ({}, {}) -> point ({:.1}, {:.1}), scale {}x, retry={}",
            x, y, point_x, point_y, scale_factor, !no_retry
        );
        println!(
            "Tapped at pixel ({}, {}) -> point ({:.1}, {:.1}) on {} (scale: {}x{}{})",
            x, y, point_x, point_y, device, scale_factor,
            if scale.is_none() { " auto-detected" } else { "" },
            if no_retry { "" } else { ", retry enabled" }
        );
    } else {
        info!("Tap success: point ({}, {}), retry={}", x, y, !no_retry);
        println!(
            "Tapped at point ({}, {}) on {}{}",
            x, y, device,
            if no_retry { "" } else { " (retry enabled)" }
        );
    }
    Ok(())
}

fn handle_fill(
    device: &str,
    ref_id: u32,
    text: &str,
    clear: bool,
    no_retry: bool,
    config: &CaptureConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let (point_x, point_y) = snapshot::resolve_ref(device, ref_id)?;

    // Tap to focus the element
    if no_retry {
        idb::tap(device, point_x, point_y)?;
    } else {
        idb::tap_with_retry(device, point_x, point_y, config)?;
    }
    info!("Fill: tapped ref [{}] to focus at ({:.1}, {:.1})", ref_id, point_x, point_y);

    // Clear existing text if requested (select all via triple-tap, then delete)
    if clear {
        // Triple-tap to select all text
        idb::tap(device, point_x, point_y)?;
        idb::tap(device, point_x, point_y)?;
        // Send delete key to clear selected text
        idb::send_key(device, "DELETE")?;
        info!("Fill: cleared existing text");
    }

    // Input the text
    idb::text_input(device, text)?;

    info!("Fill success: ref [{}], text \"{}\"", ref_id, text);
    println!(
        "Filled ref [{}] with \"{}\" on {}{}",
        ref_id,
        text,
        device,
        if clear { " (cleared first)" } else { "" }
    );
    Ok(())
}

fn handle_tap_region(
    device: &str,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    scale: Option<u32>,
    no_retry: bool,
    config: &CaptureConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let effective_scale = resolve_scale(device, scale);

    // Convert all region coordinates to logical points
    let (pt_x, pt_y, pt_w, pt_h) = if let Some(scale_factor) = effective_scale {
        let sf = f64::from(scale_factor);
        let px = f64::from(x) / sf;
        let py = f64::from(y) / sf;
        let pw = f64::from(width) / sf;
        let ph = f64::from(height) / sf;
        debug!(
            "TapRegion: pixel ({}, {}, {}x{}), scale {}x -> point ({:.1}, {:.1}, {:.1}x{:.1}), device {}",
            x, y, width, height, scale_factor, px, py, pw, ph, device
        );
        (px, py, pw, ph)
    } else {
        debug!(
            "TapRegion: point ({}, {}, {}x{}), device {}",
            x, y, width, height, device
        );
        (f64::from(x), f64::from(y), f64::from(width), f64::from(height))
    };

    idb::tap_region(device, pt_x, pt_y, pt_w, pt_h, no_retry, config)?;

    // Print result
    if let Some(scale_factor) = effective_scale {
        info!(
            "TapRegion success: pixel ({}, {}, {}x{}) -> point ({:.1}, {:.1}, {:.1}x{:.1}), scale {}x, retry={}",
            x, y, width, height, pt_x, pt_y, pt_w, pt_h, scale_factor, !no_retry
        );
        println!(
            "Tapped region pixel ({}, {}, {}x{}) -> point ({:.1}, {:.1}, {:.1}x{:.1}) on {} (scale: {}x{}{})",
            x, y, width, height, pt_x, pt_y, pt_w, pt_h, device, scale_factor,
            if scale.is_none() { " auto-detected" } else { "" },
            if no_retry { "" } else { ", retry enabled" }
        );
    } else {
        info!(
            "TapRegion success: point ({}, {}, {}x{}), retry={}",
            x, y, width, height, !no_retry
        );
        println!(
            "Tapped region ({}, {}, {}x{}) on {}{}",
            x, y, width, height, device,
            if no_retry { "" } else { " (retry enabled)" }
        );
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
    let effective_scale = resolve_scale(device, scale);

    if let Some(scale_factor) = effective_scale {
        debug!(
            "Swipe: pixel ({}, {}) -> ({}, {}), scale {}x, device {}",
            start_x, start_y, end_x, end_y, scale_factor, device
        );
        idb::swipe_pixel(
            device,
            f64::from(start_x),
            f64::from(start_y),
            f64::from(end_x),
            f64::from(end_y),
            f64::from(scale_factor),
            None,
        )?;
        info!(
            "Swipe success: pixel ({}, {}) -> ({}, {}), scale {}x",
            start_x, start_y, end_x, end_y, scale_factor
        );
        println!(
            "Swiped from pixel ({}, {}) to ({}, {}) on {} (scale: {}x{})",
            start_x, start_y, end_x, end_y, device, scale_factor,
            if scale.is_none() { " auto-detected" } else { "" }
        );
    } else {
        debug!(
            "Swipe: point ({}, {}) -> ({}, {}), device {}",
            start_x, start_y, end_x, end_y, device
        );
        idb::swipe(
            device,
            f64::from(start_x),
            f64::from(start_y),
            f64::from(end_x),
            f64::from(end_y),
            None,
        )?;
        info!(
            "Swipe success: point ({}, {}) -> ({}, {})",
            start_x, start_y, end_x, end_y
        );
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

fn handle_text(device: &str, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    idb::text_input(device, text)?;
    println!("Input text to {}: \"{}\"", device, text);
    Ok(())
}

fn handle_describe(device: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = idb::describe_all(device)?;
    println!("{}", output);
    Ok(())
}

fn handle_longpress(
    device: &str,
    x: u32,
    y: u32,
    duration: f64,
    scale: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let effective_scale = resolve_scale(device, scale);

    let (point_x, point_y) = if let Some(scale_factor) = effective_scale {
        let px = f64::from(x) / f64::from(scale_factor);
        let py = f64::from(y) / f64::from(scale_factor);
        debug!(
            "Longpress: pixel ({}, {}), scale {}x -> point ({:.1}, {:.1}), device {}",
            x, y, scale_factor, px, py, device
        );
        (px, py)
    } else {
        debug!("Longpress: point ({}, {}), device {}", x, y, device);
        (f64::from(x), f64::from(y))
    };

    idb::long_press(device, point_x, point_y, duration)?;

    if let Some(scale_factor) = effective_scale {
        println!(
            "Long pressed at pixel ({}, {}) -> point ({:.1}, {:.1}) on {} for {:.1}s (scale: {}x{})",
            x, y, point_x, point_y, device, duration, scale_factor,
            if scale.is_none() { " auto-detected" } else { "" }
        );
    } else {
        println!(
            "Long pressed at point ({}, {}) on {} for {:.1}s",
            x, y, device, duration
        );
    }
    Ok(())
}

fn handle_key(device: &str, key: &str) -> Result<(), Box<dyn std::error::Error>> {
    idb::send_key(device, key)?;
    println!("Sent key '{}' to {}", key, device);
    Ok(())
}
