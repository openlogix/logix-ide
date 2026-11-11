//! The running engine: an MCP session with the engine as a child process.
//! Calls tools, and passes the engine's progress and live output
//! notifications to the call that asked for them.
//!
//! rmcp handles each notification on its own task, so their order isn't
//! kept and a call's result can arrive before its last notifications. So
//! the engine's stdout goes through `tee` first, which hands progress and
//! output to their calls in the order the engine sent them, before passing
//! each message on to rmcp: by the time a call has its result, it has had
//! everything the engine sent before it.

use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use rmcp::model::{
    CallToolRequest, CallToolRequestParams, ClientRequest, ProgressToken, RequestId,
    RequestMetaObject, ServerResult,
};
use rmcp::service::{PeerRequestOptions, RoleClient, RunningService};
use rmcp::ServiceExt;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio_util::sync::CancellationToken;

/// Progress of one engine operation, as the engine reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    /// The current phase's message, such as "Downloading logix-core".
    /// Only sent when a phase starts.
    pub message: Option<String>,
    /// The current phase's percentage, when the phase has one.
    pub percent: Option<u32>,
}

impl Progress {
    /// The engine's progress uses one increasing scale: 100 per phase, so
    /// the phase's percentage is the progress modulo 100. A phase without a
    /// percentage reports only its start (a multiple of 100).
    fn new(progress: f64, message: Option<String>) -> Self {
        let in_phase = progress.rem_euclid(100.0);
        let percent = if message.is_some() && in_phase == 0.0 {
            None
        } else {
            Some(in_phase.round() as u32)
        };
        Progress { message, percent }
    }
}

/// A line of a tool's live output (`logix/output`, see the engine's tool
/// API).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OutputLine {
    /// The tool that printed it, such as `yosys`.
    pub tool: String,
    pub kind: OutputKind,
    pub level: OutputLevel,
    pub text: String,
    /// The tool prints over this line (a carriage return ended it, as
    /// progress bars do): the console replaces it with the tool's next line.
    #[serde(default)]
    pub redraw: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputKind {
    /// The command line of a tool about to run.
    Command,
    /// A line the tool printed.
    Output,
    /// A line from the engine itself, e.g. how the board will be
    /// programmed.
    Message,
    /// A kind this IDE doesn't know yet (the engine only adds): shown as
    /// output.
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputLevel {
    Error,
    Warning,
    Info,
}

pub type Sink<T> = Arc<dyn Fn(T) + Send + Sync>;
pub type ProgressSink = Sink<Progress>;
pub type OutputSink = Sink<OutputLine>;

/// Routes notifications to the calls that want them, by key (progress
/// token or request id). rmcp sets both and the IDE only learns them once
/// the request is sent (rmcp replaces any progress token the caller sets),
/// so notifications that arrive before the call registers are kept and
/// replayed. Sinks run under the lock, so their order is kept.
struct Routes<K, T> {
    sinks: HashMap<K, Sink<T>>,
    early: HashMap<K, Vec<T>>,
}

impl<K, T> Default for Routes<K, T> {
    fn default() -> Self {
        Routes {
            sinks: HashMap::new(),
            early: HashMap::new(),
        }
    }
}

/// At most this many early notifications are kept per call.
const MAX_EARLY: usize = 1000;

impl<K: Hash + Eq + Clone, T> Routes<K, T> {
    fn deliver(&mut self, key: &K, item: T) {
        match self.sinks.get(key) {
            Some(sink) => sink(item),
            None => {
                let early = self.early.entry(key.clone()).or_default();
                if early.len() < MAX_EARLY {
                    early.push(item);
                }
            }
        }
    }

    fn register(&mut self, key: K, sink: Sink<T>) {
        for item in self.early.remove(&key).unwrap_or_default() {
            sink(item);
        }
        self.sinks.insert(key, sink);
    }

    /// Forgets a finished call, whether or not it wanted notifications (the
    /// engine sends progress for every call, since every call has a token).
    fn finish(&mut self, key: &K) {
        self.sinks.remove(key);
        self.early.remove(key);
    }
}

/// Where the engine's notifications go. One lock for both kinds, so a
/// call's progress and output keep their order.
#[derive(Default)]
struct Listeners {
    progress: Routes<ProgressToken, Progress>,
    output: Routes<RequestId, OutputLine>,
}

/// Hands a progress or `logix/output` notification (one JSON-RPC message)
/// to its call. Other messages are left to rmcp.
fn route(listeners: &Mutex<Listeners>, message: &str) {
    // Most lines are neither; skip parsing those.
    if !message.contains("notifications/progress") && !message.contains("logix/output") {
        return;
    }
    let Ok(msg) = serde_json::from_str::<Value>(message) else {
        return;
    };
    let params = &msg["params"];
    match msg["method"].as_str() {
        Some("notifications/progress") => {
            let (Ok(token), Some(progress)) = (
                ProgressToken::deserialize(&params["progressToken"]),
                params["progress"].as_f64(),
            ) else {
                return;
            };
            let message = params["message"].as_str().map(String::from);
            listeners
                .lock()
                .unwrap()
                .progress
                .deliver(&token, Progress::new(progress, message));
        }
        Some("logix/output") => {
            let (Ok(id), Ok(line)) = (
                RequestId::deserialize(&params["request_id"]),
                OutputLine::deserialize(params),
            ) else {
                return;
            };
            listeners.lock().unwrap().output.deliver(&id, line);
        }
        _ => {}
    }
}

/// Copies the engine's stdout to rmcp line by line (one JSON-RPC message
/// per line), routing notifications on the way.
async fn tee<R, W>(engine_out: R, mut to_rmcp: W, listeners: Arc<Mutex<Listeners>>)
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(engine_out);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                if let Ok(text) = std::str::from_utf8(&buf) {
                    route(&listeners, text);
                }
                if to_rmcp.write_all(&buf).await.is_err() {
                    break;
                }
            }
        }
    }
}

/// A tool call's result: the structured output, or the engine's error
/// object (category, message, details, ...) when the tool failed.
pub type ToolResult = std::result::Result<Value, Value>;

/// What a call wants besides its result.
#[derive(Default)]
pub struct CallOptions {
    /// Gets the engine's progress notifications.
    pub on_progress: Option<ProgressSink>,
    /// Gets the tools' live output (asks the engine for `logix/output`).
    pub on_output: Option<OutputSink>,
    /// Stops the call: the engine stops its tools, and the call returns a
    /// `cancelled` error object.
    pub cancel: Option<CancellationToken>,
}

/// A running engine with an open MCP session. Dropping it stops the engine.
pub struct Engine {
    service: RunningService<RoleClient, ()>,
    listeners: Arc<Mutex<Listeners>>,
    child: std::sync::Mutex<tokio::process::Child>,
}

impl Engine {
    /// Launches the engine at `exe` with its engine folder and opens the MCP
    /// session. The engine's stderr (its log lines) goes to the IDE's
    /// stderr; the engine also writes them to its own log file.
    pub async fn start(exe: &Path, engine_folder: &Path) -> Result<Engine> {
        let mut child = tokio::process::Command::new(exe)
            .arg("--engine-folder")
            .arg(engine_folder)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("can't run {}", exe.display()))?;
        let stdin = child.stdin.take().context("no stdin pipe")?;
        let stdout = child.stdout.take().context("no stdout pipe")?;
        let listeners = Arc::new(Mutex::new(Listeners::default()));
        let (to_rmcp, rmcp_in) = tokio::io::duplex(1 << 16);
        tokio::spawn(tee(stdout, to_rmcp, listeners.clone()));
        let service =
            ().serve((rmcp_in, stdin))
                .await
                .context("the engine didn't start an MCP session")?;
        Ok(Engine {
            service,
            listeners,
            child: std::sync::Mutex::new(child),
        })
    }

    /// Whether the engine's process has exited, such as after a crash: the
    /// IDE then starts it again (`commands::ready_engine`).
    pub fn has_stopped(&self) -> bool {
        self.child
            .lock()
            .map(|mut c| c.try_wait().ok().flatten().is_some())
            .unwrap_or(true)
    }

    /// Stops the engine's process, as a crash would.
    #[cfg(test)]
    pub fn kill_for_test(&self) {
        let _ = self.child.lock().unwrap().start_kill();
    }

    /// Calls a tool with `args` (a JSON object). With `on_progress`, passes
    /// the engine's progress notifications on as they arrive. Fails only
    /// when the call can't be made or answered; a tool that fails returns
    /// its error object.
    pub async fn call(
        &self,
        name: &str,
        args: Value,
        on_progress: Option<ProgressSink>,
    ) -> Result<ToolResult> {
        let options = CallOptions {
            on_progress,
            ..CallOptions::default()
        };
        self.call_with(name, args, options).await
    }

    /// Calls a tool, as `call`, with live output and cancellation.
    pub async fn call_with(
        &self,
        name: &str,
        args: Value,
        options: CallOptions,
    ) -> Result<ToolResult> {
        let mut params = CallToolRequestParams::new(name.to_string());
        if let Value::Object(map) = args {
            params = params.with_arguments(map);
        }
        let request = ClientRequest::CallToolRequest(CallToolRequest::new(params));
        let mut request_options = PeerRequestOptions::no_options();
        if options.on_output.is_some() {
            let mut meta = RequestMetaObject::new();
            meta.0 .0.insert("logix/output".into(), Value::Bool(true));
            request_options.meta = Some(meta);
        }
        let mut handle = self
            .service
            .send_cancellable_request(request, request_options)
            .await
            .map_err(|e| anyhow!("can't send {name} to the engine: {e}"))?;
        let (token, id) = (handle.progress_token.clone(), handle.id.clone());
        {
            let mut listeners = self.listeners.lock().unwrap();
            if let Some(sink) = options.on_progress {
                listeners.progress.register(token.clone(), sink);
            }
            if let Some(sink) = options.on_output {
                listeners.output.register(id.clone(), sink);
            }
        }
        let cancel = options.cancel.unwrap_or_default();
        let response = tokio::select! {
            response = &mut handle.rx => Some(response),
            _ = cancel.cancelled() => None,
        };
        {
            let mut listeners = self.listeners.lock().unwrap();
            listeners.progress.finish(&token);
            listeners.output.finish(&id);
        }
        let Some(response) = response else {
            // The engine stops the call's tools and, as MCP requires, sends
            // no answer.
            let _ = handle.cancel(Some("Stopped by the user".into())).await;
            return Ok(Err(json!({"category": "cancelled", "message": "Stopped."})));
        };
        let response = response
            .map_err(|_| anyhow!("Logix Engine stopped while running {name}; it starts again with the next command"))?
            .map_err(|e| anyhow!("the engine didn't answer {name}: {e}"))?;
        let result = match response {
            ServerResult::CallToolResult(result) => result,
            other => {
                return Err(anyhow!(
                    "unexpected answer from the engine to {name}: {other:?}"
                ))
            }
        };
        let content = result.structured_content.unwrap_or(Value::Null);
        Ok(if result.is_error == Some(true) {
            Err(content)
        } else {
            Ok(content)
        })
    }
}

/// An engine error object as an error with its message, then the details
/// (such as tool output), for the console.
pub fn tool_error(name: &str, error: &Value) -> anyhow::Error {
    let message = error["message"].as_str().unwrap_or("unknown error");
    match error["details"].as_str().filter(|d| !d.trim().is_empty()) {
        Some(details) => anyhow!("{name} failed: {message}\n{details}"),
        None => anyhow!("{name} failed: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::NumberOrString;

    fn token(n: i64) -> ProgressToken {
        ProgressToken(NumberOrString::Number(n))
    }

    fn step(percent: u32) -> Progress {
        Progress {
            message: None,
            percent: Some(percent),
        }
    }

    #[test]
    fn routes_progress_including_early_notifications() {
        let got = Arc::new(Mutex::new(Vec::new()));
        let sink: ProgressSink = {
            let got = got.clone();
            Arc::new(move |p: Progress| got.lock().unwrap().push(p.percent.unwrap()))
        };
        let mut routes: Routes<ProgressToken, Progress> = Routes::default();
        // Arrives before the call registers: kept, then replayed in order.
        routes.deliver(&token(0), step(1));
        routes.deliver(&token(0), step(2));
        // Another call's progress (one that doesn't want it).
        routes.deliver(&token(1), step(50));
        routes.register(token(0), sink);
        routes.deliver(&token(0), step(3));
        assert_eq!(*got.lock().unwrap(), vec![1, 2, 3]);

        routes.finish(&token(0));
        routes.finish(&token(1));
        routes.deliver(&token(0), step(4));
        assert_eq!(*got.lock().unwrap(), vec![1, 2, 3]);
        // Only the late one is kept, until its call finishes.
        assert_eq!(routes.early.len(), 1);
        assert!(routes.sinks.is_empty());
    }

    #[test]
    fn reads_phase_percentages() {
        // A phase starts: its message, no percentage yet.
        assert_eq!(
            Progress::new(100.0, Some("Downloading openxc7".into())),
            Progress {
                message: Some("Downloading openxc7".into()),
                percent: None
            }
        );
        // Within the second phase.
        assert_eq!(
            Progress::new(142.4, None),
            Progress {
                message: None,
                percent: Some(42)
            }
        );
        assert_eq!(Progress::new(0.0, None).percent, Some(0));
    }

    /// The engine's messages, as it sends them (one JSON-RPC message per
    /// line).
    const PROGRESS: &str = r#"{"jsonrpc":"2.0","method":"notifications/progress","params":{"message":"Placing and routing","progress":100.0,"progressToken":3,"total":300.0}}"#;
    const OUTPUT: &str = r#"{"jsonrpc":"2.0","method":"logix/output","params":{"kind":"output","level":"error","request_id":3,"text":"main.v:25: ERROR: syntax error","tool":"yosys"}}"#;
    const RESULT: &str = r#"{"jsonrpc":"2.0","id":3,"result":{"content":[]}}"#;

    /// Listeners that record what they get, in order, as text.
    fn recording() -> (Arc<Mutex<Listeners>>, Arc<Mutex<Vec<String>>>) {
        let got = Arc::new(Mutex::new(Vec::new()));
        let listeners = Arc::new(Mutex::new(Listeners::default()));
        let (a, b) = (got.clone(), got.clone());
        let mut l = listeners.lock().unwrap();
        l.progress.register(
            token(3),
            Arc::new(move |p: Progress| a.lock().unwrap().push(format!("{p:?}"))),
        );
        l.output.register(
            NumberOrString::Number(3),
            Arc::new(move |o: OutputLine| b.lock().unwrap().push(format!("{o:?}"))),
        );
        drop(l);
        (listeners, got)
    }

    #[test]
    fn reads_the_engines_messages_and_unknown_kinds() {
        let line = |kind: &str| -> OutputLine {
            serde_json::from_value(serde_json::json!({
                "kind": kind, "level": "info", "tool": "logix-engine",
                "text": "Programming the board's flash through its USB port."
            }))
            .unwrap()
        };
        assert_eq!(line("message").kind, OutputKind::Message);
        // A kind added to the engine later still shows.
        assert_eq!(line("hint").kind, OutputKind::Other);
    }

    #[test]
    fn routes_notifications_in_order() {
        let (listeners, got) = recording();
        route(&listeners, OUTPUT);
        route(&listeners, PROGRESS);
        route(&listeners, RESULT);
        route(&listeners, "not json logix/output");
        let got = got.lock().unwrap();
        assert_eq!(got.len(), 2);
        assert!(got[0].starts_with("OutputLine { tool: \"yosys\", kind: Output, level: Error"));
        assert_eq!(
            got[1],
            "Progress { message: Some(\"Placing and routing\"), percent: None }"
        );
    }

    /// Everything the engine sent before a result is routed before rmcp
    /// gets the result, and rmcp gets every message unchanged.
    #[tokio::test]
    async fn tee_routes_before_passing_on() {
        use tokio::io::AsyncReadExt;
        let (listeners, got) = recording();
        let engine_out = format!("{OUTPUT}\n{PROGRESS}\n{RESULT}\n");
        let (to_rmcp, mut rmcp_in) = tokio::io::duplex(1 << 16);
        tee(engine_out.as_bytes(), to_rmcp, listeners).await;
        assert_eq!(got.lock().unwrap().len(), 2);
        let mut passed = String::new();
        rmcp_in.read_to_string(&mut passed).await.unwrap();
        assert_eq!(passed, engine_out);
    }

    #[test]
    fn tool_errors_keep_the_details() {
        let err = tool_error(
            "download_packages",
            &serde_json::json!({"category": "setup", "message": "Download failed", "details": "HTTP 404"}),
        );
        assert_eq!(
            err.to_string(),
            "download_packages failed: Download failed\nHTTP 404"
        );
        let err = tool_error("x", &serde_json::json!({"message": "Oops", "details": ""}));
        assert_eq!(err.to_string(), "x failed: Oops");
    }
}
