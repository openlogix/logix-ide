//! The build report: resource usage and maximum clock speeds, from
//! `apio api get-build-report` after a successful build.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use super::Apio;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildReport {
    /// FPGA family, e.g. "ice40", "ecp5", "gowin", "xilinx".
    pub arch: String,
    /// The FPGA's resources, with Apio's (family-specific) names.
    pub resources: Vec<Resource>,
    pub clocks: Vec<Clock>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub name: String,
    pub used: u64,
    pub available: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Clock {
    pub name: String,
    pub fmax_mhz: f64,
}

impl Apio {
    /// Reads the report of the last build in `dir` (silently).
    pub async fn build_report(&self, dir: &Path) -> Result<BuildReport> {
        parse_build_report(&self.api_json("get-build-report", Some(dir)).await?)
    }
}

fn parse_build_report(json: &Value) -> Result<BuildReport> {
    let report = json
        .get("build-report")
        .context("unexpected build report from Apio")?;
    let arch = report
        .get("arch")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let resources = report
        .get("resources")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .map(|(name, r)| Resource {
                    name: name.clone(),
                    used: r.get("used").and_then(Value::as_u64).unwrap_or(0),
                    available: r.get("available").and_then(Value::as_u64).unwrap_or(0),
                })
                .collect()
        })
        .unwrap_or_default();
    let clocks = report
        .get("clocks")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(name, c)| {
                    Some(Clock {
                        name: name.clone(),
                        fmax_mhz: c.get("fmax_mhz")?.as_f64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(BuildReport {
        arch,
        resources,
        clocks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_report() {
        let json = json!({"build-report": {
            "arch": "ice40",
            "env": "default",
            "resources": {
                "ICESTORM_LC": {"used": 27, "available": 7680, "percentage": 0.35},
                "SB_IO": {"used": 9, "available": 107, "percentage": 8.4}
            },
            "clocks": {"sys_clk": {"fmax_mhz": 194.36}}
        }});
        let r = parse_build_report(&json).unwrap();
        assert_eq!(r.arch, "ice40");
        assert_eq!(
            r.resources[0],
            Resource {
                name: "ICESTORM_LC".into(),
                used: 27,
                available: 7680
            }
        );
        assert_eq!(r.clocks[0].name, "sys_clk");
        assert!((r.clocks[0].fmax_mhz - 194.36).abs() < 1e-9);
    }

    #[test]
    fn tolerates_missing_parts() {
        let r = parse_build_report(&json!({"build-report": {"arch": "ecp5"}})).unwrap();
        assert!(r.resources.is_empty() && r.clocks.is_empty());
        assert!(parse_build_report(&json!({})).is_err());
    }
}
