// Auto-wait module - Playwright-inspired element waiting
//
// Polls the UI tree via `idb describe-all` until a matching element
// appears, then returns it. Supports text-based and type-based matching.

use std::thread;
use std::time::{Duration, Instant};

use crate::capture::CaptureConfig;
use crate::simulator::{idb, ui_tree};

/// Default timeout for wait operations (5 seconds)
pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Polling interval between UI tree queries
const POLL_INTERVAL_MS: u64 = 300;

/// Selector for identifying a UI element
#[derive(Debug, Clone)]
pub enum ElementSelector {
    /// Match by accessibility label/value (case-insensitive partial match)
    Text(String),
    /// Match by element type with optional 0-based index
    Type { element_type: String, index: u32 },
}

impl std::fmt::Display for ElementSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

/// Result of a successful wait
pub struct WaitResult {
    pub center_x: f64,
    pub center_y: f64,
    pub element_type: String,
    pub label: Option<String>,
}

/// Wait for an element matching the selector to appear in the UI tree.
///
/// Polls `idb describe-all` at regular intervals until the element is found
/// or the timeout is reached.
pub fn wait_for_element(
    udid: &str,
    selector: &ElementSelector,
    timeout_ms: u64,
) -> Result<WaitResult, String> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(POLL_INTERVAL_MS);
    let mut attempt = 0u32;

    eprintln!(
        "[era] Waiting for {} (timeout: {}ms)...",
        selector, timeout_ms
    );

    loop {
        attempt += 1;

        match try_find_element(udid, selector) {
            Ok(result) => {
                let elapsed = start.elapsed().as_millis();
                eprintln!(
                    "[era] Found {} after {}ms ({} polls): {} at ({:.1}, {:.1})",
                    selector,
                    elapsed,
                    attempt,
                    result
                        .label
                        .as_ref()
                        .map(|l| format!("\"{}\"", l))
                        .unwrap_or_else(|| result.element_type.clone()),
                    result.center_x,
                    result.center_y
                );
                return Ok(result);
            }
            Err(_) => {
                if start.elapsed() >= timeout {
                    return Err(format!(
                        "Timeout waiting for {} after {}ms ({} polls)",
                        selector, timeout_ms, attempt
                    ));
                }
                thread::sleep(poll_interval);
            }
        }
    }
}

/// Attempt to find an element in the current UI tree (single poll)
fn try_find_element(udid: &str, selector: &ElementSelector) -> Result<WaitResult, String> {
    let json = idb::describe_all(udid).map_err(|e| format!("idb error: {}", e))?;
    let elements = ui_tree::parse(&json)?;

    match selector {
        ElementSelector::Text(text) => find_by_text_wait(&elements, text),
        ElementSelector::Type {
            element_type,
            index,
        } => find_by_type_wait(&elements, element_type, *index),
    }
}

fn find_by_text_wait(elements: &[ui_tree::UiElement], text: &str) -> Result<WaitResult, String> {
    let lower = text.to_lowercase();
    let mut matches = Vec::new();

    for root in elements {
        collect_matching(root, &lower, &mut matches);
    }

    if matches.is_empty() {
        return Err(format!("No element with text \"{}\"", text));
    }

    // Prefer enabled elements
    let best = matches
        .iter()
        .find(|e| e.enabled)
        .unwrap_or(&matches[0]);

    let (cx, cy) = best.frame.center();
    Ok(WaitResult {
        center_x: cx,
        center_y: cy,
        element_type: best.element_type.clone(),
        label: best.label.clone().or_else(|| best.value.clone()),
    })
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

fn find_by_type_wait(
    elements: &[ui_tree::UiElement],
    element_type: &str,
    index: u32,
) -> Result<WaitResult, String> {
    let mut matches = Vec::new();
    for root in elements {
        collect_by_type(root, element_type, &mut matches);
    }

    let idx = index as usize;
    if idx >= matches.len() {
        return Err(format!(
            "Type \"{}\" has {} matches, need index {}",
            element_type,
            matches.len(),
            idx
        ));
    }

    let target = matches[idx];
    let (cx, cy) = target.frame.center();
    Ok(WaitResult {
        center_x: cx,
        center_y: cy,
        element_type: target.element_type.clone(),
        label: target.label.clone().or_else(|| target.value.clone()),
    })
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

/// Wait for an element to appear, then tap it.
///
/// Combines `wait_for_element` with tap (with optional retry).
pub fn tap_element_with_wait(
    udid: &str,
    selector: &ElementSelector,
    timeout_ms: u64,
    no_retry: bool,
    config: &CaptureConfig,
) -> Result<WaitResult, String> {
    let result = wait_for_element(udid, selector, timeout_ms)?;

    if no_retry {
        idb::tap(udid, result.center_x, result.center_y)
            .map_err(|e| format!("Tap failed: {}", e))?;
    } else {
        idb::tap_with_retry(udid, result.center_x, result.center_y, config)
            .map_err(|e| format!("Tap failed: {}", e))?;
    }

    eprintln!(
        "[era] Tapped {} at ({:.1}, {:.1})",
        selector, result.center_x, result.center_y
    );

    Ok(result)
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
) -> Result<WaitResult, String> {
    let result = wait_for_element(udid, selector, timeout_ms)?;

    // Tap to focus
    if no_retry {
        idb::tap(udid, result.center_x, result.center_y)
            .map_err(|e| format!("Tap failed: {}", e))?;
    } else {
        idb::tap_with_retry(udid, result.center_x, result.center_y, config)
            .map_err(|e| format!("Tap failed: {}", e))?;
    }

    // Clear if requested
    if clear {
        idb::tap(udid, result.center_x, result.center_y)
            .map_err(|e| format!("Clear tap failed: {}", e))?;
        idb::tap(udid, result.center_x, result.center_y)
            .map_err(|e| format!("Clear tap failed: {}", e))?;
        idb::send_key(udid, "DELETE").map_err(|e| format!("Delete key failed: {}", e))?;
    }

    // Input text
    idb::text_input(udid, text).map_err(|e| format!("Text input failed: {}", e))?;

    eprintln!(
        "[era] Filled {} with \"{}\" at ({:.1}, {:.1})",
        selector, text, result.center_x, result.center_y
    );

    Ok(result)
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
    fn test_find_by_text_wait_found() {
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

        let result = find_by_text_wait(&elements, "Login").unwrap();
        assert_eq!(result.center_x, 200.0);
        assert_eq!(result.center_y, 422.0);
        assert_eq!(result.element_type, "Button");
        assert_eq!(result.label.as_deref(), Some("Login"));
    }

    #[test]
    fn test_find_by_text_wait_not_found() {
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

        let result = find_by_text_wait(&elements, "Nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_find_by_text_wait_case_insensitive() {
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

        let result = find_by_text_wait(&elements, "submit").unwrap();
        assert_eq!(result.element_type, "Button");
    }

    #[test]
    fn test_find_by_text_wait_prefers_enabled() {
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

        let result = find_by_text_wait(&elements, "OK").unwrap();
        assert_eq!(result.center_x, 250.0); // Second element (enabled)
    }

    #[test]
    fn test_find_by_type_wait_found() {
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

        let result = find_by_type_wait(&elements, "TextField", 0).unwrap();
        assert_eq!(result.center_x, 200.0);
        assert_eq!(result.center_y, 322.0);
    }

    #[test]
    fn test_find_by_type_wait_index() {
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

        let result = find_by_type_wait(&elements, "Button", 1).unwrap();
        assert_eq!(result.label.as_deref(), Some("Second"));
    }

    #[test]
    fn test_find_by_type_wait_index_out_of_bounds() {
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

        let result = find_by_type_wait(&elements, "Button", 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_by_type_wait_not_found() {
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

        let result = find_by_type_wait(&elements, "TextField", 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_by_text_wait_nested() {
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

        let result = find_by_text_wait(&elements, "Nested").unwrap();
        assert_eq!(result.element_type, "Button");
        assert_eq!(result.label.as_deref(), Some("Nested Login"));
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(DEFAULT_TIMEOUT_MS, 5000);
    }
}
