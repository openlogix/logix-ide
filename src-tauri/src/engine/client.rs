//! The running engine: an MCP session with the engine as a child process.
//! Calls tools and routes the engine's progress notifications to the call
//! that asked for them.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use rmcp::model::{
    CallToolRequestParams, NumberOrString, ProgressNotificationParam, ProgressToken,
    RequestMetaObject,
};
use rmcp::service::{NotificationContext, RoleClient, RunningService};
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

type ProgressSink = Arc<dyn Fn(Progress) + Send + Sync>;

/// Receives the engine's notifications.
#[derive(Clone, Default)]
struct Handler {
    /// The calls waiting for progress, by progress token.
    progress: Arc<Mutex<HashMap<ProgressToken, ProgressSink>>>,
}

impl ClientHandler for Handler {
    async fn on_progress(
        &self,
        params: ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        let sink = self
            .progress
            .lock()
            .unwrap()
            .get(&params.progress_token)
            .cloned();
        if let Some(sink) = sink {
            sink(Progress::from_notification(&params));
        }
    }
}

/// A tool call's result: the structured output, or the engine's error
/// object (category, message, details, ...) when the tool failed.
pub type ToolResult = std::result::Result<Value, Value>;

/// A running engine with an open MCP session. Dropping it stops the engine.
pub struct Engine {
    service: RunningService<RoleClient, Handler>,
    handler: Handler,
    next_token: AtomicI64,
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
        Ok(Engine {
            service,
            handler,
            next_token: AtomicI64::new(1),
        })
    }

    /// Calls a tool with `args` (a JSON object). With `on_progress`, asks
    /// for progress notifications and passes them on as they arrive.
    /// Fails only when the call can't be made or answered; a tool that
    /// fails returns its error object.
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
        let token = on_progress.map(|sink| {
            let token = ProgressToken(NumberOrString::Number(
                self.next_token.fetch_add(1, Ordering::SeqCst),
            ));
            self.handler
                .progress
                .lock()
                .unwrap()
                .insert(token.clone(), sink);
            let mut meta = RequestMetaObject::new();
            meta.set_progress_token(token.clone());
            params.meta = Some(meta);
            token
        });
        let result = self.service.call_tool(params).await;
        if let Some(token) = token {
            self.handler.progress.lock().unwrap().remove(&token);
        }
        let result = result.map_err(|e| anyhow!("the engine didn't answer {name}: {e}"))?;
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
