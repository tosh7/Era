// MCP stdio transport — JSON-RPC over stdin/stdout

use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use super::handlers;
use super::protocol::{JsonRpcRequest, JsonRpcResponse, INTERNAL_ERROR, METHOD_NOT_FOUND, PARSE_ERROR};
use super::tools;

/// Run the MCP server loop over stdio.
///
/// Reads JSON-RPC messages from stdin (one per line) and writes responses to stdout.
/// Handles MCP lifecycle: initialize, tools/list, tools/call, notifications.
pub fn run_stdio_loop() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    eprintln!("[era-mcp] Server starting on stdio...");

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[era-mcp] Failed to read stdin: {}", e);
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let response = JsonRpcResponse::error(None, PARSE_ERROR, format!("Parse error: {}", e));
                write_response(&mut stdout, &response);
                continue;
            }
        };

        let response = handle_request(&request);

        // Notifications (no id) don't get a response
        if request.id.is_none() {
            continue;
        }

        if let Some(resp) = response {
            write_response(&mut stdout, &resp);
        }
    }

    eprintln!("[era-mcp] Server shutting down.");
}

fn handle_request(request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let id = request.id.clone();

    match request.method.as_str() {
        // MCP lifecycle
        "initialize" => Some(handle_initialize(id)),
        "initialized" => None, // Notification, no response
        "ping" => Some(JsonRpcResponse::success(id, json!({}))),

        // Tool discovery
        "tools/list" => Some(handle_tools_list(id)),

        // Tool execution
        "tools/call" => Some(handle_tools_call(id, &request.params)),

        // Unknown method
        _ => Some(JsonRpcResponse::error(
            id,
            METHOD_NOT_FOUND,
            format!("Unknown method: {}", request.method),
        )),
    }
}

fn handle_initialize(id: Option<Value>) -> JsonRpcResponse {
    let result = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "era",
            "version": env!("CARGO_PKG_VERSION")
        }
    });

    eprintln!("[era-mcp] Initialized (protocol version: 2024-11-05)");
    JsonRpcResponse::success(id, result)
}

fn handle_tools_list(id: Option<Value>) -> JsonRpcResponse {
    let tool_defs = tools::all_tools();
    let tools_json: Vec<Value> = tool_defs
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.input_schema,
            })
        })
        .collect();

    JsonRpcResponse::success(id, json!({ "tools": tools_json }))
}

fn handle_tools_call(id: Option<Value>, params: &Value) -> JsonRpcResponse {
    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => {
            return JsonRpcResponse::error(
                id,
                INTERNAL_ERROR,
                "Missing 'name' in tools/call params".to_string(),
            );
        }
    };

    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    eprintln!("[era-mcp] tools/call: {} args={}", tool_name, args);

    let result = handlers::dispatch(tool_name, &args);

    let result_json = json!({
        "content": result.content,
        "isError": result.is_error,
    });

    JsonRpcResponse::success(id, result_json)
}

fn write_response(writer: &mut impl Write, response: &JsonRpcResponse) {
    match serde_json::to_string(response) {
        Ok(json) => {
            if let Err(e) = writeln!(writer, "{}", json) {
                eprintln!("[era-mcp] Failed to write response: {}", e);
            }
            if let Err(e) = writer.flush() {
                eprintln!("[era-mcp] Failed to flush stdout: {}", e);
            }
        }
        Err(e) => {
            eprintln!("[era-mcp] Failed to serialize response: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(method: &str, id: Option<Value>, params: Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }

    #[test]
    fn test_initialize() {
        let req = make_request("initialize", Some(json!(1)), json!({}));
        let resp = handle_request(&req).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(
            result.get("protocolVersion").and_then(|v| v.as_str()),
            Some("2024-11-05")
        );
        assert!(result.get("capabilities").is_some());
        assert_eq!(
            result
                .get("serverInfo")
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str()),
            Some("era")
        );
    }

    #[test]
    fn test_initialized_notification() {
        let req = make_request("initialized", None, json!({}));
        let resp = handle_request(&req);
        assert!(resp.is_none(), "Notifications should not return a response");
    }

    #[test]
    fn test_ping() {
        let req = make_request("ping", Some(json!(2)), json!({}));
        let resp = handle_request(&req).unwrap();
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_tools_list() {
        let req = make_request("tools/list", Some(json!(3)), json!({}));
        let resp = handle_request(&req).unwrap();
        let result = resp.result.unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 8);

        // Verify each tool has required fields
        for tool in tools {
            assert!(tool.get("name").is_some());
            assert!(tool.get("description").is_some());
            assert!(tool.get("inputSchema").is_some());
        }
    }

    #[test]
    fn test_tools_call_unknown_tool() {
        let req = make_request(
            "tools/call",
            Some(json!(4)),
            json!({
                "name": "nonexistent_tool",
                "arguments": {}
            }),
        );
        let resp = handle_request(&req).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result.get("isError").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn test_tools_call_missing_name() {
        let req = make_request("tools/call", Some(json!(5)), json!({}));
        let resp = handle_request(&req).unwrap();
        assert!(resp.error.is_some());
    }

    #[test]
    fn test_unknown_method() {
        let req = make_request("nonexistent/method", Some(json!(6)), json!({}));
        let resp = handle_request(&req).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, METHOD_NOT_FOUND);
    }

    #[test]
    fn test_json_rpc_response_serialization() {
        let resp = JsonRpcResponse::success(Some(json!(1)), json!({"test": true}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_json_rpc_error_serialization() {
        let resp = JsonRpcResponse::error(Some(json!(1)), -32600, "Bad request".to_string());
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\""));
        assert!(!json.contains("\"result\""));
    }
}
