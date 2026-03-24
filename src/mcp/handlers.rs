// MCP tool handlers — dispatches tool calls to simulator operations

use serde_json::{json, Value};

use crate::capture::{CaptureConfig, ObservationPolicy};
use crate::simulator::{idb, operations, session, snapshot, ui_tree};

use super::protocol::ToolResult;

/// Dispatch a tool call by name to the appropriate handler
pub fn dispatch(tool_name: &str, args: &Value) -> ToolResult {
    match tool_name {
        "ios_session_connect" => handle_session_connect(args),
        "ios_list_devices" => handle_list_devices(args),
        "ios_snapshot" => handle_snapshot(args),
        "ios_tap" => handle_tap(args),
        "ios_fill" => handle_fill(args),
        "ios_swipe" => handle_swipe(args),
        "ios_screenshot" => handle_screenshot(args),
        "ios_assert" => handle_assert(args),
        _ => ToolResult::error(format!("Unknown tool: {}", tool_name)),
    }
}

// ---------------------------------------------------------------------------
// Session connect
// ---------------------------------------------------------------------------

fn handle_session_connect(args: &Value) -> ToolResult {
    let device = match args.get("device").and_then(|v| v.as_str()) {
        Some(d) => d,
        None => return ToolResult::error("Missing required parameter: device".to_string()),
    };
    let session_name = args
        .get("session_name")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    // Resolve UDID from device identifier
    let udid = match resolve_udid(device) {
        Ok(u) => u,
        Err(e) => return ToolResult::error(e),
    };

    // Create session via SessionStore
    match session::SessionStore::connect(session_name, &udid, device) {
        Ok(sess) => {
            let result = json!({
                "session": sess.name,
                "udid": sess.udid,
                "device_name": sess.device_name,
                "scale": sess.scale,
            });
            ToolResult::text(serde_json::to_string_pretty(&result).unwrap_or_default())
        }
        Err(e) => ToolResult::error(format!("Failed to connect session: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// List devices
// ---------------------------------------------------------------------------

fn handle_list_devices(args: &Value) -> ToolResult {
    let booted_only = args
        .get("booted_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match operations::list_devices() {
        Ok(devices) => {
            let filtered: Vec<_> = if booted_only {
                devices.into_iter().filter(|d| d.device.is_booted()).collect()
            } else {
                devices
            };

            let items: Vec<Value> = filtered
                .iter()
                .map(|d| {
                    json!({
                        "udid": d.device.udid,
                        "name": d.device.name,
                        "runtime": d.runtime_name,
                        "state": if d.device.is_booted() { "Booted" } else { "Shutdown" },
                    })
                })
                .collect();

            let result = json!({ "devices": items, "count": items.len() });
            ToolResult::text(serde_json::to_string_pretty(&result).unwrap_or_default())
        }
        Err(e) => ToolResult::error(format!("Failed to list devices: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------

fn handle_snapshot(args: &Value) -> ToolResult {
    let udid = match resolve_session_device(args) {
        Ok(u) => u,
        Err(e) => return ToolResult::error(e),
    };

    let interactive_only = args
        .get("interactive_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let filter_type = args
        .get("filter_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Get UI tree from device
    let json_output = match idb::describe_all(&udid) {
        Ok(o) => o,
        Err(e) => return ToolResult::error(format!("Failed to get UI tree: {}", e)),
    };

    let elements = match ui_tree::parse(&json_output) {
        Ok(e) => e,
        Err(e) => return ToolResult::error(format!("Failed to parse UI tree: {}", e)),
    };

    let options = snapshot::SnapshotOptions {
        verbose: false,
        interactive_only,
        filter_type,
    };

    let (output, ref_map) = snapshot::build_snapshot(&elements, &options);
    let count = ref_map.entries.len();

    if let Err(e) = snapshot::save_ref_map(&udid, &ref_map) {
        return ToolResult::error(format!("Failed to save ref map: {}", e));
    }

    ToolResult::text(format!(
        "{}\n- {} elements indexed. Use ref numbers with ios_tap or ios_fill.",
        output.trim_end(),
        count
    ))
}

// ---------------------------------------------------------------------------
// Tap
// ---------------------------------------------------------------------------

fn handle_tap(args: &Value) -> ToolResult {
    let udid = match resolve_session_device(args) {
        Ok(u) => u,
        Err(e) => return ToolResult::error(e),
    };

    let config = CaptureConfig::new(ObservationPolicy::OnFailure, false, String::new());

    // Ref-based tap
    if let Some(ref_id) = args.get("ref").and_then(|v| v.as_u64()) {
        let ref_id = ref_id as u32;
        let (px, py) = match snapshot::resolve_ref(&udid, ref_id) {
            Ok(coords) => coords,
            Err(e) => return ToolResult::error(e),
        };

        if let Err(e) = idb::tap_with_retry(&udid, px, py, &config) {
            return ToolResult::error(format!("Tap failed: {}", e));
        }

        let ref_entry = snapshot::load_ref_map(&udid)
            .ok()
            .and_then(|m| m.entries.get(&ref_id).cloned());

        let label = ref_entry
            .as_ref()
            .and_then(|e| e.label.as_deref())
            .unwrap_or("");
        let element_type = ref_entry
            .as_ref()
            .map(|e| e.element_type.as_str())
            .unwrap_or("");

        return ToolResult::text(format!(
            "Tapped ref [{}] {} \"{}\" at ({:.1}, {:.1})",
            ref_id, element_type, label, px, py
        ));
    }

    // Text-based tap
    if let Some(text) = args.get("text").and_then(|v| v.as_str()) {
        return tap_by_live_search(&udid, Some(text), None, None, &config);
    }

    // Type-based tap
    if let Some(element_type) = args.get("type").and_then(|v| v.as_str()) {
        let index = args.get("index").and_then(|v| v.as_u64()).map(|v| v as u32);
        return tap_by_live_search(&udid, None, Some(element_type), index, &config);
    }

    // Coordinate-based tap
    if let (Some(x), Some(y)) = (
        args.get("x").and_then(|v| v.as_u64()),
        args.get("y").and_then(|v| v.as_u64()),
    ) {
        let px = x as f64;
        let py = y as f64;

        if let Err(e) = idb::tap_with_retry(&udid, px, py, &config) {
            return ToolResult::error(format!("Tap failed: {}", e));
        }

        return ToolResult::text(format!("Tapped at point ({}, {})", x, y));
    }

    ToolResult::error(
        "No target specified. Provide one of: ref, text, type, or x+y coordinates.".to_string(),
    )
}

fn tap_by_live_search(
    udid: &str,
    text: Option<&str>,
    element_type: Option<&str>,
    index: Option<u32>,
    config: &CaptureConfig,
) -> ToolResult {
    let json_output = match idb::describe_all(udid) {
        Ok(o) => o,
        Err(e) => return ToolResult::error(format!("Failed to get UI tree: {}", e)),
    };
    let elements = match ui_tree::parse(&json_output) {
        Ok(e) => e,
        Err(e) => return ToolResult::error(format!("Failed to parse UI tree: {}", e)),
    };

    let (px, py, desc) = if let Some(text) = text {
        match snapshot::find_by_text(&elements, text) {
            Ok(r) => r,
            Err(e) => return ToolResult::error(e),
        }
    } else if let Some(element_type) = element_type {
        match snapshot::find_by_type_index(&elements, element_type, index) {
            Ok(r) => r,
            Err(e) => return ToolResult::error(e),
        }
    } else {
        return ToolResult::error("No selector provided".to_string());
    };

    if let Err(e) = idb::tap_with_retry(udid, px, py, config) {
        return ToolResult::error(format!("Tap failed: {}", e));
    }

    ToolResult::text(format!("Tapped {} at ({:.1}, {:.1})", desc, px, py))
}

// ---------------------------------------------------------------------------
// Fill
// ---------------------------------------------------------------------------

fn handle_fill(args: &Value) -> ToolResult {
    let udid = match resolve_session_device(args) {
        Ok(u) => u,
        Err(e) => return ToolResult::error(e),
    };

    let value = match args.get("value").and_then(|v| v.as_str()) {
        Some(v) => v,
        None => return ToolResult::error("Missing required parameter: value".to_string()),
    };

    let clear = args
        .get("clear")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let config = CaptureConfig::new(ObservationPolicy::OnFailure, false, String::new());

    // Resolve target element
    let (px, py, desc) = if let Some(ref_id) = args.get("ref").and_then(|v| v.as_u64()) {
        let ref_id = ref_id as u32;
        match snapshot::resolve_ref(&udid, ref_id) {
            Ok((x, y)) => (x, y, format!("ref [{}]", ref_id)),
            Err(e) => return ToolResult::error(e),
        }
    } else if let Some(target) = args.get("target").and_then(|v| v.as_str()) {
        match live_search_element(&udid, Some(target), None, None) {
            Ok(r) => r,
            Err(e) => return ToolResult::error(e),
        }
    } else if let Some(element_type) = args.get("type").and_then(|v| v.as_str()) {
        let index = args.get("index").and_then(|v| v.as_u64()).map(|v| v as u32);
        match live_search_element(&udid, None, Some(element_type), index) {
            Ok(r) => r,
            Err(e) => return ToolResult::error(e),
        }
    } else {
        return ToolResult::error(
            "No target specified. Provide one of: ref, target (text), or type.".to_string(),
        );
    };

    // Tap to focus
    if let Err(e) = idb::tap_with_retry(&udid, px, py, &config) {
        return ToolResult::error(format!("Failed to focus element: {}", e));
    }

    // Clear if requested
    if clear {
        let _ = idb::tap(&udid, px, py);
        let _ = idb::tap(&udid, px, py);
        let _ = idb::send_key(&udid, "DELETE");
    }

    // Input text
    if let Err(e) = idb::text_input(&udid, value) {
        return ToolResult::error(format!("Failed to input text: {}", e));
    }

    ToolResult::text(format!(
        "Filled {} with \"{}\"{}",
        desc,
        value,
        if clear { " (cleared first)" } else { "" }
    ))
}

fn live_search_element(
    udid: &str,
    text: Option<&str>,
    element_type: Option<&str>,
    index: Option<u32>,
) -> Result<(f64, f64, String), String> {
    let json_output = idb::describe_all(udid).map_err(|e| format!("Failed to get UI tree: {}", e))?;
    let elements = ui_tree::parse(&json_output).map_err(|e| format!("Failed to parse UI tree: {}", e))?;

    if let Some(text) = text {
        snapshot::find_by_text(&elements, text)
    } else if let Some(element_type) = element_type {
        snapshot::find_by_type_index(&elements, element_type, index)
    } else {
        Err("No selector provided".to_string())
    }
}

// ---------------------------------------------------------------------------
// Swipe
// ---------------------------------------------------------------------------

fn handle_swipe(args: &Value) -> ToolResult {
    let udid = match resolve_session_device(args) {
        Ok(u) => u,
        Err(e) => return ToolResult::error(e),
    };

    // Direction shortcut
    if let Some(direction) = args.get("direction").and_then(|v| v.as_str()) {
        // Default screen center for direction-based swipes (iPhone-like 393x852)
        let (cx, cy) = (196.0, 426.0);
        let distance = 200.0;

        let (sx, sy, ex, ey) = match direction {
            "up" => (cx, cy + distance, cx, cy - distance),
            "down" => (cx, cy - distance, cx, cy + distance),
            "left" => (cx + distance, cy, cx - distance, cy),
            "right" => (cx - distance, cy, cx + distance, cy),
            _ => {
                return ToolResult::error(format!(
                    "Invalid direction: {}. Use up, down, left, or right.",
                    direction
                ))
            }
        };

        if let Err(e) = idb::swipe(&udid, sx, sy, ex, ey, None) {
            return ToolResult::error(format!("Swipe failed: {}", e));
        }

        return ToolResult::text(format!("Swiped {}", direction));
    }

    // Explicit coordinates
    let start_x = args.get("start_x").and_then(|v| v.as_f64());
    let start_y = args.get("start_y").and_then(|v| v.as_f64());
    let end_x = args.get("end_x").and_then(|v| v.as_f64());
    let end_y = args.get("end_y").and_then(|v| v.as_f64());

    match (start_x, start_y, end_x, end_y) {
        (Some(sx), Some(sy), Some(ex), Some(ey)) => {
            if let Err(e) = idb::swipe(&udid, sx, sy, ex, ey, None) {
                return ToolResult::error(format!("Swipe failed: {}", e));
            }
            ToolResult::text(format!(
                "Swiped from ({:.0}, {:.0}) to ({:.0}, {:.0})",
                sx, sy, ex, ey
            ))
        }
        _ => ToolResult::error(
            "Provide either 'direction' or all of start_x, start_y, end_x, end_y.".to_string(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Screenshot
// ---------------------------------------------------------------------------

fn handle_screenshot(args: &Value) -> ToolResult {
    let udid = match resolve_session_device(args) {
        Ok(u) => u,
        Err(e) => return ToolResult::error(e),
    };

    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("file");

    let output_path = args
        .get("output")
        .and_then(|v| v.as_str())
        .unwrap_or("/tmp/era-screenshot.png");

    match format {
        "base64" => {
            // Capture to temp file, read, encode
            let temp_path = format!("/tmp/era-mcp-screenshot-{}.png", std::process::id());
            let path = std::path::Path::new(&temp_path);

            if let Err(e) = operations::take_screenshot(&udid, path) {
                return ToolResult::error(format!("Screenshot failed: {}", e));
            }

            match std::fs::read(&temp_path) {
                Ok(bytes) => {
                    let _ = std::fs::remove_file(&temp_path);
                    use base64::Engine;
                    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    ToolResult::image(encoded, format!("Screenshot of {}", udid))
                }
                Err(e) => ToolResult::error(format!("Failed to read screenshot: {}", e)),
            }
        }
        _ => {
            let path = std::path::Path::new(output_path);
            if let Err(e) = operations::take_screenshot(&udid, path) {
                return ToolResult::error(format!("Screenshot failed: {}", e));
            }
            ToolResult::text(format!("Screenshot saved to: {}", output_path))
        }
    }
}

// ---------------------------------------------------------------------------
// Assert
// ---------------------------------------------------------------------------

fn handle_assert(args: &Value) -> ToolResult {
    let udid = match resolve_session_device(args) {
        Ok(u) => u,
        Err(e) => return ToolResult::error(e),
    };

    let json_output = match idb::describe_all(&udid) {
        Ok(o) => o,
        Err(e) => return ToolResult::error(format!("Failed to get UI tree: {}", e)),
    };
    let elements = match ui_tree::parse(&json_output) {
        Ok(e) => e,
        Err(e) => return ToolResult::error(format!("Failed to parse UI tree: {}", e)),
    };

    let mut checks: Vec<Value> = Vec::new();
    let mut all_pass = true;

    // Check visible texts
    if let Some(visible) = args.get("visible").and_then(|v| v.as_array()) {
        for item in visible {
            if let Some(text) = item.as_str() {
                let found = snapshot::find_by_text(&elements, text).is_ok();
                if !found {
                    all_pass = false;
                }
                checks.push(json!({
                    "check": "visible",
                    "text": text,
                    "found": found,
                    "pass": found,
                }));
            }
        }
    }

    // Check not_visible texts
    if let Some(not_visible) = args.get("not_visible").and_then(|v| v.as_array()) {
        for item in not_visible {
            if let Some(text) = item.as_str() {
                let found = snapshot::find_by_text(&elements, text).is_ok();
                let pass = !found;
                if !pass {
                    all_pass = false;
                }
                checks.push(json!({
                    "check": "not_visible",
                    "text": text,
                    "found": found,
                    "pass": pass,
                }));
            }
        }
    }

    if checks.is_empty() {
        return ToolResult::error(
            "No assertions specified. Provide 'visible' and/or 'not_visible' arrays.".to_string(),
        );
    }

    let result = json!({
        "pass": all_pass,
        "checks": checks,
    });

    if all_pass {
        ToolResult::text(format!("All assertions passed.\n{}", serde_json::to_string_pretty(&result).unwrap_or_default()))
    } else {
        ToolResult::text(format!("Some assertions failed.\n{}", serde_json::to_string_pretty(&result).unwrap_or_default()))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve UDID from session args (session name or default)
fn resolve_session_device(args: &Value) -> Result<String, String> {
    let session_name = args.get("session").and_then(|v| v.as_str());

    if let Some(name) = session_name {
        let sess = session::SessionStore::get(name)?;
        return Ok(sess.udid);
    }

    // Try default session
    match session::SessionStore::get_default() {
        Ok(sess) => Ok(sess.udid),
        Err(_) => Err(
            "No session connected. Use ios_session_connect first, or provide a 'session' parameter."
                .to_string(),
        ),
    }
}

/// Resolve a device name/UDID string to a UDID.
/// Supports "booted" as a special value.
fn resolve_udid(device: &str) -> Result<String, String> {
    if device == "booted" {
        // Find the first booted device
        let devices = operations::list_devices()
            .map_err(|e| format!("Failed to list devices: {}", e))?;
        let booted = devices.iter().find(|d| d.device.is_booted());
        return match booted {
            Some(d) => Ok(d.device.udid.clone()),
            None => Err("No booted simulator found. Boot one first.".to_string()),
        };
    }

    // If it looks like a UDID already, return as-is
    if device.contains('-') && device.len() > 20 {
        return Ok(device.to_string());
    }

    // Try to find by name
    let devices = operations::list_devices()
        .map_err(|e| format!("Failed to list devices: {}", e))?;
    for d in &devices {
        if d.device.name == device || d.device.udid == device {
            return Ok(d.device.udid.clone());
        }
    }

    // Return as-is if not found (might be a partial UDID)
    Ok(device.to_string())
}
