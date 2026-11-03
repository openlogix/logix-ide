//! The running engine: an MCP session with the engine as a child process.
//! Calls tools and routes the engine's progress notifications to the call
//! that asked for them.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use rmcp::model::{
    CallToolRequest, CallToolRequestParams, ClientRequest, ProgressNotificationParam,
    ProgressToken, ServerResult,
};
use rmcp::service::{NotificationContext, PeerRequestOptions, RoleClient, RunningService};
use rmcp::transport::TokioChildProcess;
use rmcp::{ClientHandler, ServiceExt};
use serde_json::Value;

/// Progress of one engine operation, as the engine reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    /// The current phase's message, such as "Installing oss-cad-suite".
    /// Only sent when a phase starts.
    pub message: Option<String>,
    /// The current phase's percentage, when the phase has one.
    pub percent: Option<u32>,
}

impl Progress {
    /// The engine's progress uses one increasing scale: 100 per phase, so
    /// the phase's percentage is the progress modulo 100. A phase without a
    /// percentage reports only its start (a multiple of 100).
    fn from_notification(p: &ProgressNotificationParam) -> Self {
        let in_phase = p.progress.rem_euclid(100.0);
        let percent = if p.message.is_some() && in_phase == 0.0 {
            None
        } else {
            Some(in_phase.round() as u32)
        };
        Progress {
            message: p.message.clone(),
            percent,
        }
    }
}

pub type ProgressSink = Arc<dyn Fn(Progress) + Send + Sync>;

/// Routes progress notifications to the calls that want them, by progress
/// token. rmcp gives every request its own token (any token the caller sets
/// is replaced), and the IDE only learns it once the request is sent, so
/// notifications that arrive before the call registers are kept and
/// replayed. Sinks run under the lock, so their order is kept.
#[derive(Default)]
struct Routes {
    sinks: HashMap<ProgressToken, ProgressSink>,
    early: HashMap<ProgressToken, Vec<Progress>>,
}

/// At most this many early notifications are kept per call.
const MAX_EARLY: usize = 1000;

impl Routes {
    fn deliver(&mut self, token: &ProgressToken, progress: Progress) {
        match self.sinks.get(token) {
            Some(sink) => sink(progress),
            None => {
                let early = self.early.entry(token.clone()).or_default();
                if early.len() < MAX_EARLY {
                    early.push(progress);
                }
            }
        }
    }

    fn register(&mut self, token: ProgressToken, sink: ProgressSink) {
        for progress in self.early.remove(&token).unwrap_or_default() {
            sink(progress);
        }
        self.sinks.insert(token, sink);
    }

    /// Forgets a finished call, whether or not it wanted progress (the
    /// engine sends progress for every call, since every call has a token).
    fn finish(&mut self, token: &ProgressToken) {
        self.sinks.remove(token);
        self.early.remove(token);
    }
}

/// Receives the engine's notifications.
#[derive(Clone, Default)]
struct Handler {
    routes: Arc<Mutex<Routes>>,
}

impl ClientHandler for Handler {
    async fn on_progress(
        &self,
        params: ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        self.routes
            .lock()
            .unwrap()
            .deliver(&params.progress_token, Progress::from_notification(&params));
    }
}

/// A tool call's result: the structured output, or the engine's error
/// object (category, message, details, ...) when the tool failed.
pub type ToolResult = std::result::Result<Value, Value>;

/// A running engine with an open MCP session. Dropping it stops the engine.
pub struct Engine {
    service: RunningService<RoleClient, Handler>,
    handler: Handler,
}

impl Engine {
    /// Launches the engine at `exe` with its engine folder and opens the MCP
    /// session. The engine's stderr (its log lines) goes to the IDE's
    /// stderr; the engine also writes them to its own log file.
    pub async fn start(exe: &Path, engine_folder: &Path) -> Result<Engine> {
        let mut cmd = tokio::process::Command::new(exe);
        cmd.arg("--engine-folder").arg(engine_folder);
        let transport =
            TokioChildProcess::new(cmd).with_context(|| format!("can't run {}", exe.display()))?;
        let handler = Handler::default();
        let service = handler
            .clone()
            .serve(transport)
            .await
            .context("the engine didn't start an MCP session")?;
        Ok(Engine { service, handler })
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
        let mut params = CallToolRequestParams::new(name.to_string());
        if let Value::Object(map) = args {
            params = params.with_arguments(map);
        }
        let request = ClientRequest::CallToolRequest(CallToolRequest::new(params));
        let handle = self
            .service
            .send_cancellable_request(request, PeerRequestOptions::no_options())
            .await
            .map_err(|e| anyhow!("can't send {name} to the engine: {e}"))?;
        let token = handle.progress_token.clone();
        if let Some(sink) = on_progress {
            self.handler
                .routes
                .lock()
                .unwrap()
                .register(token.clone(), sink);
        }
        let response = handle.await_response().await;
        self.handler.routes.lock().unwrap().finish(&token);
        let result = match response.map_err(|e| anyhow!("the engine didn't answer {name}: {e}"))? {
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
        let mut routes = Routes::default();
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

    fn notification(progress: f64, message: Option<&str>) -> ProgressNotificationParam {
        let mut p =
            ProgressNotificationParam::new(ProgressToken(NumberOrString::Number(1)), progress);
        p.total = Some(300.0);
        p.message = message.map(String::from);
        p
    }

    #[test]
    fn reads_phase_percentages() {
        // A phase starts: its message, no percentage yet.
        assert_eq!(
            Progress::from_notification(&notification(100.0, Some("Installing openxc7"))),
            Progress {
                message: Some("Installing openxc7".into()),
                percent: None
            }
        );
        // Within the second phase.
        assert_eq!(
            Progress::from_notification(&notification(142.4, None)),
            Progress {
                message: None,
                percent: Some(42)
            }
        );
        assert_eq!(
            Progress::from_notification(&notification(0.0, None)).percent,
            Some(0)
        );
    }

    #[test]
    fn tool_errors_keep_the_details() {
        let err = tool_error(
            "install_packages",
            &serde_json::json!({"category": "setup", "message": "Download failed", "details": "HTTP 404"}),
        );
        assert_eq!(
            err.to_string(),
            "install_packages failed: Download failed\nHTTP 404"
        );
        let err = tool_error("x", &serde_json::json!({"message": "Oops", "details": ""}));
        assert_eq!(err.to_string(), "x failed: Oops");
    }
}
