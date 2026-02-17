// Integration tests for automatic scale factor detection (Phase 1-1)

use era::simulator::device::DeviceScaleFactor;

// -------------------------------------------------------------------
// DeviceScaleFactor::from_device_name — comprehensive model coverage
// -------------------------------------------------------------------

#[test]
fn test_3x_iphone_pro_models_all_generations() {
    let models = [
        "iPhone 15 Pro",
        "iPhone 15 Pro Max",
        "iPhone 16 Pro",
        "iPhone 16 Pro Max",
        "iPhone 14 Pro",
        "iPhone 14 Pro Max",
        "iPhone 13 Pro",
        "iPhone 13 Pro Max",
        "iPhone 12 Pro",
        "iPhone 12 Pro Max",
        "iPhone 11 Pro",
        "iPhone 11 Pro Max",
    ];
    for model in &models {
        assert_eq!(
            DeviceScaleFactor::from_device_name(model),
            DeviceScaleFactor::X3,
            "{} should be 3x",
            model
        );
    }
}

#[test]
fn test_3x_iphone_plus_models() {
    let models = [
        "iPhone 14 Plus",
        "iPhone 15 Plus",
        "iPhone 16 Plus",
        "iPhone 8 Plus",
    ];
    for model in &models {
        assert_eq!(
            DeviceScaleFactor::from_device_name(model),
            DeviceScaleFactor::X3,
            "{} should be 3x",
            model
        );
    }
}

#[test]
fn test_3x_iphone_air_models() {
    assert_eq!(
        DeviceScaleFactor::from_device_name("iPhone Air"),
        DeviceScaleFactor::X3,
        "iPhone Air should be 3x (1260x2736 px)"
    );
}

#[test]
fn test_2x_ipad_air_not_misclassified() {
    // iPad Air contains "air" but is NOT an iPhone — should remain 2x
    let models = [
        "iPad Air (5th generation)",
        "iPad Air (4th generation)",
        "iPad Air (M2)",
    ];
    for model in &models {
        assert_eq!(
            DeviceScaleFactor::from_device_name(model),
            DeviceScaleFactor::X2,
            "{} should be 2x (iPad Air is not 3x)",
            model
        );
    }
}

#[test]
fn test_3x_iphone_x_series() {
    // iPhone X and XS are 3x
    assert_eq!(
        DeviceScaleFactor::from_device_name("iPhone X"),
        DeviceScaleFactor::X3
    );
    assert_eq!(
        DeviceScaleFactor::from_device_name("iPhone XS"),
        DeviceScaleFactor::X3
    );
    assert_eq!(
        DeviceScaleFactor::from_device_name("iPhone XS Max"),
        DeviceScaleFactor::X3
    );
}

#[test]
fn test_3x_standard_iphones_11_and_later() {
    // Standard (non-Pro, non-Plus) iPhones from 11+ are 3x
    for gen in 11..=16 {
        let model = format!("iPhone {}", gen);
        assert_eq!(
            DeviceScaleFactor::from_device_name(&model),
            DeviceScaleFactor::X3,
            "{} should be 3x",
            model
        );
    }
}

// -------------------------------------------------------------------
// 2x devices — must NOT be misclassified as 3x
// -------------------------------------------------------------------

#[test]
fn test_2x_iphone_xr_explicit_exception() {
    // iPhone XR is 2x (828x1792 / 414x896) despite matching "iphone x*"
    assert_eq!(
        DeviceScaleFactor::from_device_name("iPhone XR"),
        DeviceScaleFactor::X2
    );
}

#[test]
fn test_2x_iphone_se_all_generations() {
    let models = [
        "iPhone SE",
        "iPhone SE (2nd generation)",
        "iPhone SE (3rd generation)",
    ];
    for model in &models {
        assert_eq!(
            DeviceScaleFactor::from_device_name(model),
            DeviceScaleFactor::X2,
            "{} should be 2x",
            model
        );
    }
}

#[test]
fn test_2x_older_iphones() {
    let models = ["iPhone 8", "iPhone 7", "iPhone 6s", "iPhone 6"];
    for model in &models {
        assert_eq!(
            DeviceScaleFactor::from_device_name(model),
            DeviceScaleFactor::X2,
            "{} should be 2x",
            model
        );
    }
}

#[test]
fn test_2x_ipad_pro_not_misclassified() {
    // iPad Pro contains "pro" but is NOT an iPhone — should be 2x
    let models = [
        "iPad Pro (12.9-inch) (6th generation)",
        "iPad Pro (11-inch) (4th generation)",
        "iPad Pro (12.9-inch) (5th generation)",
        "iPad Pro (11-inch) (3rd generation)",
    ];
    for model in &models {
        assert_eq!(
            DeviceScaleFactor::from_device_name(model),
            DeviceScaleFactor::X2,
            "{} should be 2x (iPad Pro is not 3x)",
            model
        );
    }
}

#[test]
fn test_2x_ipads_default() {
    let models = [
        "iPad Air (5th generation)",
        "iPad mini (6th generation)",
        "iPad (10th generation)",
        "iPad (9th generation)",
    ];
    for model in &models {
        assert_eq!(
            DeviceScaleFactor::from_device_name(model),
            DeviceScaleFactor::X2,
            "{} should be 2x",
            model
        );
    }
}

// -------------------------------------------------------------------
// Value and Display
// -------------------------------------------------------------------

#[test]
fn test_scale_factor_numeric_values() {
    assert!((DeviceScaleFactor::X2.value() - 2.0).abs() < f64::EPSILON);
    assert!((DeviceScaleFactor::X3.value() - 3.0).abs() < f64::EPSILON);
}

#[test]
fn test_scale_factor_display_format() {
    assert_eq!(DeviceScaleFactor::X2.to_string(), "2x");
    assert_eq!(DeviceScaleFactor::X3.to_string(), "3x");
}

// -------------------------------------------------------------------
// detect_device_scale — live integration with simctl
// -------------------------------------------------------------------

#[test]
fn test_detect_device_scale_with_real_simulator_list() {
    // Verifies detect_device_scale can query the real simulator list
    // and returns a valid result for any available device.
    let devices = era::simulator::operations::list_devices();
    if let Ok(devices) = devices {
        if let Some(device_info) = devices.first() {
            let result =
                era::simulator::operations::detect_device_scale(&device_info.device.udid);
            assert!(
                result.is_ok(),
                "detect_device_scale should succeed for existing device: {:?}",
                result.err()
            );
            let scale = result.unwrap();
            // Must be either 2x or 3x
            assert!(
                scale == DeviceScaleFactor::X2 || scale == DeviceScaleFactor::X3,
                "Scale must be X2 or X3, got: {:?}",
                scale
            );
        }
    }
}

#[test]
fn test_detect_device_scale_unknown_udid_fallback() {
    // An unknown UDID should still succeed with a 2x fallback
    let result =
        era::simulator::operations::detect_device_scale("00000000-0000-0000-0000-000000000000");
    assert!(result.is_ok(), "Should fallback gracefully for unknown UDID");
    assert_eq!(result.unwrap(), DeviceScaleFactor::X2);
}

// -------------------------------------------------------------------
// Edge cases
// -------------------------------------------------------------------

#[test]
fn test_empty_device_name_returns_2x() {
    assert_eq!(
        DeviceScaleFactor::from_device_name(""),
        DeviceScaleFactor::X2
    );
}

#[test]
fn test_case_insensitive_matching() {
    assert_eq!(
        DeviceScaleFactor::from_device_name("IPHONE 16 PRO"),
        DeviceScaleFactor::X3
    );
    assert_eq!(
        DeviceScaleFactor::from_device_name("iphone 16 pro max"),
        DeviceScaleFactor::X3
    );
}
