// Regression tests for CLI argument parsing

use clap::Parser;
use era::cli::commands::{Cli, Commands};

// Helper to parse CLI args
fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(args)
}

// -------------------------------------------------------------------
// tap command — argument parsing
// -------------------------------------------------------------------

#[test]
fn test_tap_minimal_args() {
    let cli = parse(&["era", "tap", "-d", "UDID", "-x", "100", "-y", "200"]).unwrap();
    match cli.command {
        Commands::Tap {
            device,
            x,
            y,
            scale,
            no_retry,
            ..
        } => {
            assert_eq!(device, "UDID");
            assert_eq!(x, 100);
            assert_eq!(y, 200);
            assert!(scale.is_none());
            assert!(!no_retry);
        }
        _ => panic!("Expected Tap command"),
    }
}

#[test]
fn test_tap_with_scale() {
    let cli = parse(&[
        "era", "tap", "-d", "UDID", "-x", "630", "-y", "1368", "--scale", "3",
    ])
    .unwrap();
    match cli.command {
        Commands::Tap { scale, .. } => {
            assert_eq!(scale, Some(3));
        }
        _ => panic!("Expected Tap command"),
    }
}

#[test]
fn test_tap_with_no_retry() {
    let cli = parse(&[
        "era", "tap", "-d", "UDID", "-x", "100", "-y", "200", "--no-retry",
    ])
    .unwrap();
    match cli.command {
        Commands::Tap { no_retry, .. } => {
            assert!(no_retry);
        }
        _ => panic!("Expected Tap command"),
    }
}

#[test]
fn test_tap_with_all_options() {
    let cli = parse(&[
        "era",
        "tap",
        "-d",
        "DEVICE-UUID",
        "-x",
        "300",
        "-y",
        "600",
        "--scale",
        "2",
        "--no-retry",
        "--observe",
        "always",
    ])
    .unwrap();
    match cli.command {
        Commands::Tap {
            device,
            x,
            y,
            scale,
            no_retry,
            observe,
        } => {
            assert_eq!(device, "DEVICE-UUID");
            assert_eq!(x, 300);
            assert_eq!(y, 600);
            assert_eq!(scale, Some(2));
            assert!(no_retry);
            assert_eq!(observe, era::capture::ObservationPolicy::Always);
        }
        _ => panic!("Expected Tap command"),
    }
}

#[test]
fn test_tap_missing_device_fails() {
    let result = parse(&["era", "tap", "-x", "100", "-y", "200"]);
    assert!(result.is_err());
}

#[test]
fn test_tap_missing_coordinates_fails() {
    let result = parse(&["era", "tap", "-d", "UDID", "-x", "100"]);
    assert!(result.is_err());
}

#[test]
fn test_tap_invalid_coordinate_type_fails() {
    let result = parse(&["era", "tap", "-d", "UDID", "-x", "abc", "-y", "200"]);
    assert!(result.is_err());
}

// -------------------------------------------------------------------
// swipe command — argument parsing
// -------------------------------------------------------------------

#[test]
fn test_swipe_minimal_args() {
    let cli = parse(&[
        "era",
        "swipe",
        "-d",
        "UDID",
        "--start-x",
        "100",
        "--start-y",
        "200",
        "--end-x",
        "100",
        "--end-y",
        "600",
    ])
    .unwrap();
    match cli.command {
        Commands::Swipe {
            device,
            start_x,
            start_y,
            end_x,
            end_y,
            scale,
        } => {
            assert_eq!(device, "UDID");
            assert_eq!(start_x, 100);
            assert_eq!(start_y, 200);
            assert_eq!(end_x, 100);
            assert_eq!(end_y, 600);
            assert!(scale.is_none());
        }
        _ => panic!("Expected Swipe command"),
    }
}

#[test]
fn test_swipe_with_scale() {
    let cli = parse(&[
        "era",
        "swipe",
        "-d",
        "UDID",
        "--start-x",
        "300",
        "--start-y",
        "600",
        "--end-x",
        "300",
        "--end-y",
        "1800",
        "--scale",
        "3",
    ])
    .unwrap();
    match cli.command {
        Commands::Swipe { scale, .. } => {
            assert_eq!(scale, Some(3));
        }
        _ => panic!("Expected Swipe command"),
    }
}

#[test]
fn test_swipe_missing_end_coordinates_fails() {
    let result = parse(&[
        "era",
        "swipe",
        "-d",
        "UDID",
        "--start-x",
        "100",
        "--start-y",
        "200",
    ]);
    assert!(result.is_err());
}

// -------------------------------------------------------------------
// tap-region command — argument parsing
// -------------------------------------------------------------------

#[test]
fn test_tap_region_minimal_args() {
    let cli = parse(&[
        "era",
        "tap-region",
        "-d",
        "UDID",
        "-x",
        "50",
        "-y",
        "100",
        "-W",
        "200",
        "-H",
        "80",
    ])
    .unwrap();
    match cli.command {
        Commands::TapRegion {
            device,
            x,
            y,
            width,
            height,
            scale,
            no_retry,
            ..
        } => {
            assert_eq!(device, "UDID");
            assert_eq!(x, 50);
            assert_eq!(y, 100);
            assert_eq!(width, 200);
            assert_eq!(height, 80);
            assert!(scale.is_none());
            assert!(!no_retry);
        }
        _ => panic!("Expected TapRegion command"),
    }
}

#[test]
fn test_tap_region_with_scale_and_no_retry() {
    let cli = parse(&[
        "era",
        "tap-region",
        "-d",
        "UDID",
        "-x",
        "150",
        "-y",
        "300",
        "-W",
        "600",
        "-H",
        "240",
        "--scale",
        "3",
        "--no-retry",
    ])
    .unwrap();
    match cli.command {
        Commands::TapRegion {
            scale, no_retry, ..
        } => {
            assert_eq!(scale, Some(3));
            assert!(no_retry);
        }
        _ => panic!("Expected TapRegion command"),
    }
}

#[test]
fn test_tap_region_missing_dimensions_fails() {
    // Missing -W and -H
    let result = parse(&[
        "era",
        "tap-region",
        "-d",
        "UDID",
        "-x",
        "50",
        "-y",
        "100",
    ]);
    assert!(result.is_err());
}

// -------------------------------------------------------------------
// Global flags
// -------------------------------------------------------------------

#[test]
fn test_verbose_flag_default() {
    let cli = parse(&["era", "list"]).unwrap();
    assert_eq!(cli.verbose, 0);
}

#[test]
fn test_verbose_single() {
    let cli = parse(&["era", "-v", "list"]).unwrap();
    assert_eq!(cli.verbose, 1);
}

#[test]
fn test_verbose_double() {
    let cli = parse(&["era", "-vv", "list"]).unwrap();
    assert_eq!(cli.verbose, 2);
}

#[test]
fn test_debug_capture_flag() {
    let cli = parse(&["era", "--debug-capture", "list"]).unwrap();
    assert!(cli.debug_capture);
}

#[test]
fn test_debug_dir_default() {
    let cli = parse(&["era", "list"]).unwrap();
    assert_eq!(cli.debug_dir, "/tmp/era-debug/");
}

#[test]
fn test_debug_dir_custom() {
    let cli = parse(&["era", "--debug-dir", "/custom/path/", "list"]).unwrap();
    assert_eq!(cli.debug_dir, "/custom/path/");
}

// -------------------------------------------------------------------
// Other commands — basic parse validation
// -------------------------------------------------------------------

#[test]
fn test_list_command() {
    let cli = parse(&["era", "list"]).unwrap();
    assert!(matches!(cli.command, Commands::List { booted: false }));
}

#[test]
fn test_list_booted_flag() {
    let cli = parse(&["era", "list", "--booted"]).unwrap();
    assert!(matches!(cli.command, Commands::List { booted: true }));
}

#[test]
fn test_boot_command() {
    let cli = parse(&["era", "boot", "MY-UDID"]).unwrap();
    match cli.command {
        Commands::Boot { device } => assert_eq!(device, "MY-UDID"),
        _ => panic!("Expected Boot command"),
    }
}

#[test]
fn test_shutdown_all() {
    let cli = parse(&["era", "shutdown", "all"]).unwrap();
    match cli.command {
        Commands::Shutdown { device } => assert_eq!(device, "all"),
        _ => panic!("Expected Shutdown command"),
    }
}

#[test]
fn test_unknown_command_fails() {
    let result = parse(&["era", "nonexistent"]);
    assert!(result.is_err());
}

#[test]
fn test_no_command_fails() {
    let result = parse(&["era"]);
    assert!(result.is_err());
}
