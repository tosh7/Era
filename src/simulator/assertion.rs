// Assertion module - UI element condition checking with polling
//
// Provides `assert_condition()` which polls the UI tree until a condition
// is met or a timeout expires. Used by `era assert` CLI commands.
// Exit code 0 = pass, 1 = fail.

use std::thread;
use std::time::{Duration, Instant};

use log::info;

use super::idb;
use super::snapshot;
use super::ui_tree::{self, UiElement};

/// Element selector for assertion targets
#[derive(Debug, Clone)]
pub enum Selector {
    /// Match by text (case-insensitive partial match on label/value)
    Text(String),
    /// Match by element type with optional 0-based index
    Type(String, Option<u32>),
    /// Match by ref number from a previous snapshot
    Ref(u32),
}

/// Condition to assert on a UI element or set of elements
#[derive(Debug, Clone)]
pub enum Condition {
    /// Element exists and is visible (non-zero frame)
    Visible,
    /// Element does not exist or has zero-size frame
    Hidden,
    /// Element exists and is enabled
    Enabled,
    /// Element exists and is disabled
    Disabled,
    /// Element's label or value exactly equals the given text
    TextEquals(String),
    /// Element's label or value contains the given text (case-insensitive)
    TextContains(String),
    /// The number of matching elements equals the expected count
    Count(u32),
}

/// Result of an assertion check
#[derive(Debug)]
pub struct AssertResult {
    pub passed: bool,
    pub message: String,
}

/// Default timeout for assertions (5 seconds)
const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Polling interval (500ms)
const POLL_INTERVAL_MS: u64 = 500;

/// Run an assertion with polling until the condition is met or timeout expires.
///
/// Fetches the live UI tree on each poll and checks the condition.
/// Returns immediately on first pass; retries until timeout on failure.
pub fn assert_condition(
    udid: &str,
    selector: &Selector,
    condition: &Condition,
    timeout_ms: Option<u64>,
) -> AssertResult {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));
    let poll_interval = Duration::from_millis(POLL_INTERVAL_MS);
    let start = Instant::now();

    loop {
        match check_once(udid, selector, condition) {
            Ok(result) if result.passed => {
                info!("Assertion passed: {}", result.message);
                return result;
            }
            Ok(result) => {
                if start.elapsed() >= timeout {
                    info!("Assertion failed (timeout): {}", result.message);
                    return result;
                }
                info!("Assertion not yet met, retrying... ({})", result.message);
            }
            Err(e) => {
                if start.elapsed() >= timeout {
                    return AssertResult {
                        passed: false,
                        message: format!("Assertion error: {}", e),
                    };
                }
                info!("Assertion check error, retrying... ({})", e);
            }
        }

        thread::sleep(poll_interval);
    }
}

/// Perform a single assertion check against the live UI tree.
fn check_once(
    udid: &str,
    selector: &Selector,
    condition: &Condition,
) -> Result<AssertResult, String> {
    let json_output =
        idb::describe_all(udid).map_err(|e| format!("Failed to get UI tree: {}", e))?;
    let elements = ui_tree::parse(&json_output)?;

    match condition {
        Condition::Count(expected) => check_count(&elements, selector, *expected),
        _ => check_element_condition(&elements, udid, selector, condition),
    }
}

/// Check conditions that operate on a single resolved element.
fn check_element_condition(
    elements: &[UiElement],
    udid: &str,
    selector: &Selector,
    condition: &Condition,
) -> Result<AssertResult, String> {
    let found = resolve_selector(elements, udid, selector);

    match condition {
        Condition::Visible => match found {
            Ok(el) => {
                if el.frame.width > 0.0 && el.frame.height > 0.0 {
                    Ok(AssertResult {
                        passed: true,
                        message: format!(
                            "{} \"{}\" is visible",
                            el.element_type,
                            display_label(&el)
                        ),
                    })
                } else {
                    Ok(AssertResult {
                        passed: false,
                        message: format!(
                            "{} \"{}\" exists but has zero size",
                            el.element_type,
                            display_label(&el)
                        ),
                    })
                }
            }
            Err(_) => Ok(AssertResult {
                passed: false,
                message: format!("Element not found: {}", selector_desc(selector)),
            }),
        },
        Condition::Hidden => match found {
            Ok(el) => {
                if el.frame.width <= 0.0 && el.frame.height <= 0.0 {
                    Ok(AssertResult {
                        passed: true,
                        message: format!(
                            "{} \"{}\" has zero size (hidden)",
                            el.element_type,
                            display_label(&el)
                        ),
                    })
                } else {
                    Ok(AssertResult {
                        passed: false,
                        message: format!(
                            "{} \"{}\" is still visible",
                            el.element_type,
                            display_label(&el)
                        ),
                    })
                }
            }
            Err(_) => Ok(AssertResult {
                passed: true,
                message: format!("Element not found (hidden): {}", selector_desc(selector)),
            }),
        },
        Condition::Enabled => match found {
            Ok(el) => Ok(AssertResult {
                passed: el.enabled,
                message: if el.enabled {
                    format!("{} \"{}\" is enabled", el.element_type, display_label(&el))
                } else {
                    format!("{} \"{}\" is disabled", el.element_type, display_label(&el))
                },
            }),
            Err(_) => Ok(AssertResult {
                passed: false,
                message: format!("Element not found: {}", selector_desc(selector)),
            }),
        },
        Condition::Disabled => match found {
            Ok(el) => Ok(AssertResult {
                passed: !el.enabled,
                message: if !el.enabled {
                    format!("{} \"{}\" is disabled", el.element_type, display_label(&el))
                } else {
                    format!("{} \"{}\" is enabled", el.element_type, display_label(&el))
                },
            }),
            Err(_) => Ok(AssertResult {
                passed: false,
                message: format!("Element not found: {}", selector_desc(selector)),
            }),
        },
        Condition::TextEquals(expected) => match found {
            Ok(el) => {
                let actual = el
                    .label
                    .as_deref()
                    .or(el.value.as_deref())
                    .unwrap_or("");
                Ok(AssertResult {
                    passed: actual == expected.as_str(),
                    message: if actual == expected.as_str() {
                        format!("{} text equals \"{}\"", el.element_type, expected)
                    } else {
                        format!(
                            "{} text is \"{}\" (expected \"{}\")",
                            el.element_type, actual, expected
                        )
                    },
                })
            }
            Err(_) => Ok(AssertResult {
                passed: false,
                message: format!("Element not found: {}", selector_desc(selector)),
            }),
        },
        Condition::TextContains(expected) => match found {
            Ok(el) => {
                let actual = el
                    .label
                    .as_deref()
                    .or(el.value.as_deref())
                    .unwrap_or("");
                let contains = actual.to_lowercase().contains(&expected.to_lowercase());
                Ok(AssertResult {
                    passed: contains,
                    message: if contains {
                        format!(
                            "{} text \"{}\" contains \"{}\"",
                            el.element_type, actual, expected
                        )
                    } else {
                        format!(
                            "{} text \"{}\" does not contain \"{}\"",
                            el.element_type, actual, expected
                        )
                    },
                })
            }
            Err(_) => Ok(AssertResult {
                passed: false,
                message: format!("Element not found: {}", selector_desc(selector)),
            }),
        },
        Condition::Count(_) => unreachable!("Count handled separately"),
    }
}

/// Check element count condition.
fn check_count(
    elements: &[UiElement],
    selector: &Selector,
    expected: u32,
) -> Result<AssertResult, String> {
    let count = count_matches(elements, selector);
    Ok(AssertResult {
        passed: count == expected,
        message: if count == expected {
            format!(
                "Found {} elements matching {} (expected {})",
                count,
                selector_desc(selector),
                expected
            )
        } else {
            format!(
                "Found {} elements matching {} (expected {})",
                count,
                selector_desc(selector),
                expected
            )
        },
    })
}

/// Resolve a selector to a single UiElement (cloned).
///
/// For Text: case-insensitive partial match, prefers enabled elements.
/// For Type: exact type match with optional 0-based index.
/// For Ref: looks up ref map, then finds element at those coordinates.
fn resolve_selector(
    elements: &[UiElement],
    udid: &str,
    selector: &Selector,
) -> Result<UiElement, String> {
    match selector {
        Selector::Text(text) => {
            let mut matches = Vec::new();
            for root in elements {
                collect_by_text(root, text, &mut matches);
            }
            if matches.is_empty() {
                Err(format!("No element found with text \"{}\"", text))
            } else {
                let best = matches.iter().find(|e| e.enabled).unwrap_or(&matches[0]);
                Ok((*best).clone())
            }
        }
        Selector::Type(element_type, index) => {
            let mut matches = Vec::new();
            for root in elements {
                collect_by_type(root, element_type, &mut matches);
            }
            let idx = index.unwrap_or(0) as usize;
            if matches.is_empty() {
                Err(format!("No element found with type \"{}\"", element_type))
            } else if idx >= matches.len() {
                Err(format!(
                    "Type \"{}\" has {} elements, index {} requested",
                    element_type,
                    matches.len(),
                    idx
                ))
            } else {
                Ok(matches[idx].clone())
            }
        }
        Selector::Ref(ref_id) => {
            let ref_map = snapshot::load_ref_map(udid)?;
            match ref_map.entries.get(ref_id) {
                Some(entry) => {
                    for root in elements {
                        if let Some(el) = root.find_at_point(entry.center_x, entry.center_y) {
                            return Ok(el.clone());
                        }
                    }
                    Err(format!(
                        "Ref [{}] element not found at ({:.1}, {:.1})",
                        ref_id, entry.center_x, entry.center_y
                    ))
                }
                None => Err(format!("Ref [{}] not found in snapshot", ref_id)),
            }
        }
    }
}

/// Count elements matching a selector in the UI tree.
fn count_matches(elements: &[UiElement], selector: &Selector) -> u32 {
    match selector {
        Selector::Text(text) => {
            let mut matches = Vec::new();
            for root in elements {
                collect_by_text(root, text, &mut matches);
            }
            matches.len() as u32
        }
        Selector::Type(element_type, _) => {
            let mut matches = Vec::new();
            for root in elements {
                collect_by_type(root, element_type, &mut matches);
            }
            matches.len() as u32
        }
        Selector::Ref(_) => {
            // Ref always identifies a single element
            1
        }
    }
}

fn collect_by_text<'a>(element: &'a UiElement, text: &str, results: &mut Vec<&'a UiElement>) {
    let lower = text.to_lowercase();
    let label_match = element
        .label
        .as_ref()
        .map(|l| l.to_lowercase().contains(&lower))
        .unwrap_or(false);
    let value_match = element
        .value
        .as_ref()
        .map(|v| v.to_lowercase().contains(&lower))
        .unwrap_or(false);
    if label_match || value_match {
        results.push(element);
    }
    for child in &element.children {
        collect_by_text(child, text, results);
    }
}

fn collect_by_type<'a>(
    element: &'a UiElement,
    element_type: &str,
    results: &mut Vec<&'a UiElement>,
) {
    if element.element_type == element_type {
        results.push(element);
    }
    for child in &element.children {
        collect_by_type(child, element_type, results);
    }
}

fn display_label(element: &UiElement) -> String {
    element
        .label
        .as_deref()
        .or(element.value.as_deref())
        .unwrap_or("")
        .to_string()
}

fn selector_desc(selector: &Selector) -> String {
    match selector {
        Selector::Text(text) => format!("text \"{}\"", text),
        Selector::Type(t, Some(i)) => format!("type \"{}\"[{}]", t, i),
        Selector::Type(t, None) => format!("type \"{}\"", t),
        Selector::Ref(id) => format!("ref [{}]", id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::ui_tree::Frame;

    fn sample_elements() -> Vec<UiElement> {
        vec![UiElement {
            element_type: "Window".to_string(),
            label: Some("MainWindow".to_string()),
            value: None,
            frame: Frame {
                x: 0.0,
                y: 0.0,
                width: 393.0,
                height: 852.0,
            },
            enabled: true,
            traits: vec![],
            children: vec![
                UiElement {
                    element_type: "Button".to_string(),
                    label: Some("Login".to_string()),
                    value: None,
                    frame: Frame {
                        x: 100.0,
                        y: 400.0,
                        width: 193.0,
                        height: 44.0,
                    },
                    enabled: true,
                    traits: vec!["button".to_string()],
                    children: vec![],
                },
                UiElement {
                    element_type: "TextField".to_string(),
                    label: Some("Email".to_string()),
                    value: Some("user@example.com".to_string()),
                    frame: Frame {
                        x: 50.0,
                        y: 300.0,
                        width: 293.0,
                        height: 44.0,
                    },
                    enabled: true,
                    traits: vec![],
                    children: vec![],
                },
                UiElement {
                    element_type: "StaticText".to_string(),
                    label: Some("Welcome".to_string()),
                    value: None,
                    frame: Frame {
                        x: 50.0,
                        y: 200.0,
                        width: 293.0,
                        height: 30.0,
                    },
                    enabled: false,
                    traits: vec![],
                    children: vec![],
                },
                UiElement {
                    element_type: "Button".to_string(),
                    label: Some("Sign Up".to_string()),
                    value: None,
                    frame: Frame {
                        x: 100.0,
                        y: 500.0,
                        width: 193.0,
                        height: 44.0,
                    },
                    enabled: false,
                    traits: vec![],
                    children: vec![],
                },
                UiElement {
                    element_type: "View".to_string(),
                    label: None,
                    value: None,
                    frame: Frame {
                        x: 0.0,
                        y: 0.0,
                        width: 0.0,
                        height: 0.0,
                    },
                    enabled: true,
                    traits: vec![],
                    children: vec![],
                },
            ],
        }]
    }

    // -------------------------------------------------------------------
    // check_element_condition
    // -------------------------------------------------------------------

    #[test]
    fn test_visible_pass() {
        let elements = sample_elements();
        let selector = Selector::Text("Login".to_string());
        let result =
            check_element_condition(&elements, "test-udid", &selector, &Condition::Visible)
                .unwrap();
        assert!(result.passed);
        assert!(result.message.contains("is visible"));
    }

    #[test]
    fn test_visible_zero_size_fail() {
        let elements = sample_elements();
        // The zero-size View
        let selector = Selector::Type("View".to_string(), None);
        let result =
            check_element_condition(&elements, "test-udid", &selector, &Condition::Visible)
                .unwrap();
        assert!(!result.passed);
        assert!(result.message.contains("zero size"));
    }

    #[test]
    fn test_visible_not_found_fail() {
        let elements = sample_elements();
        let selector = Selector::Text("NonExistent".to_string());
        let result =
            check_element_condition(&elements, "test-udid", &selector, &Condition::Visible)
                .unwrap();
        assert!(!result.passed);
        assert!(result.message.contains("not found"));
    }

    #[test]
    fn test_hidden_not_found_pass() {
        let elements = sample_elements();
        let selector = Selector::Text("NonExistent".to_string());
        let result =
            check_element_condition(&elements, "test-udid", &selector, &Condition::Hidden)
                .unwrap();
        assert!(result.passed);
        assert!(result.message.contains("hidden"));
    }

    #[test]
    fn test_hidden_visible_element_fail() {
        let elements = sample_elements();
        let selector = Selector::Text("Login".to_string());
        let result =
            check_element_condition(&elements, "test-udid", &selector, &Condition::Hidden)
                .unwrap();
        assert!(!result.passed);
        assert!(result.message.contains("still visible"));
    }

    #[test]
    fn test_hidden_zero_size_pass() {
        let elements = sample_elements();
        let selector = Selector::Type("View".to_string(), None);
        let result =
            check_element_condition(&elements, "test-udid", &selector, &Condition::Hidden)
                .unwrap();
        assert!(result.passed);
        assert!(result.message.contains("zero size"));
    }

    #[test]
    fn test_enabled_pass() {
        let elements = sample_elements();
        let selector = Selector::Text("Login".to_string());
        let result =
            check_element_condition(&elements, "test-udid", &selector, &Condition::Enabled)
                .unwrap();
        assert!(result.passed);
        assert!(result.message.contains("is enabled"));
    }

    #[test]
    fn test_enabled_fail() {
        let elements = sample_elements();
        let selector = Selector::Text("Welcome".to_string());
        let result =
            check_element_condition(&elements, "test-udid", &selector, &Condition::Enabled)
                .unwrap();
        assert!(!result.passed);
        assert!(result.message.contains("is disabled"));
    }

    #[test]
    fn test_disabled_pass() {
        let elements = sample_elements();
        let selector = Selector::Text("Welcome".to_string());
        let result =
            check_element_condition(&elements, "test-udid", &selector, &Condition::Disabled)
                .unwrap();
        assert!(result.passed);
        assert!(result.message.contains("is disabled"));
    }

    #[test]
    fn test_disabled_fail() {
        let elements = sample_elements();
        let selector = Selector::Text("Login".to_string());
        let result =
            check_element_condition(&elements, "test-udid", &selector, &Condition::Disabled)
                .unwrap();
        assert!(!result.passed);
        assert!(result.message.contains("is enabled"));
    }

    #[test]
    fn test_text_equals_pass() {
        let elements = sample_elements();
        let selector = Selector::Text("Login".to_string());
        let result = check_element_condition(
            &elements,
            "test-udid",
            &selector,
            &Condition::TextEquals("Login".to_string()),
        )
        .unwrap();
        assert!(result.passed);
        assert!(result.message.contains("equals"));
    }

    #[test]
    fn test_text_equals_fail() {
        let elements = sample_elements();
        let selector = Selector::Text("Login".to_string());
        let result = check_element_condition(
            &elements,
            "test-udid",
            &selector,
            &Condition::TextEquals("Logout".to_string()),
        )
        .unwrap();
        assert!(!result.passed);
        assert!(result.message.contains("expected"));
    }

    #[test]
    fn test_text_contains_pass() {
        let elements = sample_elements();
        let selector = Selector::Text("Login".to_string());
        let result = check_element_condition(
            &elements,
            "test-udid",
            &selector,
            &Condition::TextContains("log".to_string()),
        )
        .unwrap();
        assert!(result.passed);
        assert!(result.message.contains("contains"));
    }

    #[test]
    fn test_text_contains_fail() {
        let elements = sample_elements();
        let selector = Selector::Text("Login".to_string());
        let result = check_element_condition(
            &elements,
            "test-udid",
            &selector,
            &Condition::TextContains("xyz".to_string()),
        )
        .unwrap();
        assert!(!result.passed);
        assert!(result.message.contains("does not contain"));
    }

    // -------------------------------------------------------------------
    // check_count
    // -------------------------------------------------------------------

    #[test]
    fn test_count_by_type_pass() {
        let elements = sample_elements();
        let selector = Selector::Type("Button".to_string(), None);
        let result = check_count(&elements, &selector, 2).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_count_by_type_fail() {
        let elements = sample_elements();
        let selector = Selector::Type("Button".to_string(), None);
        let result = check_count(&elements, &selector, 5).unwrap();
        assert!(!result.passed);
        assert!(result.message.contains("Found 2"));
    }

    #[test]
    fn test_count_by_text() {
        let elements = sample_elements();
        let selector = Selector::Text("Login".to_string());
        let result = check_count(&elements, &selector, 1).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_count_zero() {
        let elements = sample_elements();
        let selector = Selector::Type("Switch".to_string(), None);
        let result = check_count(&elements, &selector, 0).unwrap();
        assert!(result.passed);
    }

    // -------------------------------------------------------------------
    // resolve_selector
    // -------------------------------------------------------------------

    #[test]
    fn test_resolve_by_text() {
        let elements = sample_elements();
        let el = resolve_selector(
            &elements,
            "test-udid",
            &Selector::Text("Login".to_string()),
        )
        .unwrap();
        assert_eq!(el.element_type, "Button");
        assert_eq!(el.label.as_deref(), Some("Login"));
    }

    #[test]
    fn test_resolve_by_text_prefers_enabled() {
        let elements = sample_elements();
        // "Sign Up" is disabled, "Login" is enabled - both are buttons
        // Searching for a text that matches a disabled element
        let el = resolve_selector(
            &elements,
            "test-udid",
            &Selector::Text("Sign Up".to_string()),
        )
        .unwrap();
        // Should find Sign Up since it's the only match
        assert_eq!(el.label.as_deref(), Some("Sign Up"));
    }

    #[test]
    fn test_resolve_by_type() {
        let elements = sample_elements();
        let el = resolve_selector(
            &elements,
            "test-udid",
            &Selector::Type("TextField".to_string(), None),
        )
        .unwrap();
        assert_eq!(el.element_type, "TextField");
        assert_eq!(el.label.as_deref(), Some("Email"));
    }

    #[test]
    fn test_resolve_by_type_with_index() {
        let elements = sample_elements();
        let el = resolve_selector(
            &elements,
            "test-udid",
            &Selector::Type("Button".to_string(), Some(1)),
        )
        .unwrap();
        assert_eq!(el.label.as_deref(), Some("Sign Up"));
    }

    #[test]
    fn test_resolve_by_type_index_out_of_range() {
        let elements = sample_elements();
        let result = resolve_selector(
            &elements,
            "test-udid",
            &Selector::Type("Button".to_string(), Some(10)),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has 2 elements"));
    }

    #[test]
    fn test_resolve_not_found() {
        let elements = sample_elements();
        let result = resolve_selector(
            &elements,
            "test-udid",
            &Selector::Text("NonExistent".to_string()),
        );
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------
    // count_matches
    // -------------------------------------------------------------------

    #[test]
    fn test_count_matches_by_type() {
        let elements = sample_elements();
        assert_eq!(
            count_matches(&elements, &Selector::Type("Button".to_string(), None)),
            2
        );
        assert_eq!(
            count_matches(&elements, &Selector::Type("TextField".to_string(), None)),
            1
        );
        assert_eq!(
            count_matches(&elements, &Selector::Type("Switch".to_string(), None)),
            0
        );
    }

    #[test]
    fn test_count_matches_by_text() {
        let elements = sample_elements();
        assert_eq!(
            count_matches(&elements, &Selector::Text("Login".to_string())),
            1
        );
        assert_eq!(
            count_matches(&elements, &Selector::Text("Welcome".to_string())),
            1
        );
    }

    // -------------------------------------------------------------------
    // helpers
    // -------------------------------------------------------------------

    #[test]
    fn test_display_label() {
        let el = UiElement {
            element_type: "Button".to_string(),
            label: Some("Login".to_string()),
            value: None,
            frame: Frame {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 44.0,
            },
            enabled: true,
            traits: vec![],
            children: vec![],
        };
        assert_eq!(display_label(&el), "Login");
    }

    #[test]
    fn test_display_label_value_fallback() {
        let el = UiElement {
            element_type: "TextField".to_string(),
            label: None,
            value: Some("typed text".to_string()),
            frame: Frame {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 44.0,
            },
            enabled: true,
            traits: vec![],
            children: vec![],
        };
        assert_eq!(display_label(&el), "typed text");
    }

    #[test]
    fn test_display_label_empty() {
        let el = UiElement {
            element_type: "View".to_string(),
            label: None,
            value: None,
            frame: Frame {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 44.0,
            },
            enabled: true,
            traits: vec![],
            children: vec![],
        };
        assert_eq!(display_label(&el), "");
    }

    #[test]
    fn test_selector_desc() {
        assert_eq!(
            selector_desc(&Selector::Text("Login".to_string())),
            "text \"Login\""
        );
        assert_eq!(
            selector_desc(&Selector::Type("Button".to_string(), None)),
            "type \"Button\""
        );
        assert_eq!(
            selector_desc(&Selector::Type("Button".to_string(), Some(2))),
            "type \"Button\"[2]"
        );
        assert_eq!(selector_desc(&Selector::Ref(5)), "ref [5]");
    }
}
