//! Serial ports, from `apio api scan-devices` (Tools > Connected Devices).
//! The USB devices come from Logix Engine; TODO(revisit): the engine
//! doesn't list serial ports yet. Move this there once it does.

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use super::Apio;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialDevice {
    /// E.g. "/dev/cu.usbserial-FTATANKC0".
    pub port: String,
    pub vid: String,
    pub pid: String,
    pub manufacturer: String,
    pub product: String,
}

impl Apio {
    /// Scans the serial ports (silently).
    pub async fn serial_ports(&self) -> Result<Vec<SerialDevice>> {
        parse_serial_ports(&self.api_json("scan-devices", None).await?)
    }
}

fn parse_serial_ports(json: &Value) -> Result<Vec<SerialDevice>> {
    let list = |key: &str| -> Result<Vec<Value>> {
        Ok(json
            .get(key)
            .context("unexpected device list from Apio")?
            .as_array()
            .cloned()
            .unwrap_or_default())
    };
    let text = |v: &Value, key: &str| {
        v.get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let serial = list("serial-devices")?
        .iter()
        .map(|d| SerialDevice {
            port: text(d, "port"),
            vid: text(d, "vendor-id"),
            pid: text(d, "product-id"),
            manufacturer: text(d, "manufacturer"),
            product: text(d, "product"),
        })
        .collect();
    Ok(serial)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_serial_ports() {
        let json = json!({
            "usb-devices": [],
            "serial-devices": [{
                "port": "/dev/cu.usbserial-FTATANKC0", "port-name": "cu.usbserial-FTATANKC0",
                "vendor-id": "0403", "product-id": "6010",
                "manufacturer": "AlhambraBits", "product": "Alhambra II v1.0A",
                "serial-number": "FTATANKCA", "device-type": "FT2232H"
            }]
        });
        let ports = parse_serial_ports(&json).unwrap();
        assert_eq!(ports[0].port, "/dev/cu.usbserial-FTATANKC0");
        assert_eq!(ports[0].vid, "0403");
        assert_eq!(ports[0].product, "Alhambra II v1.0A");
    }

    #[test]
    fn handles_nothing_connected_and_bad_json() {
        assert!(parse_serial_ports(&json!({"serial-devices": []}))
            .unwrap()
            .is_empty());
        assert!(parse_serial_ports(&json!({})).is_err());
    }
}
