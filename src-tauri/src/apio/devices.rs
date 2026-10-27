//! Connected USB devices and serial ports, from `apio api scan-devices`.
//! (Which device is the board is decided by Apio when uploading.)

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use super::Apio;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDevice {
    /// USB vendor and product ids, as 4 hex digits.
    pub vid: String,
    pub pid: String,
    pub manufacturer: String,
    pub product: String,
    pub serial_number: String,
    /// E.g. "FT2232H".
    pub device_type: String,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Devices {
    pub usb: Vec<UsbDevice>,
    pub serial: Vec<SerialDevice>,
}

impl Apio {
    /// Scans the connected devices (silently).
    pub async fn scan_devices(&self) -> Result<Devices> {
        parse_devices(&self.api_json("scan-devices", None).await?)
    }
}

fn parse_devices(json: &Value) -> Result<Devices> {
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
    let usb = list("usb-devices")?
        .iter()
        .map(|d| UsbDevice {
            vid: text(d, "vid"),
            pid: text(d, "pid"),
            manufacturer: text(d, "manufacturer"),
            product: text(d, "product"),
            serial_number: text(d, "serial-number"),
            // Apio uses "device_type" here and "device-type" for serial ports.
            device_type: Some(text(d, "device_type"))
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| text(d, "device-type")),
        })
        .collect();
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
    Ok(Devices { usb, serial })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_devices() {
        let json = json!({
            "usb-devices": [{
                "vid": "0403", "pid": "6010", "bus": "0", "device": "2",
                "manufacturer": "AlhambraBits", "product": "Alhambra II v1.0A",
                "serial-number": "FTATANKC", "device_type": "FT2232H"
            }],
            "serial-devices": [{
                "port": "/dev/cu.usbserial-FTATANKC0", "port-name": "cu.usbserial-FTATANKC0",
                "vendor-id": "0403", "product-id": "6010",
                "manufacturer": "AlhambraBits", "product": "Alhambra II v1.0A",
                "serial-number": "FTATANKCA", "device-type": "FT2232H"
            }]
        });
        let d = parse_devices(&json).unwrap();
        assert_eq!(d.usb[0].product, "Alhambra II v1.0A");
        assert_eq!(
            (d.usb[0].vid.as_str(), d.usb[0].pid.as_str()),
            ("0403", "6010")
        );
        assert_eq!(d.usb[0].device_type, "FT2232H");
        assert_eq!(d.serial[0].port, "/dev/cu.usbserial-FTATANKC0");
        assert_eq!(d.serial[0].vid, "0403");
    }

    #[test]
    fn handles_nothing_connected_and_bad_json() {
        let d = parse_devices(&json!({"usb-devices": [], "serial-devices": []})).unwrap();
        assert!(d.usb.is_empty() && d.serial.is_empty());
        assert!(parse_devices(&json!({})).is_err());
    }
}
