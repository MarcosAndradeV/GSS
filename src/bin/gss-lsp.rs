use std::collections::HashMap;
use std::error::Error;

use lsp_server::{Connection, Message, Notification, Response};
use serde_json::json;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = json!({
        "textDocumentSync": 1, // Full document sync
        "documentFormattingProvider": true,
    });

    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params)?;
    io_threads.join()?;

    Ok(())
}

fn main_loop(
    connection: Connection,
    _params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let mut documents: HashMap<String, String> = HashMap::new();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                match req.method.as_str() {
                    "textDocument/formatting" => {
                        let params = req.params;
                        let uri = params
                            .get("textDocument")
                            .and_then(|td| td.get("uri"))
                            .and_then(|u| u.as_str())
                            .unwrap_or_default();

                        let text = documents.get(uri).cloned().unwrap_or_default();
                        let result = match gss::format_str(&text) {
                            Ok(formatted) => {
                                if formatted == text {
                                    json!([])
                                } else {
                                    let line_count = text.lines().count().max(1);
                                    json!([
                                        {
                                            "range": {
                                                "start": { "line": 0, "character": 0 },
                                                "end": { "line": line_count + 1, "character": 0 }
                                            },
                                            "newText": formatted
                                        }
                                    ])
                                }
                            }
                            Err(_) => json!([]),
                        };

                        let resp = Response::new_ok(req.id, result);
                        connection.sender.send(Message::Response(resp))?;
                    }
                    _ => {}
                }
            }
            Message::Response(_) => {}
            Message::Notification(not) => match not.method.as_str() {
                "textDocument/didOpen" => {
                    if let Some(td) = not.params.get("textDocument") {
                        if let (Some(uri), Some(text)) = (
                            td.get("uri").and_then(|u| u.as_str()),
                            td.get("text").and_then(|t| t.as_str()),
                        ) {
                            documents.insert(uri.to_string(), text.to_string());
                            publish_diagnostics(&connection, uri, text)?;
                        }
                    }
                }
                "textDocument/didChange" => {
                    if let (Some(uri), Some(changes)) = (
                        not.params
                            .get("textDocument")
                            .and_then(|td| td.get("uri"))
                            .and_then(|u| u.as_str()),
                        not.params.get("contentChanges").and_then(|c| c.as_array()),
                    ) {
                        if let Some(last_change) = changes
                            .last()
                            .and_then(|c| c.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            documents.insert(uri.to_string(), last_change.to_string());
                            publish_diagnostics(&connection, uri, last_change)?;
                        }
                    }
                }
                "textDocument/didClose" => {
                    if let Some(uri) = not
                        .params
                        .get("textDocument")
                        .and_then(|td| td.get("uri"))
                        .and_then(|u| u.as_str())
                    {
                        documents.remove(uri);
                        let not = Notification::new(
                            "textDocument/publishDiagnostics".to_string(),
                            json!({
                                "uri": uri,
                                "diagnostics": []
                            }),
                        );
                        connection.sender.send(Message::Notification(not))?;
                    }
                }
                _ => {}
            },
        }
    }
    Ok(())
}

fn publish_diagnostics(
    connection: &Connection,
    uri: &str,
    text: &str,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let diagnostics = match gss::parse_str(text) {
        Ok(_) => json!([]),
        Err(err) => {
            let err_str = err.to_string();
            let diag = parse_error_to_diagnostic(&err_str, text);
            json!([diag])
        }
    };

    let not = Notification::new(
        "textDocument/publishDiagnostics".to_string(),
        json!({
            "uri": uri,
            "diagnostics": diagnostics
        }),
    );
    connection.sender.send(Message::Notification(not))?;
    Ok(())
}

fn parse_error_to_diagnostic(err: &str, text: &str) -> serde_json::Value {
    let mut line = 0u32;
    let mut col = 0u32;
    let mut message = err;

    let parts: Vec<&str> = err.splitn(4, ':').collect();
    if parts.len() >= 4 {
        if let (Ok(l), Ok(c)) = (parts[1].trim().parse::<u32>(), parts[2].trim().parse::<u32>()) {
            line = l.saturating_sub(1);
            col = c.saturating_sub(1);
            message = parts[3].trim();
        }
    }

    let line_count = text.lines().count().max(1) as u32;
    let safe_line = line.min(line_count.saturating_sub(1));
    let line_str = text.lines().nth(safe_line as usize).unwrap_or("");
    let safe_col = col.min(line_str.len() as u32);
    let end_col = (safe_col + 1).max(line_str.len() as u32);

    json!({
        "range": {
            "start": { "line": safe_line, "character": safe_col },
            "end": { "line": safe_line, "character": end_col }
        },
        "severity": 1,
        "source": "gss",
        "message": message
    })
}
