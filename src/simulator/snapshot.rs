// Snapshot module - ref-numbered UI tree output and mapping persistence
//
// Provides Playwright-style snapshot output: each UI element gets a [ref]
// number for subsequent interaction via `tap --ref` or `fill --ref`.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::ui_tree::UiElement;

/// A ref-numbered element with its center coordinates for tap resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefEntry {
    pub center_x: f64,
    pub center_y: f64,
    pub width: f64,
    pub height: f64,
    pub element_type: String,
    pub label: Option<String>,
}

/// Complete ref mapping for a snapshot session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefMap {
    pub entries: HashMap<u32, RefEntry>,
}

/// Options controlling snapshot output format
pub struct SnapshotOptions {
    /// Include frame coordinates in output
    pub verbose: bool,
    /// Only show interactive (enabled + tappable) elements
    pub interactive_only: bool,
    /// Filter by element type
    pub filter_type: Option<String>,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            verbose: false,
            interactive_only: false,
            filter_type: None,
        }
    }
}

/// Build a ref-numbered snapshot from parsed UI elements.
///
/// Assigns ref numbers in DFS order starting from 1.
/// Returns the formatted text output and the ref mapping.
pub fn build_snapshot(roots: &[UiElement], options: &SnapshotOptions) -> (String, RefMap) {
    let mut output = String::new();
    let mut ref_map = RefMap {
        entries: HashMap::new(),
    };
    let mut ref_counter: u32 = 0;

    for root in roots {
        build_snapshot_recursive(root, 0, &mut ref_counter, &mut output, &mut ref_map, options);
    }

    (output, ref_map)
}

fn build_snapshot_recursive(
    element: &UiElement,
    depth: usize,
    ref_counter: &mut u32,
    output: &mut String,
    ref_map: &mut RefMap,
    options: &SnapshotOptions,
) {
    // Filter: skip zero-size elements
    if element.frame.width <= 0.0 && element.frame.height <= 0.0 {
        return;
    }

    // Filter: interactive only
    if options.interactive_only && !is_interactive(element) {
        // Still recurse into children — a non-interactive container may have interactive children
        for child in &element.children {
            build_snapshot_recursive(child, depth, ref_counter, output, ref_map, options);
        }
        return;
    }

    // Filter: type filter
    if let Some(ref filter_type) = options.filter_type {
        if element.element_type != *filter_type {
            for child in &element.children {
                build_snapshot_recursive(child, depth, ref_counter, output, ref_map, options);
            }
            return;
        }
    }

    // Assign ref number
    *ref_counter += 1;
    let ref_id = *ref_counter;

    // Store in ref map
    let (cx, cy) = element.frame.center();
    ref_map.entries.insert(
        ref_id,
        RefEntry {
            center_x: cx,
            center_y: cy,
            width: element.frame.width,
            height: element.frame.height,
            element_type: element.element_type.clone(),
            label: element.label.clone(),
        },
    );

    // Format output line
    let indent = "  ".repeat(depth);
    let label_part = format_label(element);
    let traits_part = format_traits(element);

    if options.verbose {
        let frame_part = format!(
            " (x:{} y:{} w:{} h:{})",
            element.frame.x as i32,
            element.frame.y as i32,
            element.frame.width as i32,
            element.frame.height as i32
        );
        output.push_str(&format!(
            "{}[{}] {}{}{}{}",
            indent, ref_id, element.element_type, label_part, frame_part, traits_part
        ));
    } else {
        output.push_str(&format!(
            "{}[{}] {}{}{}",
            indent, ref_id, element.element_type, label_part, traits_part
        ));
    }
    output.push('\n');

    // Recurse into children
    for child in &element.children {
        build_snapshot_recursive(child, depth + 1, ref_counter, output, ref_map, options);
    }
}

fn format_label(element: &UiElement) -> String {
    if let Some(ref label) = element.label {
        if !label.is_empty() {
            return format!(" \"{}\"", label);
        }
    }
    if let Some(ref value) = element.value {
        if !value.is_empty() {
            return format!(" \"{}\"", value);
        }
    }
    String::new()
}

fn format_traits(element: &UiElement) -> String {
    if element.traits.is_empty() {
        return String::new();
    }
    format!(" {{{}}}", element.traits.join(", "))
}

/// Determine if an element is likely interactive (tappable/fillable)
fn is_interactive(element: &UiElement) -> bool {
    if !element.enabled {
        return false;
    }
    let interactive_types = [
        "Button",
        "TextField",
        "SecureTextField",
        "TextArea",
        "Switch",
        "Slider",
        "Stepper",
        "Picker",
        "SegmentedControl",
        "Link",
        "Cell",
        "Tab",
        "MenuItem",
        "Toggle",
    ];
    interactive_types
        .iter()
        .any(|t| element.element_type.contains(t))
}

// -------------------------------------------------------------------
// Ref map file persistence
// -------------------------------------------------------------------

/// Path for the snapshot ref map file
pub fn ref_map_path(udid: &str) -> String {
    format!("/tmp/era-snapshot-{}.json", udid)
}

/// Save a ref map to disk
pub fn save_ref_map(udid: &str, ref_map: &RefMap) -> Result<(), String> {
    let path = ref_map_path(udid);
    let json =
        serde_json::to_string_pretty(ref_map).map_err(|e| format!("Failed to serialize: {}", e))?;

    let mut file =
        fs::File::create(&path).map_err(|e| format!("Failed to create {}: {}", path, e))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write {}: {}", path, e))?;

    Ok(())
}

/// Load a ref map from disk
pub fn load_ref_map(udid: &str) -> Result<RefMap, String> {
    let path = ref_map_path(udid);
    if !Path::new(&path).exists() {
        return Err(format!(
            "No snapshot found. Run `era snapshot` first. (expected: {})",
            path
        ));
    }
    let json = fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path, e))?;
    let ref_map: RefMap =
        serde_json::from_str(&json).map_err(|e| format!("Failed to parse {}: {}", path, e))?;
    Ok(ref_map)
}

/// Resolve a ref ID to center coordinates
pub fn resolve_ref(udid: &str, ref_id: u32) -> Result<(f64, f64), String> {
    let ref_map = load_ref_map(udid)?;
    match ref_map.entries.get(&ref_id) {
        Some(entry) => {
            let label_info = entry
                .label
                .as_ref()
                .map(|l| format!(" \"{}\"", l))
                .unwrap_or_default();
            eprintln!(
                "[era] Resolved ref [{}] -> {}{} at ({:.1}, {:.1})",
                ref_id, entry.element_type, label_info, entry.center_x, entry.center_y
            );
            Ok((entry.center_x, entry.center_y))
        }
        None => Err(format!(
            "Ref [{}] not found. Run `era snapshot` to refresh.",
            ref_id
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::ui_tree::{self, Frame};

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
                    label: Some("ログイン".to_string()),
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
                    label: Some("メールアドレス".to_string()),
                    value: None,
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

    #[test]
    fn test_build_snapshot_default() {
        let elements = sample_elements();
        let options = SnapshotOptions::default();
        let (output, ref_map) = build_snapshot(&elements, &options);

        // Zero-size View should be excluded
        assert!(!output.contains("View"));
        // Window, Button, TextField, StaticText = 4 elements
        assert_eq!(ref_map.entries.len(), 4);
        // Check DFS order
        assert!(output.starts_with("[1] Window \"MainWindow\""));
        assert!(output.contains("  [2] Button \"ログイン\""));
        assert!(output.contains("  [3] TextField \"メールアドレス\""));
        assert!(output.contains("  [4] StaticText \"Welcome\""));
    }

    #[test]
    fn test_build_snapshot_verbose() {
        let elements = sample_elements();
        let options = SnapshotOptions {
            verbose: true,
            ..Default::default()
        };
        let (output, _) = build_snapshot(&elements, &options);

        assert!(output.contains("(x:100 y:400 w:193 h:44)"));
    }

    #[test]
    fn test_build_snapshot_interactive_only() {
        let elements = sample_elements();
        let options = SnapshotOptions {
            interactive_only: true,
            ..Default::default()
        };
        let (output, ref_map) = build_snapshot(&elements, &options);

        // Only Button and TextField are interactive
        assert_eq!(ref_map.entries.len(), 2);
        assert!(output.contains("Button"));
        assert!(output.contains("TextField"));
        assert!(!output.contains("StaticText"));
        assert!(!output.contains("Window"));
    }

    #[test]
    fn test_build_snapshot_filter_type() {
        let elements = sample_elements();
        let options = SnapshotOptions {
            filter_type: Some("Button".to_string()),
            ..Default::default()
        };
        let (output, ref_map) = build_snapshot(&elements, &options);

        assert_eq!(ref_map.entries.len(), 1);
        assert!(output.contains("Button \"ログイン\""));
        assert!(!output.contains("TextField"));
    }

    #[test]
    fn test_build_snapshot_traits() {
        let elements = sample_elements();
        let options = SnapshotOptions::default();
        let (output, _) = build_snapshot(&elements, &options);

        assert!(output.contains("{button}"));
    }

    #[test]
    fn test_ref_map_center_coordinates() {
        let elements = sample_elements();
        let options = SnapshotOptions::default();
        let (_, ref_map) = build_snapshot(&elements, &options);

        // Button at (100, 400, 193x44) -> center (196.5, 422.0)
        let button_ref = ref_map.entries.get(&2).unwrap();
        assert_eq!(button_ref.center_x, 196.5);
        assert_eq!(button_ref.center_y, 422.0);
        assert_eq!(button_ref.element_type, "Button");
        assert_eq!(button_ref.label.as_deref(), Some("ログイン"));
    }

    #[test]
    fn test_ref_map_save_load() {
        let elements = sample_elements();
        let options = SnapshotOptions::default();
        let (_, ref_map) = build_snapshot(&elements, &options);

        let udid = "test-snapshot-save-load";
        save_ref_map(udid, &ref_map).unwrap();

        let loaded = load_ref_map(udid).unwrap();
        assert_eq!(loaded.entries.len(), ref_map.entries.len());

        // Verify a specific entry survived round-trip
        let entry = loaded.entries.get(&2).unwrap();
        assert_eq!(entry.element_type, "Button");
        assert_eq!(entry.label.as_deref(), Some("ログイン"));

        // Cleanup
        let _ = fs::remove_file(ref_map_path(udid));
    }

    #[test]
    fn test_resolve_ref_success() {
        let elements = sample_elements();
        let options = SnapshotOptions::default();
        let (_, ref_map) = build_snapshot(&elements, &options);

        let udid = "test-resolve-ref";
        save_ref_map(udid, &ref_map).unwrap();

        let (x, y) = resolve_ref(udid, 2).unwrap();
        assert_eq!(x, 196.5);
        assert_eq!(y, 422.0);

        // Cleanup
        let _ = fs::remove_file(ref_map_path(udid));
    }

    #[test]
    fn test_resolve_ref_not_found() {
        let elements = sample_elements();
        let options = SnapshotOptions::default();
        let (_, ref_map) = build_snapshot(&elements, &options);

        let udid = "test-resolve-ref-notfound";
        save_ref_map(udid, &ref_map).unwrap();

        let result = resolve_ref(udid, 999);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));

        // Cleanup
        let _ = fs::remove_file(ref_map_path(udid));
    }

    #[test]
    fn test_resolve_ref_no_snapshot_file() {
        let result = resolve_ref("nonexistent-device-xyz", 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No snapshot found"));
    }

    #[test]
    fn test_build_snapshot_from_parsed_json() {
        let json = r#"{
            "type": "Window",
            "AXLabel": "Root",
            "frame": {"x": 0, "y": 0, "width": 393, "height": 852},
            "enabled": true,
            "subelements": [
                {
                    "type": "Button",
                    "AXLabel": "タップ",
                    "frame": {"x": 10, "y": 100, "width": 100, "height": 44},
                    "enabled": true,
                    "subelements": []
                }
            ]
        }"#;
        let elements = ui_tree::parse(json).unwrap();
        let options = SnapshotOptions::default();
        let (output, ref_map) = build_snapshot(&elements, &options);

        assert_eq!(ref_map.entries.len(), 2);
        assert!(output.contains("[1] Window \"Root\""));
        assert!(output.contains("  [2] Button \"タップ\""));
    }

    #[test]
    fn test_format_label_with_value_fallback() {
        let element = UiElement {
            element_type: "TextField".to_string(),
            label: None,
            value: Some("入力済みテキスト".to_string()),
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
        let elements = vec![element];
        let options = SnapshotOptions::default();
        let (output, _) = build_snapshot(&elements, &options);

        assert!(output.contains("\"入力済みテキスト\""));
    }
}
