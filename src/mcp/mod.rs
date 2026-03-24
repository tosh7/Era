// MCP (Model Context Protocol) server module
//
// Exposes Era's iOS Simulator operations as MCP tools via stdio JSON-RPC.
// Launch with: `era mcp`

pub mod handlers;
pub mod protocol;
pub mod tools;
pub mod transport;

/// Start the MCP server on stdio
pub fn serve() {
    transport::run_stdio_loop();
}
