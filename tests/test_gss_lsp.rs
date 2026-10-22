use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn send_rpc(stdin: &mut dyn Write, msg: &Value) {
    let body = serde_json::to_string(msg).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    stdin.write_all(header.as_bytes()).unwrap();
    stdin.write_all(body.as_bytes()).unwrap();
    stdin.flush().unwrap();
}

fn read_rpc(reader: &mut dyn BufRead) -> Value {
    let mut line = String::new();
    let mut content_length = 0;
    loop {
        line.clear();
        reader.read_line(&mut line).unwrap();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(stripped) = trimmed.strip_prefix("Content-Length:") {
            content_length = stripped.trim().parse::<usize>().unwrap();
        }
    }

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[test]
fn test_lsp_initialize_and_format() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_gss-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn gss-lsp");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // 1. Initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "capabilities": {}
        }
    });
    send_rpc(&mut stdin, &init_req);
    let init_resp = read_rpc(&mut stdout);

    assert_eq!(init_resp["id"], 1);
    assert_eq!(
        init_resp["result"]["capabilities"]["documentFormattingProvider"],
        true
    );

    // 2. Initialized notification
    let initialized_not = json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    send_rpc(&mut stdin, &initialized_not);

    // 3. didOpen with unformatted document
    let did_open = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.gss",
                "languageId": "gss",
                "version": 1,
                "text": "b=2,a=1"
            }
        }
    });
    send_rpc(&mut stdin, &did_open);

    // Expect publishDiagnostics with empty diagnostics (valid syntax)
    let diag_not = read_rpc(&mut stdout);
    assert_eq!(diag_not["method"], "textDocument/publishDiagnostics");
    assert_eq!(diag_not["params"]["diagnostics"], json!([]));

    // 4. Request formatting
    let format_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/formatting",
        "params": {
            "textDocument": {
                "uri": "file:///test.gss"
            },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }
    });
    send_rpc(&mut stdin, &format_req);
    let format_resp = read_rpc(&mut stdout);
    assert_eq!(format_resp["id"], 2);
    let edits = format_resp["result"].as_array().expect("result array");
    assert!(!edits.is_empty());
    let new_text = edits[0]["newText"].as_str().unwrap();
    assert_eq!(new_text, "a = 1,\nb = 2,\n");

    // 5. Shutdown and exit
    let shutdown_req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "shutdown",
        "params": null
    });
    send_rpc(&mut stdin, &shutdown_req);
    let shutdown_resp = read_rpc(&mut stdout);
    assert_eq!(shutdown_resp["id"], 3);

    let exit_not = json!({
        "jsonrpc": "2.0",
        "method": "exit",
        "params": null
    });
    send_rpc(&mut stdin, &exit_not);

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn test_lsp_diagnostics_on_syntax_error() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_gss-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn gss-lsp");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Initialize
    send_rpc(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "capabilities": {} }
        }),
    );
    let _ = read_rpc(&mut stdout);

    // Initialized notification
    send_rpc(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    // didOpen with duplicate key error
    send_rpc(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///error.gss",
                    "languageId": "gss",
                    "version": 1,
                    "text": "key = 1,\nkey = 2,"
                }
            }
        }),
    );

    let diag_not = read_rpc(&mut stdout);
    assert_eq!(diag_not["method"], "textDocument/publishDiagnostics");
    let diags = diag_not["params"]["diagnostics"].as_array().unwrap();
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0]["message"]
            .as_str()
            .unwrap()
            .contains("Redefinition of key key")
    );

    // Cleanup
    send_rpc(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "shutdown",
            "params": null
        }),
    );
    let _ = read_rpc(&mut stdout);

    send_rpc(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}
