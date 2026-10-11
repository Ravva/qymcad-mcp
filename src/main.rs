//! `qymcad-mcp` stdio bridge: JSON-RPC 2.0 over stdin/stdout for AI agents.
//!
//! One line holds one request: `{"jsonrpc":"2.0","id":1,"method":"tools/call",
//! "params":{"name":"apply_recipe","arguments":{"ops":[...]}}}`. Answers carry `result`
//! or `error`. The document lives for the whole process; `new_project` starts over.

use qymcad_mcp::{McpSession, RecipeOp};
use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let mut session = McpSession::new_empty();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        let answer = handle_line(&line, &mut session);
        println!("{answer}");
        let _ = std::io::stdout().flush();
    }
}

/// Answer one input line with one JSON-RPC response line.
fn handle_line(line: &str, session: &mut McpSession) -> String {
    let request: serde_json::Value = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(_) => return parse_error(),
    };
    let id = request.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or(serde_json::Value::Null);
    match method {
        "initialize" => ok(&id, serde_json::json!({"protocolVersion": "2024-11-05", "serverInfo": {"name": "qymcad-mcp", "version": "0.1.0"}})),
        "tools/list" => ok(&id, serde_json::json!({"tools": tool_list()})),
        "tools/call" => call_tool(&id, &params, session),
        _ => err(&id, -32601, "mcp-unknown-method"),
    }
}

/// The eight tools of the first release.
fn tool_list() -> Vec<serde_json::Value> {
    let tool = |name: &str, about: &str| serde_json::json!({"name": name, "description": about, "inputSchema": {"type": "object"}});
    vec![
        tool("new_project", "Start an empty document with one part"),
        tool("open_project", "Open a .qcad file with a full rebuild"),
        tool("save_project", "Save the document as .qcad"),
        tool("apply_recipe", "Build Box/RectSketch/Extrude/Fillet ops in order and rebuild once"),
        tool("inspect", "Count parts, bodies, sketches and nodes with measured body numbers"),
        tool("measure", "Measured numbers of every body without rebuilding"),
        tool("export_stl", "Write display meshes as binary STL"),
        tool("export_step", "Rebuild and write the live B-rep as STEP"),
    ]
}

/// Run one `tools/call` request against the session.
fn call_tool(id: &serde_json::Value, params: &serde_json::Value, session: &mut McpSession) -> String {
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(serde_json::Value::Null);
    match name {
        "new_project" => {
            *session = McpSession::new_empty();
            ok(id, serde_json::json!({"ok": true}))
        }
        "open_project" => {
            let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
            match McpSession::open(path) {
                Ok(next) => {
                    *session = next;
                    ok(id, serde_json::json!({"ok": true}))
                }
                Err(message) => err(id, -32000, &message),
            }
        }
        "save_project" => {
            let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
            match session.save(path) {
                Ok(()) => ok(id, serde_json::json!({"ok": true})),
                Err(message) => err(id, -32000, &message),
            }
        }
        "apply_recipe" => {
            let ops: Vec<RecipeOp> = args.get("ops").and_then(|o| serde_json::from_value(o.clone()).ok()).unwrap_or_default();
            match session.apply(&ops) {
                Ok(result) => ok(id, serde_json::to_value(result).unwrap_or(serde_json::Value::Null)),
                Err(message) => err(id, -32000, &message),
            }
        }
        "inspect" | "measure" => ok(id, serde_json::to_value(session.inspect()).unwrap_or(serde_json::Value::Null)),
        "export_stl" => {
            let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
            match session.export_stl(path) {
                Ok(()) => ok(id, serde_json::json!({"ok": true})),
                Err(message) => err(id, -32000, &message),
            }
        }
        "export_step" => {
            let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
            match session.export_step(path) {
                Ok(()) => ok(id, serde_json::json!({"ok": true})),
                Err(message) => err(id, -32000, &message),
            }
        }
        _ => err(id, -32601, "mcp-unknown-tool"),
    }
}

/// A `result` answer reusing the request id.
fn ok(id: &serde_json::Value, result: serde_json::Value) -> String {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string()
}

/// An `error` answer with a machine-readable message.
fn err(id: &serde_json::Value, code: i32, message: &str) -> String {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}).to_string()
}

/// The input was no JSON at all.
fn parse_error() -> String {
    serde_json::json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "mcp-parse-error"}}).to_string()
}
