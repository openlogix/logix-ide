//! Running Apio as a child process and streaming its output line by line.

use std::process::{ExitStatus, Stdio};
use std::sync::LazyLock;

use anyhow::{Context, Result};
use regex::Regex;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

tokio::task_local! {
    /// Set (with `quietly`) for frequent background runs that would flood
    /// the log, e.g. the connected-board polling.
    static QUIET: bool;
}

/// Runs `work` without logging the Apio runs it makes.
pub async fn quietly<T>(work: impl std::future::Future<Output = T>) -> T {
    QUIET.scope(true, work).await
}

fn quiet() -> bool {
    QUIET.try_with(|q| *q).unwrap_or(false)
}

/// Runs `cmd`, calling `on_line` for every line it writes to stdout or
/// stderr, and returns its exit status. If `cancel` is triggered, the
/// process and everything it started are stopped.
pub async fn run_streaming<F>(
    mut cmd: Command,
    on_line: F,
    cancel: &CancellationToken,
) -> Result<ExitStatus>
where
    F: Fn(String) + Send + Sync + Clone + 'static,
{
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    crate::platform::isolate_process_tree(&mut cmd);
    let std_cmd = cmd.as_std();
    let line = std::iter::once(std_cmd.get_program())
        .chain(std_cmd.get_args())
        .map(|a| a.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    if !quiet() {
        crate::log!("Run: {line}");
    }
    let mut child = cmd.spawn().context("can't start Apio")?;
    let stdout = child.stdout.take().context("no stdout pipe")?;
    let stderr = child.stderr.take().context("no stderr pipe")?;
    let out_task = tokio::spawn(pump_lines(stdout, on_line.clone()));
    let err_task = tokio::spawn(pump_lines(stderr, on_line));
    let status = tokio::select! {
        status = child.wait() => status,
        _ = cancel.cancelled() => {
            crate::platform::stop_process_tree(&mut child);
            child.wait().await
        }
    }
    .context("waiting for Apio failed")?;
    let stopped = if cancel.is_cancelled() {
        " (stopped)"
    } else {
        ""
    };
    if !quiet() || !status.success() {
        crate::log!("Exit: {status}{stopped}");
    }
    let _ = out_task.await;
    let _ = err_task.await;
    Ok(status)
}

async fn pump_lines<R, F>(reader: R, on_line: F)
where
    R: AsyncRead + Unpin,
    F: Fn(String),
{
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => on_line(clean_line(&buf)),
        }
    }
}

/// Terminal escape sequences: colors and styles (`ESC[...m`), cursor and
/// erase commands, and title/link sequences (`ESC]...BEL`).
static ANSI: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07\x1b]*(?:\x07|\x1b\\)|[@-Z\\-_])").unwrap()
});

/// Removes terminal escape sequences (e.g. colors), for parsing output.
pub fn strip_ansi(text: &str) -> std::borrow::Cow<'_, str> {
    ANSI.replace_all(text, "")
}

/// Decodes a raw output line. For lines redrawn with carriage returns
/// (progress bars), keeps only the last version, as a terminal would show.
fn clean_line(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    let text = text.trim_end_matches(['\n', '\r']);
    text.rsplit('\r').next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[cfg(unix)]
    #[tokio::test]
    async fn cancel_stops_the_process_and_its_children() {
        let cancel = CancellationToken::new();
        let trigger = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            trigger.cancel();
        });
        let mut cmd = Command::new("/bin/sh");
        // A child process that would run for a minute.
        cmd.args(["-c", "sleep 60 & wait"]);
        let started = std::time::Instant::now();
        let status = run_streaming(cmd, |_| {}, &cancel).await.unwrap();
        assert!(!status.success());
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
    }

    #[test]
    fn strips_ansi_sequences() {
        assert_eq!(strip_ansi("\x1b[32mSUCCESS\x1b[0m"), "SUCCESS");
        assert_eq!(strip_ansi("a\x1b[1;38;5;208mb\x1b[0m c"), "ab c");
        assert_eq!(strip_ansi("\x1b[2K\x1b[?25lline"), "line");
        assert_eq!(strip_ansi("\x1b]8;;http://x\x07link\x1b]8;;\x07"), "link");
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[test]
    fn clean_line_keeps_last_redraw() {
        assert_eq!(clean_line(b"hello\n"), "hello");
        assert_eq!(clean_line(b"hello\r\n"), "hello");
        assert_eq!(clean_line(b"10%\r50%\r100%\n"), "100%");
        assert_eq!(clean_line(b"bad \xff byte\n"), "bad \u{fffd} byte");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn streams_stdout_and_stderr() {
        let lines = Arc::new(Mutex::new(Vec::new()));
        let sink = lines.clone();
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "echo out; echo err 1>&2; exit 3"]);
        let status = run_streaming(
            cmd,
            move |l| sink.lock().unwrap().push(l),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(status.code(), Some(3));
        let mut got = lines.lock().unwrap().clone();
        got.sort();
        assert_eq!(got, vec!["err", "out"]);
    }
}
