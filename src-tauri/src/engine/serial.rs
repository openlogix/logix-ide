//! The Serial Monitor's engine calls: Logix Engine's serial tools
//! (its docs/tool-api.md, Serial ports). The engine finds the board's port
//! (`list_devices` `serial_port`), keeps the session, pauses it during
//! Upload and reopens it after an unplug; Logix IDE only opens, reads in a
//! loop (commands.rs, `serial_open`), sends and closes.

use anyhow::Result;
use serde::Serialize;
use serde_json::{json, Value};

use super::client::{Engine, ToolFailure};

/// One read: what arrived, and what happened to the port meanwhile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialRead {
    /// What arrived, as the board sent it (line endings included).
    pub text: String,
    /// The engine's messages for what happened, such as "The serial port
    /// was closed while the board is programmed.", in order.
    pub events: Vec<String>,
    /// Whether the port is open now (not while paused or unplugged).
    pub open: bool,
    /// The port's name now (it may change when the board comes back).
    pub port: String,
}

async fn call(engine: &Engine, tool: &str, args: Value) -> Result<Value> {
    engine
        .call(tool, args, None)
        .await?
        .map_err(|e| anyhow::Error::from(ToolFailure::from_value(&e)))
}

/// Opens `port` at `baud`; with `device` (its engine id) the engine pauses
/// it during Upload and finds it again after an unplug. Returns the
/// session's ID.
pub async fn open(engine: &Engine, port: &str, baud: u32, device: Option<&str>) -> Result<String> {
    let mut args = json!({ "port": port, "baud": baud });
    if let Some(device) = device {
        args["device"] = json!(device);
    }
    let result = call(engine, "open_serial", args).await?;
    Ok(result["session"].as_str().unwrap_or_default().to_string())
}

/// What arrived since the last read, waiting up to `wait_ms` for some.
pub async fn read(engine: &Engine, session: &str, wait_ms: u32) -> Result<SerialRead> {
    let r = call(
        engine,
        "read_serial",
        json!({ "session": session, "wait_ms": wait_ms }),
    )
    .await?;
    Ok(parse_read(&r))
}

fn parse_read(r: &Value) -> SerialRead {
    SerialRead {
        text: r["text"].as_str().unwrap_or_default().to_string(),
        events: r["events"]
            .as_array()
            .map(|events| {
                events
                    .iter()
                    .filter_map(|e| e["message"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        open: r["open"].as_bool().unwrap_or(false),
        port: r["port"].as_str().unwrap_or_default().to_string(),
    }
}

/// Sends `text` as it is (the caller adds the line ending).
pub async fn write(engine: &Engine, session: &str, text: &str) -> Result<()> {
    call(
        engine,
        "write_serial",
        json!({ "session": session, "text": text }),
    )
    .await?;
    Ok(())
}

/// Ends the session, freeing the port for other programs.
pub async fn close(engine: &Engine, session: &str) -> Result<()> {
    call(engine, "close_serial", json!({ "session": session })).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_take_the_text_events_and_port() {
        let r = parse_read(&json!({
            "text": "Hello from your FPGA board\r\n",
            "events": [{"kind": "resumed", "message": "The serial port COM5 was reopened."}],
            "open": true,
            "port": "COM5"
        }));
        assert_eq!(r.text, "Hello from your FPGA board\r\n");
        assert_eq!(r.events, ["The serial port COM5 was reopened."]);
        assert!(r.open);
        assert_eq!(r.port, "COM5");
        assert_eq!(parse_read(&json!({})).events, Vec::<String>::new());
    }
}
