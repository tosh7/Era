// UI tree structured parser for idb describe-all output
//
// Parses the JSON output from `idb ui describe-all` into a structured
// tree of `UiElement` nodes, enabling label-based search, coordinate
// hit-testing, and flat iteration over the accessibility tree.

use serde_json::Value;

/// Frame rectangle of a UI element (in logical points)
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Frame {
    /// Returns the center point of this frame
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Returns true if the given point is inside this frame
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

/// A single UI element from the idb accessibility tree
#[derive(Debug, Clone)]
pub struct UiElement {
    /// Element type (e.g. "Button", "StaticText", "TextField")
    pub element_type: String,
    /// Accessibility label (AXLabel)
    pub label: Option<String>,
    /// Accessibility value (AXValue)
    pub value: Option<String>,
    /// Position and size in logical points
    pub frame: Frame,
    /// Whether the element is interactable
    pub enabled: bool,
    /// Accessibility traits
    pub traits: Vec<String>,
    /// Child elements
    pub children: Vec<UiElement>,
}

/// Parse the top-level JSON from `idb ui describe-all`.
///
/// The output can be either a single root object or an array of root objects.
/// Returns all root-level elements.
pub fn parse(json_str: &str) -> Result<Vec<UiElement>, String> {
    let value: Value =
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    match &value {
        Value::Array(arr) => {
            let elements: Vec<UiElement> = arr.iter().filter_map(UiElement::from_json).collect();
            Ok(elements)
        }
        Value::Object(_) => match UiElement::from_json(&value) {
            Some(el) => Ok(vec![el]),
            None => Ok(vec![]),
        },
        _ => Err("Expected JSON object or array".to_string()),
    }
}

impl UiElement {
    /// Parse a single UI element from a JSON value.
    ///
    /// Handles idb_companion's output format where accessibility properties
    /// use keys like `AXLabel`, `AXValue`, `type`, `frame`, `enabled`,
    /// and children are in `subelements`.
    pub fn from_json(value: &Value) -> Option<Self> {
        let obj = value.as_object()?;

        let element_type = obj
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let label = obj
            .get("AXLabel")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let value_str = obj
            .get("AXValue")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let frame = parse_frame(obj.get("frame"))?;

        let enabled = obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);

        let traits = obj
            .get("AXTraits")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let children = obj
            .get("subelements")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(UiElement::from_json).collect())
            .unwrap_or_default();

        Some(UiElement {
            element_type,
            label,
            value: value_str,
            frame,
            enabled,
            traits,
            children,
        })
    }

    /// Find all elements whose label contains the given substring (case-insensitive).
    pub fn find_by_label(&self, label: &str) -> Vec<&UiElement> {
        let mut results = Vec::new();
        self.find_by_label_recursive(label, &mut results);
        results
    }

    fn find_by_label_recursive<'a>(&'a self, label: &str, results: &mut Vec<&'a UiElement>) {
        if let Some(ref my_label) = self.label {
            if my_label.to_lowercase().contains(&label.to_lowercase()) {
                results.push(self);
            }
        }
        for child in &self.children {
            child.find_by_label_recursive(label, results);
        }
    }

    /// Find the deepest element whose frame contains the given point.
    ///
    /// Searches depth-first, returning the most deeply nested matching element.
    pub fn find_at_point(&self, x: f64, y: f64) -> Option<&UiElement> {
        if !self.frame.contains(x, y) {
            return None;
        }

        // Check children depth-first; return the deepest match
        for child in &self.children {
            if let Some(found) = child.find_at_point(x, y) {
                return Some(found);
            }
        }

        // No child contains the point, so this element is the deepest match
        Some(self)
    }

    /// Flatten the element tree into a DFS-ordered list of references.
    pub fn flatten(&self) -> Vec<&UiElement> {
        let mut results = Vec::new();
        self.flatten_recursive(&mut results);
        results
    }

    fn flatten_recursive<'a>(&'a self, results: &mut Vec<&'a UiElement>) {
        results.push(self);
        for child in &self.children {
            child.flatten_recursive(results);
        }
    }

    /// Find all elements matching the given type string.
    pub fn find_by_type(&self, element_type: &str) -> Vec<&UiElement> {
        let mut results = Vec::new();
        self.find_by_type_recursive(element_type, &mut results);
        results
    }

    fn find_by_type_recursive<'a>(
        &'a self,
        element_type: &str,
        results: &mut Vec<&'a UiElement>,
    ) {
        if self.element_type == element_type {
            results.push(self);
        }
        for child in &self.children {
            child.find_by_type_recursive(element_type, results);
        }
    }
}

fn parse_frame(value: Option<&Value>) -> Option<Frame> {
    let obj = value?.as_object()?;
    Some(Frame {
        x: obj.get("x")?.as_f64()?,
        y: obj.get("y")?.as_f64()?,
        width: obj.get("width")?.as_f64()?,
        height: obj.get("height")?.as_f64()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_json() -> &'static str {
        r#"{
            "type": "Window",
            "AXLabel": "MainWindow",
            "frame": {"x": 0, "y": 0, "width": 393, "height": 852},
            "enabled": true,
            "subelements": [
                {
                    "type": "Button",
                    "AXLabel": "Login",
                    "AXValue": null,
                    "frame": {"x": 100, "y": 400, "width": 193, "height": 44},
                    "enabled": true,
                    "subelements": []
                },
                {
                    "type": "TextField",
                    "AXLabel": "Username",
                    "AXValue": "user@example.com",
                    "frame": {"x": 50, "y": 300, "width": 293, "height": 44},
                    "enabled": true,
                    "subelements": []
                },
                {
                    "type": "StaticText",
                    "AXLabel": "Welcome",
                    "frame": {"x": 50, "y": 200, "width": 293, "height": 30},
                    "enabled": false,
                    "subelements": []
                },
                {
                    "type": "Button",
                    "AXLabel": "Login with Apple",
                    "frame": {"x": 100, "y": 500, "width": 193, "height": 44},
                    "enabled": true,
                    "subelements": []
                }
            ]
        }"#
    }

    fn nested_json() -> &'static str {
        r#"{
            "type": "Window",
            "AXLabel": "Root",
            "frame": {"x": 0, "y": 0, "width": 393, "height": 852},
            "enabled": true,
            "subelements": [
                {
                    "type": "View",
                    "frame": {"x": 10, "y": 100, "width": 200, "height": 300},
                    "enabled": true,
                    "subelements": [
                        {
                            "type": "Button",
                            "AXLabel": "Nested Button",
                            "frame": {"x": 20, "y": 150, "width": 100, "height": 44},
                            "enabled": true,
                            "subelements": []
                        }
                    ]
                }
            ]
        }"#
    }

    // ---------------------------------------------------------------
    // parse
    // ---------------------------------------------------------------

    #[test]
    fn test_parse_single_object() {
        let elements = parse(sample_json()).unwrap();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, "Window");
        assert_eq!(elements[0].children.len(), 4);
    }

    #[test]
    fn test_parse_array() {
        let json = format!("[{}]", sample_json());
        let elements = parse(&json).unwrap();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, "Window");
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse("not json").is_err());
    }

    #[test]
    fn test_parse_empty_array() {
        let elements = parse("[]").unwrap();
        assert!(elements.is_empty());
    }

    // ---------------------------------------------------------------
    // from_json
    // ---------------------------------------------------------------

    #[test]
    fn test_from_json_full_element() {
        let value: Value = serde_json::from_str(sample_json()).unwrap();
        let el = UiElement::from_json(&value).unwrap();
        assert_eq!(el.element_type, "Window");
        assert_eq!(el.label.as_deref(), Some("MainWindow"));
        assert!(el.enabled);
        assert_eq!(el.frame.width, 393.0);
        assert_eq!(el.frame.height, 852.0);
    }

    #[test]
    fn test_from_json_missing_label() {
        let json: Value = serde_json::from_str(
            r#"{"type": "View", "frame": {"x": 0, "y": 0, "width": 100, "height": 100}}"#,
        )
        .unwrap();
        let el = UiElement::from_json(&json).unwrap();
        assert!(el.label.is_none());
        assert!(el.enabled); // default true
    }

    #[test]
    fn test_from_json_missing_type() {
        let json: Value = serde_json::from_str(
            r#"{"frame": {"x": 0, "y": 0, "width": 100, "height": 100}}"#,
        )
        .unwrap();
        let el = UiElement::from_json(&json).unwrap();
        assert_eq!(el.element_type, "Unknown");
    }

    #[test]
    fn test_from_json_missing_frame() {
        let json: Value = serde_json::from_str(r#"{"type": "Button"}"#).unwrap();
        assert!(UiElement::from_json(&json).is_none());
    }

    #[test]
    fn test_from_json_with_traits() {
        let json: Value = serde_json::from_str(
            r#"{
                "type": "Button",
                "frame": {"x": 0, "y": 0, "width": 100, "height": 44},
                "AXTraits": ["button", "enabled"]
            }"#,
        )
        .unwrap();
        let el = UiElement::from_json(&json).unwrap();
        assert_eq!(el.traits, vec!["button", "enabled"]);
    }

    // ---------------------------------------------------------------
    // Frame
    // ---------------------------------------------------------------

    #[test]
    fn test_frame_center() {
        let frame = Frame {
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 40.0,
        };
        assert_eq!(frame.center(), (125.0, 220.0));
    }

    #[test]
    fn test_frame_contains() {
        let frame = Frame {
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 40.0,
        };
        assert!(frame.contains(125.0, 220.0)); // center
        assert!(frame.contains(100.0, 200.0)); // top-left corner
        assert!(frame.contains(150.0, 240.0)); // bottom-right corner
        assert!(!frame.contains(99.0, 220.0)); // left of frame
        assert!(!frame.contains(151.0, 220.0)); // right of frame
        assert!(!frame.contains(125.0, 199.0)); // above frame
        assert!(!frame.contains(125.0, 241.0)); // below frame
    }

    #[test]
    fn test_frame_zero_size() {
        let frame = Frame {
            x: 50.0,
            y: 50.0,
            width: 0.0,
            height: 0.0,
        };
        assert_eq!(frame.center(), (50.0, 50.0));
        assert!(frame.contains(50.0, 50.0));
        assert!(!frame.contains(50.1, 50.0));
    }

    // ---------------------------------------------------------------
    // find_by_label
    // ---------------------------------------------------------------

    #[test]
    fn test_find_by_label_exact() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        let results = root.find_by_label("Login");
        // Should match "Login" and "Login with Apple"
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_find_by_label_partial() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        let results = root.find_by_label("user");
        // Should match "Username" (case-insensitive)
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].element_type, "TextField");
    }

    #[test]
    fn test_find_by_label_case_insensitive() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        let results = root.find_by_label("welcome");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label.as_deref(), Some("Welcome"));
    }

    #[test]
    fn test_find_by_label_no_match() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        let results = root.find_by_label("NonExistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_by_label_includes_root() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        let results = root.find_by_label("MainWindow");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].element_type, "Window");
    }

    // ---------------------------------------------------------------
    // find_at_point
    // ---------------------------------------------------------------

    #[test]
    fn test_find_at_point_leaf() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        // Point inside the Login button (100,400 193x44)
        let found = root.find_at_point(150.0, 420.0).unwrap();
        assert_eq!(found.label.as_deref(), Some("Login"));
    }

    #[test]
    fn test_find_at_point_root_only() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        // Point inside root window but outside any child
        let found = root.find_at_point(5.0, 5.0).unwrap();
        assert_eq!(found.element_type, "Window");
    }

    #[test]
    fn test_find_at_point_outside() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        assert!(root.find_at_point(500.0, 500.0).is_none());
    }

    #[test]
    fn test_find_at_point_deepest_nested() {
        let elements = parse(nested_json()).unwrap();
        let root = &elements[0];
        // Point inside the nested button (20,150 100x44)
        let found = root.find_at_point(50.0, 170.0).unwrap();
        assert_eq!(found.label.as_deref(), Some("Nested Button"));
    }

    // ---------------------------------------------------------------
    // flatten
    // ---------------------------------------------------------------

    #[test]
    fn test_flatten_count() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        let flat = root.flatten();
        // Window + 4 children = 5
        assert_eq!(flat.len(), 5);
    }

    #[test]
    fn test_flatten_dfs_order() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        let flat = root.flatten();
        assert_eq!(flat[0].element_type, "Window");
        assert_eq!(flat[1].label.as_deref(), Some("Login"));
        assert_eq!(flat[2].label.as_deref(), Some("Username"));
        assert_eq!(flat[3].label.as_deref(), Some("Welcome"));
        assert_eq!(flat[4].label.as_deref(), Some("Login with Apple"));
    }

    #[test]
    fn test_flatten_nested() {
        let elements = parse(nested_json()).unwrap();
        let root = &elements[0];
        let flat = root.flatten();
        // Window > View > Button = 3
        assert_eq!(flat.len(), 3);
        assert_eq!(flat[0].element_type, "Window");
        assert_eq!(flat[1].element_type, "View");
        assert_eq!(flat[2].label.as_deref(), Some("Nested Button"));
    }

    // ---------------------------------------------------------------
    // find_by_type
    // ---------------------------------------------------------------

    #[test]
    fn test_find_by_type() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        let buttons = root.find_by_type("Button");
        assert_eq!(buttons.len(), 2);
    }

    #[test]
    fn test_find_by_type_no_match() {
        let elements = parse(sample_json()).unwrap();
        let root = &elements[0];
        let switches = root.find_by_type("Switch");
        assert!(switches.is_empty());
    }

    #[test]
    fn test_find_by_type_nested() {
        let elements = parse(nested_json()).unwrap();
        let root = &elements[0];
        let buttons = root.find_by_type("Button");
        assert_eq!(buttons.len(), 1);
        assert_eq!(buttons[0].label.as_deref(), Some("Nested Button"));
    }
}
