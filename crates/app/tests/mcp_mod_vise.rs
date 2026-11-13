//! The MCP mod-vise tool (#820), driven in-process over the server's
//! JSON-RPC handler: a pair placed on a plate's grid and fitted to the
//! stock, its holes, slide and opening reported, one undo for both
//! blocks, and every bad input a named tool error.
//!
//! Gate map (M4.4): covers app::mcp::modvise_tools.

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

#[test]
fn a_mod_vise_pair_is_placed_fitted_and_undone_over_mcp() {
    let mut c = Client::new();
    let listed = c.request("tools/list", json!({}))["result"]["tools"].as_array().cloned().unwrap_or_default();
    assert!(listed.iter().any(|t| t["name"] == json!("add_mod_vise_pair")), "listed");

    c.ok("set_stock", json!({ "box": { "dx": 60.0, "dy": 60.0, "dz": 20.0 } }));
    // A 12 in plate laid so C3's fixed block face sits on the stock's -Y
    // side: C3 is (2.5, 2.5) in from the corner; the 1 in preset's face is
    // 1.75 in in front of its anchor and its body centred 1 in across.
    let x = -25.4 - 2.5 * 25.4;
    let y = -30.0 - 1.75 * 25.4 - 2.5 * 25.4;
    let plate = c.ok("add_fixture_plate", json!({ "preset": "12 x 12 in", "position": [x, y, 0.0] }))["fixture"]
        .as_str()
        .expect("name")
        .to_owned();

    let why = c.refused("add_mod_vise_pair", json!({ "plate": plate, "hole": "C3", "size": "3 in grid" }));
    assert!(why.contains("no mod vise preset") && why.contains("1 in grid 3/8-16"), "{why}");
    let why = c.refused("add_mod_vise_pair", json!({ "plate": plate, "hole": "C3", "size": "1 in grid 3/8-16", "adjustable": { "slot_travel": 10.0 } }));
    assert!(why.contains("out of reach"), "{why}");
    let why = c.refused("add_mod_vise_pair", json!({ "plate": plate, "hole": "C3", "size": "1 in grid 3/8-16", "fixed": { "body_length": -1.0 } }));
    assert!(why.contains("body length"), "{why}");
    let why = c.refused("add_mod_vise_pair", json!({ "plate": plate, "hole": "C3", "size": "1 in grid 3/8-16", "fixed": { "wings": 1.0 } }));
    assert!(why.contains("unknown jaw block figure"), "{why}");
    let why = c.refused("add_mod_vise_pair", json!({ "plate": plate, "hole": "C3", "size": "1 in grid 3/8-16", "yaw": 30.0 }));
    assert!(why.contains("0, 90, 180 or 270"), "{why}");

    let out = c.ok("add_mod_vise_pair", json!({ "plate": plate, "hole": "C3", "size": "1 in grid 3/8-16", "insert": "soft" }));
    assert_eq!(out["fixed"]["at"], json!("C3"), "{out}");
    assert_eq!(out["adjustable"]["at"], json!("I5"), "{out}");
    assert_eq!(out["fixed"]["block"]["insert"], json!("soft"), "{out}");
    let slide = out["slide"].as_f64().unwrap_or_default();
    assert!((slide - (5.0 * 25.4 - 104.45)).abs() < 1e-6, "{out}");
    assert!((out["opening"].as_f64().unwrap_or_default() - 60.0).abs() < 1e-6, "{out}");
    let grid = c.ok("get_plate_grid", json!({ "name": plate }));
    assert_eq!(grid["carries"].as_array().map(Vec::len), Some(2), "{grid}");

    c.ok("undo", json!({}));
    let grid = c.ok("get_plate_grid", json!({ "name": plate }));
    assert_eq!(grid["carries"].as_array().map(Vec::len).unwrap_or_default(), 0, "one undo takes both: {grid}");
}
