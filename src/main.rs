//! gray-session-name — human names for sessions.
//!
//! Port of pi's `session-name` extension: `/name <text>` gives the current
//! session a friendly name, `/name` shows it. Pi stored names in session
//! metadata; gray sidecars keep a map at
//! `~/.gray/session-name/names.json` keyed by `session.id`.
//!
//! A `prompt/context` hook surfaces the name to the model once per session
//! (the host dedups injected context, so it stays quiet after the first
//! turn).

use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

fn manifest() -> Value {
    json!({
        "name": "session-name",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "1.1",
        "tools": [],
        "commands": ["/name"],
        "hooks": ["prompt/context"],
    })
}

fn names_path() -> PathBuf {
    let home = std::env::var_os("GRAY_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".gray")))
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("session-name").join("names.json")
}

fn load_names() -> Value {
    std::fs::read_to_string(names_path())
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| json!({}))
}

fn save_names(names: &Value) -> std::io::Result<()> {
    let path = names_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, names.to_string())?;
    std::fs::rename(&tmp, path)
}

fn session_id(params: &Value) -> &str {
    params
        .pointer("/session/id")
        .and_then(Value::as_str)
        .unwrap_or("")
}

/// `/name …` — `argv` excludes the command name.
fn run_command(argv: &[&str], session: &str) -> String {
    let name = argv.join(" ");
    let name = name.trim();
    if session.is_empty() {
        return "no session id — nothing to name".into();
    }
    let mut names = load_names();
    if name.is_empty() {
        return match names.get(session).and_then(Value::as_str) {
            Some(n) => format!("Session: {n}"),
            None => "No session name set".into(),
        };
    }
    names[session] = json!(name);
    match save_names(&names) {
        Ok(()) => format!("Session named: {name}"),
        Err(e) => format!("couldn't save name: {e}"),
    }
}

/// `prompt/context` — tell the model the session's name once (host dedups).
fn prompt_context(params: &Value) -> Value {
    let session = session_id(params);
    if session.is_empty() {
        return json!({});
    }
    match load_names().get(session).and_then(Value::as_str) {
        Some(name) => json!({ "text": format!("This session is named \"{name}\".") }),
        None => json!({}),
    }
}

/// One request → `Some(reply)`, or `None` for notifications. The bool asks
/// the loop to exit after writing the reply.
fn handle(req: &Value) -> (Option<Value>, bool) {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let Some(id) = id else {
        return (None, method == "plugin/shutdown");
    };
    let result = match method {
        "plugin/manifest" => manifest(),
        "command/run" => {
            let argv: Vec<&str> = params
                .get("argv")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            json!({ "text": run_command(&argv, session_id(&params)) })
        }
        "prompt/context" => prompt_context(&params),
        "plugin/shutdown" => return (Some(json!({ "id": id, "result": {} })), true),
        _ => {
            let error = json!({ "code": -32601, "message": "method not found" });
            return (Some(json!({ "id": id, "error": error })), false);
        }
    };
    (Some(json!({ "id": id, "result": result })), false)
}

fn main() -> std::io::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("manifest") {
        println!("{}", manifest());
        return Ok(());
    }
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(req) = serde_json::from_str::<Value>(&line) else { continue };
        let (reply, exit) = handle(&req);
        if let Some(reply) = reply {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
        if exit {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(method: &str, params: Value) -> Value {
        handle(&json!({ "id": 1, "method": method, "params": params })).0.unwrap()
    }

    /// Serialized: GRAY_HOME is process-global and tests share the process.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    static CTR: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn with_home(body: impl FnOnce(&std::path::Path)) {
        let _g = LOCK.lock().unwrap();
        let n = CTR.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("gray-session-name-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("GRAY_HOME", &dir) };
        body(&dir);
        unsafe { std::env::remove_var("GRAY_HOME") };
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_has_command_and_context_hook() {
        let m = call("plugin/manifest", Value::Null)["result"].clone();
        assert_eq!(m["name"], "session-name");
        assert_eq!(m["commands"], json!(["/name"]));
        assert_eq!(m["hooks"], json!(["prompt/context"]));
    }

    #[test]
    fn name_set_show_and_context() {
        with_home(|home| {
            let sess = json!({"id": "s-42", "cwd": "/tmp/x"});
            // Nothing set yet.
            let r = call("command/run", json!({"name": "/name", "argv": [], "session": sess}));
            assert_eq!(r["result"]["text"], "No session name set");
            assert_eq!(call("prompt/context", json!({"session": sess}))["result"], json!({}));

            // Set it.
            let r = call(
                "command/run",
                json!({"name": "/name", "argv": ["deep", "dive"], "session": sess}),
            );
            assert_eq!(r["result"]["text"], "Session named: deep dive");
            let r = call("command/run", json!({"name": "/name", "argv": [], "session": sess}));
            assert_eq!(r["result"]["text"], "Session: deep dive");

            // prompt/context surfaces it.
            let r = call("prompt/context", json!({"session": sess}));
            assert_eq!(r["result"]["text"], "This session is named \"deep dive\".");

            // Persisted keyed by session id.
            let names = load_names();
            assert_eq!(names["s-42"], "deep dive");
            let _ = home;

            // A different session sees nothing.
            let other = json!({"id": "s-99", "cwd": "/tmp/x"});
            assert_eq!(call("prompt/context", json!({"session": other}))["result"], json!({}));
        });
    }

    #[test]
    fn empty_session_id_is_safe() {
        with_home(|_| {
            let r = call("command/run", json!({"name": "/name", "argv": ["x"], "session": {}}));
            assert!(r["result"]["text"].as_str().unwrap().contains("no session id"));
            assert_eq!(call("prompt/context", json!({"session": {}}))["result"], json!({}));
        });
    }

    #[test]
    fn shutdown_replies_then_exits() {
        let (reply, exit) = handle(&json!({ "id": 2, "method": "plugin/shutdown" }));
        assert!(reply.is_some() && exit);
    }
}
