//! The Serial Monitor's engine calls: Logix Engine's serial tools
//! (its docs/tool-api.md, Serial ports). The engine passes bytes through
//! unchanged, as hex, as a UART does; turning them into text and back is
//! Logix IDE's (`Utf8Decoder`, and UTF-8 for what's typed). The engine finds the board's port
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
    /// What arrived as text (UTF-8, line endings as the board sent them),
    /// decoded by the read loop (`Utf8Decoder`). Empty from `read` itself.
    pub text: String,
    /// What arrived, exactly (the engine's `hex`).
    #[serde(skip)]
    pub bytes: Vec<u8>,
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
        text: String::new(),
        bytes: hex::decode(r["hex"].as_str().unwrap_or_default()).unwrap_or_default(),
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

/// Sends `text` as UTF-8 bytes, as it is (the caller adds the line ending).
pub async fn write(engine: &Engine, session: &str, text: &str) -> Result<()> {
    let hex = hex::encode(text.as_bytes());
    call(
        engine,
        "write_serial",
        json!({ "session": session, "hex": hex }),
    )
    .await?;
    Ok(())
}

/// Turns the bytes of successive reads into text (UTF-8): a character split
/// between two reads comes whole with the second; bytes that aren't UTF-8
/// become U+FFFD.
#[derive(Default)]
pub struct Utf8Decoder {
    partial: Vec<u8>,
}

impl Utf8Decoder {
    pub fn decode(&mut self, bytes: &[u8]) -> String {
        self.partial.extend_from_slice(bytes);
        let complete = match std::str::from_utf8(&self.partial) {
            Ok(_) => self.partial.len(),
            // An incomplete character at the end: kept for next time.
            Err(e) if e.error_len().is_none() => e.valid_up_to(),
            Err(_) => self.partial.len(),
        };
        let text = String::from_utf8_lossy(&self.partial[..complete]).into_owned();
        self.partial.drain(..complete);
        text
    }
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
            "hex": "48690d0a",
            "events": [{"kind": "resumed", "message": "The serial port COM5 was reopened."}],
            "open": true,
            "port": "COM5"
        }));
        assert_eq!(r.bytes, b"Hi\r\n");
        assert_eq!(r.events, ["The serial port COM5 was reopened."]);
        assert!(r.open);
        assert_eq!(r.port, "COM5");
        assert_eq!(parse_read(&json!({})).events, Vec::<String>::new());
    }

    #[test]
    fn utf8_split_between_reads_comes_whole() {
        let mut d = Utf8Decoder::default();
        let e_acute = "é".as_bytes();
        assert_eq!(d.decode(&[b'a', e_acute[0]]), "a");
        assert_eq!(d.decode(&[e_acute[1], b'b']), "éb");
        assert_eq!(d.decode(&[0xff, b'c']), "\u{fffd}c");
    }
}
