// Auto-wait module - Playwright-inspired element waiting
//
// Polls the UI tree via `idb describe-all` until a matching element
// appears and stabilizes, then returns it. Supports text-based and
// type-based matching with frame stability verification.

use std::fmt;
use std::thread;
use std::time::{Duration, Instant};

use crate::capture::CaptureConfig;
use crate::simulator::{idb, ui_tree};

/// Default timeout for wait operations (5 seconds)
pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Polling interval between UI tree queries (150ms)
const POLL_INTERVAL_MS: u64 = 150;

/// Error type for wait operations
#[derive(Debug)]
pub enum WaitError {
    /// Element was not found before timeout
    Timeout {
        selector: String,
        timeout_ms: u64,
        polls: u32,
    },
    /// IDB communication error
    IdbError(String),
    /// No matching element found (single poll)
    NotFound(String),
}

impl fmt::Display for WaitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaitError::Timeout {
                selector,
                timeout_ms,
                polls,
            } => write!(
                f,
                "Timeout waiting for {} after {}ms ({} polls)",
                selector, timeout_ms, polls
            ),
            WaitError::IdbError(msg) => write!(f, "idb error: {}", msg),
            WaitError::NotFound(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for WaitError {}

/// Selector for identifying a UI element
#[derive(Debug, Clone)]
pub enum ElementSelector {
    /// Match by accessibility label/value (case-insensitive partial match)
    Text(String),
    /// Match by element type with optional 0-based index
    Type { element_type: String, index: u32 },
}

impl fmt::Display for ElementSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElementSelector::Text(text) => write!(f, "text \"{}\"", text),
            ElementSelector::Type {
                element_type,
                index,
            } => {
                if *index == 0 {
                    write!(f, "type \"{}\"", element_type)
                } else {
                    write!(f, "type \"{}\"[{}]", element_type, index)
                }
            }
        }
    }
}

/// Check if a frame is visible (non-zero dimensions)
fn is_visible(frame: &ui_tree::Frame) -> bool {
    frame.width > 0.0 && frame.height > 0.0
}

/// Find a matching element from parsed UI elements (single poll).
/// Returns element that is visible (non-zero frame) and enabled.
fn find_match(
    elements: &[ui_tree::UiElement],
    selector: &ElementSelector,
) -> Option<ui_tree::UiElement> {
    match selector {
        ElementSelector::Text(text) => find_by_text(elements, text),
        ElementSelector::Type {
            element_type,
            index,
        } => find_by_type(elements, element_type, *index),
    }
}

fn find_by_text(elements: &[ui_tree::UiElement], text: &str) -> Option<ui_tree::UiElement> {
    let lower = text.to_lowercase();
    let mut matches = Vec::new();
    for root in elements {
        collect_matching(root, &lower, &mut matches);
    }

    // Prefer visible + enabled
    if let Some(e) = matches
        .iter()
        .find(|e| is_visible(&e.frame) && e.enabled)
    {
        return Some((*e).clone());
    }
    // Fallback: visible but disabled
    matches
        .iter()
        .find(|e| is_visible(&e.frame))
        .map(|e| (*e).clone())
}

fn find_by_type(
    elements: &[ui_tree::UiElement],
    element_type: &str,
    index: u32,
) -> Option<ui_tree::UiElement> {
    let mut matches = Vec::new();
    for root in elements {
        collect_by_type(root, element_type, &mut matches);
    }

    // Filter visible + enabled, then pick by index
    let viable: Vec<_> = matches
        .iter()
        .copied()
        .filter(|e| is_visible(&e.frame) && e.enabled)
        .collect();

    viable.get(index as usize).map(|e| (*e).clone())
}

fn collect_matching<'a>(
    element: &'a ui_tree::UiElement,
    lower_text: &str,
    results: &mut Vec<&'a ui_tree::UiElement>,
) {
    let label_match = element
        .label
        .as_ref()
        .map(|l| l.to_lowercase().contains(lower_text))
        .unwrap_or(false);
    let value_match = element
        .value
        .as_ref()
        .map(|v| v.to_lowercase().contains(lower_text))
        .unwrap_or(false);

    if label_match || value_match {
        results.push(element);
    }
    for child in &element.children {
        collect_matching(child, lower_text, results);
    }
}

fn collect_by_type<'a>(
    element: &'a ui_tree::UiElement,
    element_type: &str,
    results: &mut Vec<&'a ui_tree::UiElement>,
) {
    if element.element_type == element_type {
        results.push(element);
    }
    for child in &element.children {
        collect_by_type(child, element_type, results);
    }
}

/// Single poll of the UI tree
fn poll_ui_tree(udid: &str) -> Result<Vec<ui_tree::UiElement>, WaitError> {
    let json = idb::describe_all(udid).map_err(|e| WaitError::IdbError(format!("{}", e)))?;
    ui_tree::parse(&json).map_err(WaitError::IdbError)
}

/// Wait for an element matching the selector to appear and stabilize.
///
/// Polls `idb describe-all` at 150ms intervals. The element must:
/// - Match the selector
/// - Be visible (frame has non-zero dimensions)
/// - Be enabled
/// - Have a stable frame (same position in 2 consecutive polls)
///
/// Returns the matched element and its center coordinates.
pub fn wait_for_element(
    udid: &str,
    selector: &ElementSelector,
    timeout_ms: u64,
) -> Result<(ui_tree::UiElement, f64, f64), WaitError> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(POLL_INTERVAL_MS);
    let mut attempt = 0u32;
    let mut prev_frame: Option<ui_tree::Frame> = None;

    eprintln!(
        "[auto-wait] Waiting for {} (timeout: {}ms)...",
        selector, timeout_ms
    );

    loop {
        attempt += 1;

        let elements = match poll_ui_tree(udid) {
            Ok(elems) => elems,
            Err(e) => {
                if start.elapsed() >= timeout {
                    return Err(WaitError::Timeout {
                        selector: selector.to_string(),
                        timeout_ms,
                        polls: attempt,
                    });
                }
                eprintln!("[auto-wait] Poll {} failed: {}, retrying...", attempt, e);
                thread::sleep(poll_interval);
                continue;
            }
        };

        if let Some(element) = find_match(&elements, selector) {
            let frame = &element.frame;

            // Check frame stability: must match previous poll's frame
            if let Some(ref prev) = prev_frame {
                if prev == frame {
                    let (cx, cy) = frame.center();
                    eprintln!(
                        "[auto-wait] Found {} after {}ms ({} polls): {} at ({:.1}, {:.1})",
                        selector,
                        start.elapsed().as_millis(),
                        attempt,
                        element
                            .label
                            .as_ref()
                            .map(|l| format!("\"{}\"", l))
                            .unwrap_or_else(|| element.element_type.clone()),
                        cx,
                        cy
                    );
                    return Ok((element, cx, cy));
                }
            }
            // Save current frame for next stability check
            prev_frame = Some(frame.clone());
        } else {
            // Element not found, reset stability tracking
            prev_frame = None;
        }

        if start.elapsed() >= timeout {
            return Err(WaitError::Timeout {
                selector: selector.to_string(),
                timeout_ms,
                polls: attempt,
            });
        }
        thread::sleep(poll_interval);
    }
}

/// Wait for an element to appear and stabilize, then tap it.
///
/// After tapping, performs post-tap verification by re-polling the UI tree
/// and checking if the element's state changed (frame, enabled, label, or value).
pub fn tap_element_with_wait(
    udid: &str,
    selector: &ElementSelector,
    timeout_ms: u64,
    no_retry: bool,
    config: &CaptureConfig,
) -> Result<(ui_tree::UiElement, f64, f64), WaitError> {
    let (element, cx, cy) = wait_for_element(udid, selector, timeout_ms)?;

    // Tap
    if no_retry {
        idb::tap(udid, cx, cy).map_err(|e| WaitError::IdbError(format!("Tap failed: {}", e)))?;
    } else {
        idb::tap_with_retry(udid, cx, cy, config)
            .map_err(|e| WaitError::IdbError(format!("Tap failed: {}", e)))?;
    }

    eprintln!(
        "[auto-wait] Tapped {} at ({:.1}, {:.1})",
        selector, cx, cy
    );

    // Post-tap verification: check if element state changed
    thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    if let Ok(elements) = poll_ui_tree(udid) {
        if let Some(post) = find_match(&elements, selector) {
            let changed = post.frame != element.frame
                || post.enabled != element.enabled
                || post.label != element.label
                || post.value != element.value;
            if changed {
                eprintln!("[auto-wait] Post-tap: element state changed (tap likely registered)");
            } else {
                eprintln!("[auto-wait] Post-tap: element state unchanged");
            }
        } else {
            eprintln!(
                "[auto-wait] Post-tap: element no longer found (navigation likely occurred)"
            );
        }
    }

    Ok((element, cx, cy))
}

/// Wait for an element to appear, then fill text into it.
///
/// Taps the element to focus, optionally clears existing text, then inputs new text.
pub fn fill_element_with_wait(
    udid: &str,
    selector: &ElementSelector,
    text: &str,
    clear: bool,
    timeout_ms: u64,
    no_retry: bool,
    config: &CaptureConfig,
) -> Result<(ui_tree::UiElement, f64, f64), WaitError> {
    let (element, cx, cy) = wait_for_element(udid, selector, timeout_ms)?;

    // Tap to focus
    if no_retry {
        idb::tap(udid, cx, cy)
            .map_err(|e| WaitError::IdbError(format!("Tap failed: {}", e)))?;
    } else {
        idb::tap_with_retry(udid, cx, cy, config)
            .map_err(|e| WaitError::IdbError(format!("Tap failed: {}", e)))?;
    }

    // Clear if requested
    if clear {
        idb::tap(udid, cx, cy)
            .map_err(|e| WaitError::IdbError(format!("Clear tap failed: {}", e)))?;
        idb::tap(udid, cx, cy)
            .map_err(|e| WaitError::IdbError(format!("Clear tap failed: {}", e)))?;
        idb::send_key(udid, "DELETE")
            .map_err(|e| WaitError::IdbError(format!("Delete key failed: {}", e)))?;
    }

    // Input text
    idb::text_input(udid, text)
        .map_err(|e| WaitError::IdbError(format!("Text input failed: {}", e)))?;

    eprintln!(
        "[auto-wait] Filled {} with \"{}\" at ({:.1}, {:.1})",
        selector, text, cx, cy
    );

    Ok((element, cx, cy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_display_text() {
        let sel = ElementSelector::Text("Login".to_string());
        assert_eq!(format!("{}", sel), "text \"Login\"");
    }

    #[test]
    fn test_selector_display_type() {
        let sel = ElementSelector::Type {
            element_type: "Button".to_string(),
            index: 0,
        };
        assert_eq!(format!("{}", sel), "type \"Button\"");
    }

    #[test]
    fn test_selector_display_type_with_index() {
        let sel = ElementSelector::Type {
            element_type: "Cell".to_string(),
            index: 3,
        };
        assert_eq!(format!("{}", sel), "type \"Cell\"[3]");
    }

    #[test]
    fn test_wait_error_display_timeout() {
        let err = WaitError::Timeout {
            selector: "text \"Login\"".to_string(),
            timeout_ms: 5000,
            polls: 33,
        };
        assert_eq!(
            format!("{}", err),
            "Timeout waiting for text \"Login\" after 5000ms (33 polls)"
        );
    }

    #[test]
    fn test_wait_error_display_idb() {
        let err = WaitError::IdbError("connection refused".to_string());
        assert_eq!(format!("{}", err), "idb error: connection refused");
    }

    #[test]
    fn test_wait_error_display_not_found() {
        let err = WaitError::NotFound("no match".to_string());
        assert_eq!(format!("{}", err), "no match");
    }

    #[test]
    fn test_is_visible_nonzero() {
        let frame = ui_tree::Frame {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 44.0,
        };
        assert!(is_visible(&frame));
    }

    #[test]
    fn test_is_visible_zero_width() {
        let frame = ui_tree::Frame {
            x: 10.0,
            y: 20.0,
            width: 0.0,
            height: 44.0,
        };
        assert!(!is_visible(&frame));
    }

    #[test]
    fn test_is_visible_zero_height() {
        let frame = ui_tree::Frame {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 0.0,
        };
        assert!(!is_visible(&frame));
    }

    #[test]
    fn test_find_by_text_visible_enabled() {
        let elements = vec![ui_tree::UiElement {
            element_type: "Button".to_string(),
            label: Some("Login".to_string()),
            value: None,
            frame: ui_tree::Frame {
                x: 100.0,
                y: 400.0,
                width: 200.0,
                height: 44.0,
            },
            enabled: true,
            traits: vec![],
            children: vec![],
        }];

        let result = find_by_text(&elements, "Login").unwrap();
        assert_eq!(result.element_type, "Button");
        assert_eq!(result.label.as_deref(), Some("Login"));
    }

    #[test]
    fn test_find_by_text_skips_invisible() {
        let elements = vec![
            ui_tree::UiElement {
                element_type: "Button".to_string(),
                label: Some("Login".to_string()),
                value: None,
                frame: ui_tree::Frame {
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 0.0,
                },
                enabled: true,
                traits: vec![],
                children: vec![],
            },
            ui_tree::UiElement {
                element_type: "Button".to_string(),
                label: Some("Login".to_string()),
                value: None,
                frame: ui_tree::Frame {
                    x: 100.0,
                    y: 400.0,
                    width: 200.0,
                    height: 44.0,
                },
                enabled: true,
                traits: vec![],
                children: vec![],
            },
        ];

        let result = find_by_text(&elements, "Login").unwrap();
        // Should pick the visible one (second element)
        assert_eq!(result.frame.x, 100.0);
    }

    #[test]
    fn test_find_by_text_prefers_enabled() {
        let elements = vec![
            ui_tree::UiElement {
                element_type: "Button".to_string(),
                label: Some("OK".to_string()),
                value: None,
                frame: ui_tree::Frame {
                    x: 100.0,
                    y: 100.0,
                    width: 100.0,
                    height: 44.0,
                },
                enabled: false,
                traits: vec![],
                children: vec![],
            },
            ui_tree::UiElement {
                element_type: "Button".to_string(),
                label: Some("OK".to_string()),
                value: None,
                frame: ui_tree::Frame {
                    x: 200.0,
                    y: 200.0,
                    width: 100.0,
                    height: 44.0,
                },
                enabled: true,
                traits: vec![],
                children: vec![],
            },
        ];

        let result = find_by_text(&elements, "OK").unwrap();
        assert_eq!(result.frame.x, 200.0); // Second element (enabled)
    }

    #[test]
    fn test_find_by_text_case_insensitive() {
        let elements = vec![ui_tree::UiElement {
            element_type: "Button".to_string(),
            label: Some("Submit Order".to_string()),
            value: None,
            frame: ui_tree::Frame {
                x: 50.0,
                y: 500.0,
                width: 300.0,
                height: 44.0,
            },
            enabled: true,
            traits: vec![],
            children: vec![],
        }];

        let result = find_by_text(&elements, "submit").unwrap();
        assert_eq!(result.element_type, "Button");
    }

    #[test]
    fn test_find_by_text_not_found() {
        let elements = vec![ui_tree::UiElement {
            element_type: "Button".to_string(),
            label: Some("Login".to_string()),
            value: None,
            frame: ui_tree::Frame {
                x: 100.0,
                y: 400.0,
                width: 200.0,
                height: 44.0,
            },
            enabled: true,
            traits: vec![],
            children: vec![],
        }];

        assert!(find_by_text(&elements, "Nonexistent").is_none());
    }

    #[test]
    fn test_find_by_text_nested() {
        let elements = vec![ui_tree::UiElement {
            element_type: "Window".to_string(),
            label: None,
            value: None,
            frame: ui_tree::Frame {
                x: 0.0,
                y: 0.0,
                width: 393.0,
                height: 852.0,
            },
            enabled: true,
            traits: vec![],
            children: vec![ui_tree::UiElement {
                element_type: "Button".to_string(),
                label: Some("Nested Login".to_string()),
                value: None,
                frame: ui_tree::Frame {
                    x: 50.0,
                    y: 400.0,
                    width: 200.0,
                    height: 44.0,
                },
                enabled: true,
                traits: vec![],
                children: vec![],
            }],
        }];

        let result = find_by_text(&elements, "Nested").unwrap();
        assert_eq!(result.element_type, "Button");
        assert_eq!(result.label.as_deref(), Some("Nested Login"));
    }

    #[test]
    fn test_find_by_type_visible_enabled() {
        let elements = vec![ui_tree::UiElement {
            element_type: "TextField".to_string(),
            label: Some("Email".to_string()),
            value: None,
            frame: ui_tree::Frame {
                x: 50.0,
                y: 300.0,
                width: 300.0,
                height: 44.0,
            },
            enabled: true,
            traits: vec![],
            children: vec![],
        }];

        let result = find_by_type(&elements, "TextField", 0).unwrap();
        let (cx, cy) = result.frame.center();
        assert_eq!(cx, 200.0);
        assert_eq!(cy, 322.0);
    }

    #[test]
    fn test_find_by_type_index() {
        let elements = vec![
            ui_tree::UiElement {
                element_type: "Button".to_string(),
                label: Some("First".to_string()),
                value: None,
                frame: ui_tree::Frame {
                    x: 10.0,
                    y: 100.0,
                    width: 100.0,
                    height: 44.0,
                },
                enabled: true,
                traits: vec![],
                children: vec![],
            },
            ui_tree::UiElement {
                element_type: "Button".to_string(),
                label: Some("Second".to_string()),
                value: None,
                frame: ui_tree::Frame {
                    x: 10.0,
                    y: 200.0,
                    width: 100.0,
                    height: 44.0,
                },
                enabled: true,
                traits: vec![],
                children: vec![],
            },
        ];

        let result = find_by_type(&elements, "Button", 1).unwrap();
        assert_eq!(result.label.as_deref(), Some("Second"));
    }

    #[test]
    fn test_find_by_type_index_out_of_bounds() {
        let elements = vec![ui_tree::UiElement {
            element_type: "Button".to_string(),
            label: Some("Only".to_string()),
            value: None,
            frame: ui_tree::Frame {
                x: 10.0,
                y: 100.0,
                width: 100.0,
                height: 44.0,
            },
            enabled: true,
            traits: vec![],
            children: vec![],
        }];

        assert!(find_by_type(&elements, "Button", 1).is_none());
    }

    #[test]
    fn test_find_by_type_not_found() {
        let elements = vec![ui_tree::UiElement {
            element_type: "Button".to_string(),
            label: None,
            value: None,
            frame: ui_tree::Frame {
                x: 10.0,
                y: 100.0,
                width: 100.0,
                height: 44.0,
            },
            enabled: true,
            traits: vec![],
            children: vec![],
        }];

        assert!(find_by_type(&elements, "TextField", 0).is_none());
    }

    #[test]
    fn test_find_by_type_skips_disabled() {
        let elements = vec![
            ui_tree::UiElement {
                element_type: "Button".to_string(),
                label: Some("Disabled".to_string()),
                value: None,
                frame: ui_tree::Frame {
                    x: 10.0,
                    y: 100.0,
                    width: 100.0,
                    height: 44.0,
                },
                enabled: false,
                traits: vec![],
                children: vec![],
            },
            ui_tree::UiElement {
                element_type: "Button".to_string(),
                label: Some("Enabled".to_string()),
                value: None,
                frame: ui_tree::Frame {
                    x: 10.0,
                    y: 200.0,
                    width: 100.0,
                    height: 44.0,
                },
                enabled: true,
                traits: vec![],
                children: vec![],
            },
        ];

        // Index 0 should be the enabled one (disabled is filtered out)
        let result = find_by_type(&elements, "Button", 0).unwrap();
        assert_eq!(result.label.as_deref(), Some("Enabled"));
    }

    #[test]
    fn test_frame_stability_required() {
        // This tests the concept: same frame in 2 consecutive polls = stable
        let frame1 = ui_tree::Frame {
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 44.0,
        };
        let frame2 = ui_tree::Frame {
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 44.0,
        };
        assert_eq!(frame1, frame2); // PartialEq on Frame

        let frame3 = ui_tree::Frame {
            x: 100.0,
            y: 201.0, // shifted
            width: 50.0,
            height: 44.0,
        };
        assert_ne!(frame1, frame3); // Different = not stable yet
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(DEFAULT_TIMEOUT_MS, 5000);
    }

    #[test]
    fn test_poll_interval() {
        assert_eq!(POLL_INTERVAL_MS, 150);
    }
}
