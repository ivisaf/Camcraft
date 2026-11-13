//! The MCP built-in vise tools (#788), driven in-process over the
//! server's JSON-RPC handler: a 6 in vise fitted to the stock, its jaws
//! re-opened and resized, every step one undo, every bad input a named
//! tool error.
//!
//! Gate map (M4.4): covers app::mcp::vise_tools.

use camcraft_app::mcp::Server;

use serde_json::{Value, json};

struct Client {
    server: Server,
    next: i64,
}

impl Client {
    fn new() -> Self {
        let mut c = Self { server: Server::new(), next: 1 };
        c.request("initialize", json!({ "protocolVersion": "2025-06-18" }));
        c
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        let msg = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.server.handle(&msg).expect("a request gets a response")
    }

    fn call(&mut self, name: &str, args: Value) -> (bool, Value) {
        let reply = self.request("tools/call", json!({ "name": name, "arguments": args }));
        let result = &reply["result"];
        assert!(result.is_object(), "tools/call answers with a result: {reply}");
        let is_err = result["isError"] == json!(true);
        let body = if is_err { result["content"][0]["text"].clone() } else { result["structuredContent"].clone() };
        (is_err, body)
    }

    fn ok(&mut self, name: &str, args: Value) -> Value {
        let (err, body) = self.call(name, args);
        assert!(!err, "{name} refused: {body}");
        body
    }

    fn refused(&mut self, name: &str, args: Value) -> String {
        let (err, body) = self.call(name, args);
        assert!(err, "{name} should have refused, got {body}");
        body.as_str().unwrap_or_default().to_owned()
    }
}

fn fixtures(c: &mut Client) -> Vec<String> {
    let f = c.ok("get_fixture_findings", json!({}));
    f.to_string().match_indices("Vise 6 in").map(|(_, s)| s.to_owned()).collect()
}

#[test]
fn a_vise_is_fitted_reopened_and_undone_over_mcp() {
    let mut c = Client::new();
    let names: Vec<String> = c.request("tools/list", json!({}))["result"]["tools"]
        .as_array()
        .map(|t| t.iter().filter_map(|t| t["name"].as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    for want in ["add_builtin_vise", "set_vise"] {
        assert!(names.iter().any(|n| n == want), "missing tool {want}");
    }

    let why = c.refused("add_builtin_vise", json!({ "size": "7 in" }));
    assert!(why.contains("no built-in vise '7 in'") && why.contains("6 in"), "{why}");
    let why = c.refused("add_builtin_vise", json!({ "size": "6 in", "fit_to_stock": true }));
    assert!(why.contains("no stock"), "{why}");

    c.ok("set_stock", json!({ "box": { "dx": 60.0, "dy": 50.0, "dz": 20.0 } }));
    let placed = c.ok("add_builtin_vise", json!({ "size": "6 in", "fit_to_stock": true }));
    assert_eq!(placed["fixture"], json!("Vise 6 in"), "{placed}");
    assert_eq!(placed["vise"]["opening"], json!(50.0), "{placed}");
    assert_eq!(placed["origin"], json!([0.0, 25.0, 0.0]), "{placed}");
    assert!(!fixtures(&mut c).is_empty(), "the verifier sees it");

    let why = c.refused("set_vise", json!({ "name": "Vise 6 in", "opening": 500.0 }));
    assert!(why.contains("over the max opening"), "{why}");
    let why = c.refused("set_vise", json!({ "name": "Vise 6 in", "dimensions": { "jaw_height": 0.0 } }));
    assert!(why.contains("jaw height"), "{why}");
    let why = c.refused("set_vise", json!({ "name": "Vise 6 in", "dimensions": { "throat": 3.0 } }));
    assert!(why.contains("unknown vise dimension"), "{why}");

    let set = c.ok("set_vise", json!({ "name": "Vise 6 in", "opening": 120.0, "dimensions": { "jaw_height": 50.0 } }));
    assert_eq!(set["vise"]["opening"], json!(120.0), "{set}");
    assert_eq!(set["vise"]["jaw_height"], json!(50.0), "{set}");
    let set = c.ok("set_vise", json!({ "name": "Vise 6 in", "from_stock": true }));
    assert_eq!(set["vise"]["opening"], json!(50.0), "back against the stock: {set}");

    c.ok("undo", json!({}));
    c.ok("undo", json!({}));
    c.ok("undo", json!({}));
    assert!(fixtures(&mut c).is_empty(), "three undos: the two edits, then the vise");
}
