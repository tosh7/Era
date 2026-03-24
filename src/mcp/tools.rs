// MCP tool definitions — schema for each iOS Simulator tool

use serde_json::json;

use super::protocol::ToolDefinition;

/// Return all available MCP tool definitions
pub fn all_tools() -> Vec<ToolDefinition> {
    vec![
        ios_session_connect(),
        ios_list_devices(),
        ios_snapshot(),
        ios_tap(),
        ios_fill(),
        ios_swipe(),
        ios_screenshot(),
        ios_assert(),
    ]
}

fn ios_session_connect() -> ToolDefinition {
    ToolDefinition {
        name: "ios_session_connect".to_string(),
        description: "Connect to an iOS Simulator. Auto-detects scale factor. Returns session info. Use 'booted' to connect to the currently booted device.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "device": {
                    "type": "string",
                    "description": "Device UDID, name, or 'booted' for the currently booted simulator"
                },
                "session_name": {
                    "type": "string",
                    "description": "Optional session alias (default: 'default')"
                }
            },
            "required": ["device"]
        }),
    }
}

fn ios_list_devices() -> ToolDefinition {
    ToolDefinition {
        name: "ios_list_devices".to_string(),
        description: "List available iOS Simulators with their UDID, name, runtime, and state.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "booted_only": {
                    "type": "boolean",
                    "description": "Only show booted simulators (default: false)"
                }
            }
        }),
    }
}

fn ios_snapshot() -> ToolDefinition {
    ToolDefinition {
        name: "ios_snapshot".to_string(),
        description: "Get a ref-numbered UI element tree of the current screen. Each element gets a [ref] number for use with ios_tap and ios_fill. This is the primary way to understand what's on screen.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "session": {
                    "type": "string",
                    "description": "Session name (default: uses default session)"
                },
                "interactive_only": {
                    "type": "boolean",
                    "description": "Only show interactive elements like buttons and text fields (default: false)"
                },
                "filter_type": {
                    "type": "string",
                    "description": "Filter by element type (e.g. 'Button', 'TextField', 'Cell')"
                }
            }
        }),
    }
}

fn ios_tap() -> ToolDefinition {
    ToolDefinition {
        name: "ios_tap".to_string(),
        description: "Tap on a UI element. Supports ref number (from ios_snapshot), text search (case-insensitive partial match), type+index, or raw coordinates. Prefer ref-based tapping after ios_snapshot.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "session": {
                    "type": "string",
                    "description": "Session name (default: uses default session)"
                },
                "ref": {
                    "type": "integer",
                    "description": "Ref number from ios_snapshot output"
                },
                "text": {
                    "type": "string",
                    "description": "Tap element matching this text (case-insensitive partial match on label/value)"
                },
                "type": {
                    "type": "string",
                    "description": "Tap element matching this type (e.g. 'Button', 'Cell', 'TextField')"
                },
                "index": {
                    "type": "integer",
                    "description": "0-based index when multiple elements match --type (default: 0)"
                },
                "x": {
                    "type": "integer",
                    "description": "X coordinate in logical points"
                },
                "y": {
                    "type": "integer",
                    "description": "Y coordinate in logical points"
                }
            }
        }),
    }
}

fn ios_fill() -> ToolDefinition {
    ToolDefinition {
        name: "ios_fill".to_string(),
        description: "Focus a text field and input text. Taps the target element first, then types. Supports ref number, text search, or type+index to find the field.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "session": {
                    "type": "string",
                    "description": "Session name (default: uses default session)"
                },
                "ref": {
                    "type": "integer",
                    "description": "Ref number from ios_snapshot output"
                },
                "target": {
                    "type": "string",
                    "description": "Find field by text (case-insensitive partial match on label/value)"
                },
                "type": {
                    "type": "string",
                    "description": "Find field by type (e.g. 'TextField', 'SecureTextField')"
                },
                "index": {
                    "type": "integer",
                    "description": "0-based index when multiple elements match type (default: 0)"
                },
                "value": {
                    "type": "string",
                    "description": "Text to input into the field"
                },
                "clear": {
                    "type": "boolean",
                    "description": "Clear existing text before input (default: false)"
                }
            },
            "required": ["value"]
        }),
    }
}

fn ios_swipe() -> ToolDefinition {
    ToolDefinition {
        name: "ios_swipe".to_string(),
        description: "Swipe gesture on the simulator screen. Use 'direction' for simple scrolling, or explicit start/end coordinates for precise control.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "session": {
                    "type": "string",
                    "description": "Session name (default: uses default session)"
                },
                "direction": {
                    "type": "string",
                    "enum": ["up", "down", "left", "right"],
                    "description": "Swipe direction shortcut. 'up' scrolls content up (finger moves up), 'down' scrolls content down."
                },
                "start_x": {
                    "type": "integer",
                    "description": "Start X coordinate in logical points"
                },
                "start_y": {
                    "type": "integer",
                    "description": "Start Y coordinate in logical points"
                },
                "end_x": {
                    "type": "integer",
                    "description": "End X coordinate in logical points"
                },
                "end_y": {
                    "type": "integer",
                    "description": "End Y coordinate in logical points"
                }
            }
        }),
    }
}

fn ios_screenshot() -> ToolDefinition {
    ToolDefinition {
        name: "ios_screenshot".to_string(),
        description: "Capture a screenshot of the simulator screen. Returns the image as base64 PNG or saves to a file path.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "session": {
                    "type": "string",
                    "description": "Session name (default: uses default session)"
                },
                "format": {
                    "type": "string",
                    "enum": ["base64", "file"],
                    "description": "Output format: 'base64' returns image data inline, 'file' saves to output path (default: 'file')"
                },
                "output": {
                    "type": "string",
                    "description": "File path for format=file (default: /tmp/era-screenshot.png)"
                }
            }
        }),
    }
}

fn ios_assert() -> ToolDefinition {
    ToolDefinition {
        name: "ios_assert".to_string(),
        description: "Assert the current UI state matches expectations. Checks if specified text is visible or not visible on screen. Returns pass/fail with details.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "session": {
                    "type": "string",
                    "description": "Session name (default: uses default session)"
                },
                "visible": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of text strings that should be visible on screen"
                },
                "not_visible": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of text strings that should NOT be visible on screen"
                }
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tools_count() {
        let tools = all_tools();
        assert_eq!(tools.len(), 8);
    }

    #[test]
    fn test_tool_names_unique() {
        let tools = all_tools();
        let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), original_len, "Tool names must be unique");
    }

    #[test]
    fn test_all_tools_have_descriptions() {
        let tools = all_tools();
        for tool in &tools {
            assert!(!tool.description.is_empty(), "Tool {} has empty description", tool.name);
        }
    }

    #[test]
    fn test_all_tools_have_valid_schemas() {
        let tools = all_tools();
        for tool in &tools {
            let schema = &tool.input_schema;
            assert_eq!(
                schema.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "Tool {} schema must be an object",
                tool.name
            );
            assert!(
                schema.get("properties").is_some(),
                "Tool {} schema must have properties",
                tool.name
            );
        }
    }

    #[test]
    fn test_ios_fill_requires_value() {
        let tools = all_tools();
        let fill = tools.iter().find(|t| t.name == "ios_fill").unwrap();
        let required = fill.input_schema.get("required").unwrap().as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("value")));
    }

    #[test]
    fn test_ios_session_connect_requires_device() {
        let tools = all_tools();
        let connect = tools.iter().find(|t| t.name == "ios_session_connect").unwrap();
        let required = connect.input_schema.get("required").unwrap().as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("device")));
    }
}
