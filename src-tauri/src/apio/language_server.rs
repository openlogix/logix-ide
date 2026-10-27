//! Verible's Verilog language server (from Apio's packages), for live
//! errors and other editor help while typing. Logix relays the language
//! server protocol (LSP) between the server and the editor's LSP client:
//! this module only frames messages; the UI speaks the protocol.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};

use super::Apio;

/// Server options: only syntax errors (Verible's style rules would flood
/// beginners; Verify's lint catches design problems), plus hover info.
const SERVER_ARGS: &[&str] = &["--ruleset=none", "--lsp_enable_hover=true"];

/// A running language server. Dropping it stops the server.
pub struct LanguageServer {
    stdin: ChildStdin,
    _child: Child,
}

impl Apio {
    /// Where Apio's packages have the language server (silently asks Apio).
    pub async fn language_server_path(&self) -> Result<PathBuf> {
        let system = self.api_json("get-system", None).await?;
        let path = system
            .pointer("/system/verible-language-server")
            .and_then(|v| v.as_str())
            .context("Apio doesn't report a Verilog language server")?;
        Ok(PathBuf::from(path))
    }
}

impl LanguageServer {
    /// Starts the server at `exe`. Each message from the server (JSON,
    /// without headers) goes to `on_message`; `on_exit` is called once
    /// when the server stops.
    pub fn start<M, E>(exe: &Path, on_message: M, on_exit: E) -> Result<Self>
    where
        M: Fn(String) + Send + 'static,
        E: FnOnce() + Send + 'static,
    {
        let mut child = Command::new(exe)
            .args(SERVER_ARGS)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("can't start {}", exe.display()))?;
        let stdin = child.stdin.take().context("no stdin pipe")?;
        let stdout = child.stdout.take().context("no stdout pipe")?;
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            while let Ok(Some(message)) = read_message(&mut reader).await {
                on_message(message);
            }
            on_exit();
        });
        Ok(LanguageServer {
            stdin,
            _child: child,
        })
    }

    /// Sends one message (JSON, without headers) to the server.
    pub async fn send(&mut self, message: &str) -> Result<()> {
        self.stdin.write_all(&frame(message)).await?;
        self.stdin.flush().await?;
        Ok(())
    }
}

/// An LSP message with its `Content-Length` header.
fn frame(message: &str) -> Vec<u8> {
    let mut out = format!("Content-Length: {}\r\n\r\n", message.len()).into_bytes();
    out.extend_from_slice(message.as_bytes());
    out
}

/// Reads one LSP message. `None` at the end of the stream.
async fn read_message<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<Option<String>> {
    let mut length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break; // End of the headers.
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                length = Some(value.trim().parse().context("bad Content-Length")?);
            }
        }
    }
    let Some(length) = length else {
        bail!("LSP message without Content-Length");
    };
    let mut body = vec![0; length];
    reader.read_exact(&mut body).await?;
    Ok(Some(String::from_utf8_lossy(&body).into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reads_framed_messages() {
        let mut data = frame(r#"{"id":1}"#);
        data.extend(b"Content-Type: application/vscode-jsonrpc\r\n");
        data.extend(frame("{\"x\":\"\u{e9}\"}"));
        let mut reader = BufReader::new(&data[..]);
        assert_eq!(
            read_message(&mut reader).await.unwrap().as_deref(),
            Some(r#"{"id":1}"#)
        );
        assert_eq!(
            read_message(&mut reader).await.unwrap().as_deref(),
            Some("{\"x\":\"\u{e9}\"}")
        );
        assert_eq!(read_message(&mut reader).await.unwrap(), None);
    }

    #[test]
    fn frames_with_byte_length() {
        // Content-Length counts bytes, not characters.
        assert_eq!(frame("\u{e9}"), b"Content-Length: 2\r\n\r\n\xc3\xa9");
    }

    /// Talks to the real Verible language server. Run with:
    /// `LOGIX_TEST_VERIBLE_LS=<path to verible-verilog-ls>
    ///  cargo test real_language_server -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn real_language_server() {
        use std::sync::{Arc, Mutex};
        let exe = PathBuf::from(std::env::var("LOGIX_TEST_VERIBLE_LS").unwrap());
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = received.clone();
        let mut server =
            LanguageServer::start(&exe, move |m| sink.lock().unwrap().push(m), || {}).unwrap();
        server
            .send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}"#)
            .await
            .unwrap();
        server
            .send(r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#)
            .await
            .unwrap();
        let text = "module main;\\n  wire x\\nendmodule\\n";
        server
            .send(&format!(
                r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"file:///tmp/logix-test/main.v","languageId":"verilog","version":1,"text":"{text}"}}}}}}"#
            ))
            .await
            .unwrap();
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if received
                .lock()
                .unwrap()
                .iter()
                .any(|m| m.contains("publishDiagnostics"))
            {
                break;
            }
        }
        let messages = received.lock().unwrap().clone();
        for m in &messages {
            println!("{}", &m[..m.len().min(300)]);
        }
        let diag = messages
            .iter()
            .find(|m| m.contains("publishDiagnostics"))
            .expect("diagnostics");
        assert!(diag.contains("syntax error"), "{diag}");
    }
}
