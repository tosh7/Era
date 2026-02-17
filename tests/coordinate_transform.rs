// Integration tests for orientation-based coordinate transformation (Phase 1-2)

use era::simulator::orientation::{detect_orientation, transform_coordinates, Orientation};

// -------------------------------------------------------------------
// Portrait — identity transformation
// -------------------------------------------------------------------

#[test]
fn test_portrait_identity() {
    let (x, y) = transform_coordinates(150.0, 300.0, 390.0, 844.0, Orientation::Portrait);
    assert_eq!(x, 150.0);
    assert_eq!(y, 300.0);
}

#[test]
fn test_portrait_origin() {
    let (x, y) = transform_coordinates(0.0, 0.0, 390.0, 844.0, Orientation::Portrait);
    assert_eq!(x, 0.0);
    assert_eq!(y, 0.0);
}

#[test]
fn test_portrait_max_coordinates() {
    let (x, y) = transform_coordinates(390.0, 844.0, 390.0, 844.0, Orientation::Portrait);
    assert_eq!(x, 390.0);
    assert_eq!(y, 844.0);
}

// -------------------------------------------------------------------
// LandscapeLeft — (x, y) -> (y, screen_width - x)
// -------------------------------------------------------------------

#[test]
fn test_landscape_left_center() {
    // iPhone 16 Pro: 393pt x 852pt (portrait)
    let (x, y) = transform_coordinates(200.0, 400.0, 393.0, 852.0, Orientation::LandscapeLeft);
    assert_eq!(x, 400.0);
    assert_eq!(y, 193.0); // 393 - 200
}

#[test]
fn test_landscape_left_origin() {
    let (x, y) = transform_coordinates(0.0, 0.0, 390.0, 844.0, Orientation::LandscapeLeft);
    assert_eq!(x, 0.0);
    assert_eq!(y, 390.0); // width - 0
}

#[test]
fn test_landscape_left_max_coordinates() {
    // In landscape, user coordinate space is (0..844, 0..390)
    // max point = (844, 390) in landscape coords
    let (x, y) = transform_coordinates(844.0, 390.0, 390.0, 844.0, Orientation::LandscapeLeft);
    assert_eq!(x, 390.0);
    assert_eq!(y, -454.0); // 390 - 844 = negative (valid but outside screen)
}

#[test]
fn test_landscape_left_top_right_corner() {
    // Screen top-right in landscape = (screen_height, 0) in user coords
    // Transform: (y=0, width - x)
    let (x, y) = transform_coordinates(390.0, 0.0, 390.0, 844.0, Orientation::LandscapeLeft);
    assert_eq!(x, 0.0);
    assert_eq!(y, 0.0); // 390 - 390
}

// -------------------------------------------------------------------
// LandscapeRight — (x, y) -> (screen_height - y, x)
// -------------------------------------------------------------------

#[test]
fn test_landscape_right_center() {
    let (x, y) = transform_coordinates(200.0, 400.0, 393.0, 852.0, Orientation::LandscapeRight);
    assert_eq!(x, 452.0); // 852 - 400
    assert_eq!(y, 200.0);
}

#[test]
fn test_landscape_right_origin() {
    let (x, y) = transform_coordinates(0.0, 0.0, 390.0, 844.0, Orientation::LandscapeRight);
    assert_eq!(x, 844.0); // height - 0
    assert_eq!(y, 0.0);
}

#[test]
fn test_landscape_right_bottom_left_corner() {
    let (x, y) = transform_coordinates(0.0, 844.0, 390.0, 844.0, Orientation::LandscapeRight);
    assert_eq!(x, 0.0); // 844 - 844
    assert_eq!(y, 0.0);
}

// -------------------------------------------------------------------
// UpsideDown — (x, y) -> (screen_width - x, screen_height - y)
// -------------------------------------------------------------------

#[test]
fn test_upside_down_center() {
    let (x, y) = transform_coordinates(195.0, 422.0, 390.0, 844.0, Orientation::UpsideDown);
    assert_eq!(x, 195.0); // 390 - 195
    assert_eq!(y, 422.0); // 844 - 422
}

#[test]
fn test_upside_down_origin() {
    let (x, y) = transform_coordinates(0.0, 0.0, 390.0, 844.0, Orientation::UpsideDown);
    assert_eq!(x, 390.0); // width - 0
    assert_eq!(y, 844.0); // height - 0
}

#[test]
fn test_upside_down_max_coordinates() {
    let (x, y) = transform_coordinates(390.0, 844.0, 390.0, 844.0, Orientation::UpsideDown);
    assert_eq!(x, 0.0); // 390 - 390
    assert_eq!(y, 0.0); // 844 - 844
}

#[test]
fn test_upside_down_is_double_rotation() {
    // UpsideDown(UpsideDown(x, y)) should return (x, y)
    let w = 390.0;
    let h = 844.0;
    let (x1, y1) = transform_coordinates(100.0, 250.0, w, h, Orientation::UpsideDown);
    let (x2, y2) = transform_coordinates(x1, y1, w, h, Orientation::UpsideDown);
    assert!((x2 - 100.0).abs() < f64::EPSILON);
    assert!((y2 - 250.0).abs() < f64::EPSILON);
}

// -------------------------------------------------------------------
// Different screen sizes
// -------------------------------------------------------------------

#[test]
fn test_ipad_dimensions_portrait() {
    // iPad Pro 11": 834pt x 1194pt
    let (x, y) = transform_coordinates(417.0, 597.0, 834.0, 1194.0, Orientation::Portrait);
    assert_eq!(x, 417.0);
    assert_eq!(y, 597.0);
}

#[test]
fn test_ipad_dimensions_landscape_left() {
    let (x, y) = transform_coordinates(417.0, 597.0, 834.0, 1194.0, Orientation::LandscapeLeft);
    assert_eq!(x, 597.0);
    assert_eq!(y, 417.0); // 834 - 417
}

// -------------------------------------------------------------------
// Fallback behavior
// -------------------------------------------------------------------

#[test]
fn test_orientation_fallback_for_unknown_device() {
    // detect_orientation with a fake UDID should fall back to Portrait
    let info = detect_orientation("fake-udid-00000000");
    assert_eq!(info.orientation, Orientation::Portrait);
}

#[test]
fn test_fallback_portrait_with_zero_dimensions_is_safe() {
    // When fallback returns 0.0 for screen dimensions, Portrait still works
    // because Portrait returns (x, y) unchanged
    let (x, y) = transform_coordinates(200.0, 400.0, 0.0, 0.0, Orientation::Portrait);
    assert_eq!(x, 200.0);
    assert_eq!(y, 400.0);
}
