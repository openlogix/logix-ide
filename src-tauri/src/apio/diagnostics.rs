//! Turns Apio's (and its tools') console output into diagnostics with file
//! and line, so the editor can mark them. Text parsing is a stopgap until
//! Apio reports diagnostics as events (apio#1184).

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    /// Project file name, if the message points into one.
    pub file: Option<String>,
    /// 1-based line, if known.
    pub line: Option<u32>,
    /// 1-based column, if known.
    pub column: Option<u32>,
    pub message: String,
}

/// `main.v:7: ERROR: syntax error` (yosys), `main.v:5.3-5.9: Warning: ...`,
/// `main_tb.v:12: error: ...` (iverilog).
static FILE_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<file>[^\s:][^:]*):(?P<line>\d+)(?:\.(?P<col>\d+)(?:-\d+\.\d+)?)?:\s*(?P<sev>(?i:error|warning))\s*:\s*(?P<msg>.*)$",
    )
    .unwrap()
});

/// `%Error: main.v:5:3: msg` (verilator, used by lint).
static VERILATOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^%(?P<sev>Error|Warning)(?:-[\w]+)?:\s*(?P<file>[^\s:][^:]*):(?P<line>\d+):(?:(?P<col>\d+):)?\s*(?P<msg>.*)$",
    )
    .unwrap()
});

/// `*** ASSERTION FAILED at main_tb.v:52: expected 'h01, actual 'h00`
/// (the check macro in Apio's example testbenches).
static ASSERTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\*+\s*ASSERTION FAILED at (?P<file>[^\s:][^:]*):(?P<line>\d+):?\s*(?P<msg>.*)$")
        .unwrap()
});

/// `FATAL: main_tb.v:57: msg` (a testbench's `$fatal`, from vvp).
static FATAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^FATAL:\s*(?P<file>[^\s:][^:]*):(?P<line>\d+):\s*(?P<msg>.*)$").unwrap()
});

/// `main_tb.v:64: syntax error` (iverilog, without an "error:" tag).
static SYNTAX_ERROR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<file>[^\s:][^:]*):(?P<line>\d+):\s*(?P<msg>syntax error.*)$").unwrap()
});

/// `bad.v: bad.v:4:1-3: syntax error at token "end"` (Verible, used by
/// `apio format`): an optional `file: ` prefix, then `file:line:col[-col]:`.
static VERIBLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:[^\s:]+:\s+)?(?P<file>[^\s:][^:]*):(?P<line>\d+):(?P<col>\d+)(?:-\d+)?:\s*(?P<msg>.*)$",
    )
    .unwrap()
});

/// `Warning: unmatched constraint 'x' (on line 6)` (nextpnr, about the
/// constraint file).
static CONSTRAINT_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<sev>(?i:error|warning)):\s*(?P<msg>.*?)\s*\(on line (?P<line>\d+)\)\s*$")
        .unwrap()
});

/// `ERROR: Loading PCF failed.`, `Error: No matching USB device.`
static PLAIN_ERROR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:ERROR|Error):\s*(?P<msg>\S.*)$").unwrap());

/// Parses one output line. `project_files` are the names of the files in
/// the build folder (the same names as in the project); `constraint_file`
/// is the project's pin constraint file, if any.
pub fn parse_line(
    line: &str,
    project_files: &[String],
    constraint_file: Option<&str>,
) -> Option<Diagnostic> {
    let line = line.trim_end();
    let located = |caps: &regex::Captures, file: Option<&str>| {
        let file = file.map(|f| f.trim_start_matches("./").to_string());
        // Locations in files outside the project (e.g. tool libraries)
        // are reported without a location.
        let in_project = file
            .as_ref()
            .is_some_and(|f| project_files.iter().any(|p| p == f));
        Diagnostic {
            severity: severity(&caps["sev"]),
            file: if in_project { file } else { None },
            line: in_project
                .then(|| caps.name("line").and_then(|m| m.as_str().parse().ok()))
                .flatten(),
            column: in_project
                .then(|| caps.name("col").and_then(|m| m.as_str().parse().ok()))
                .flatten(),
            message: caps["msg"].trim().to_string(),
        }
    };
    if let Some(caps) = ASSERTION.captures(line) {
        let mut d = located_error(&caps, project_files);
        d.message = format!("Assertion failed: {}", d.message);
        return Some(d);
    }
    if let Some(caps) = FATAL.captures(line) {
        let mut d = located_error(&caps, project_files);
        if d.message.is_empty() {
            d.message = "The testbench stopped with $fatal.".into();
        }
        return Some(d);
    }
    if let Some(caps) = SYNTAX_ERROR.captures(line) {
        return Some(located_error(&caps, project_files));
    }
    if let Some(caps) = VERILATOR.captures(line) {
        return Some(located(&caps, caps.name("file").map(|m| m.as_str())));
    }
    if let Some(caps) = FILE_LINE.captures(line) {
        return Some(located(&caps, caps.name("file").map(|m| m.as_str())));
    }
    if let Some(caps) = VERIBLE.captures(line) {
        let mut d = located_error(&caps, project_files);
        if d.line.is_some() {
            d.column = caps["col"].parse().ok();
        }
        return Some(d);
    }
    if let Some(caps) = CONSTRAINT_LINE.captures(line) {
        return Some(located(&caps, constraint_file));
    }
    if let Some(caps) = PLAIN_ERROR.captures(line) {
        return Some(Diagnostic {
            severity: Severity::Error,
            file: None,
            line: None,
            column: None,
            message: caps["msg"].trim().to_string(),
        });
    }
    None
}

/// An error at the captured `file`/`line` (dropped if outside the project).
fn located_error(caps: &regex::Captures, project_files: &[String]) -> Diagnostic {
    let file = caps["file"].trim_start_matches("./").to_string();
    let in_project = project_files.contains(&file);
    Diagnostic {
        severity: Severity::Error,
        file: in_project.then_some(file),
        line: if in_project {
            caps["line"].parse().ok()
        } else {
            None
        },
        column: None,
        message: caps["msg"].trim().to_string(),
    }
}

fn severity(s: &str) -> Severity {
    if s.eq_ignore_ascii_case("warning") {
        Severity::Warning
    } else {
        Severity::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<String> {
        vec!["main.v".into(), "main.sv".into(), "pinout.pcf".into()]
    }

    fn parse(line: &str) -> Option<Diagnostic> {
        parse_line(line, &files(), Some("pinout.pcf"))
    }

    fn diag(sev: Severity, file: Option<&str>, line: Option<u32>, msg: &str) -> Diagnostic {
        Diagnostic {
            severity: sev,
            file: file.map(Into::into),
            line,
            column: None,
            message: msg.into(),
        }
    }

    #[test]
    fn yosys_errors() {
        assert_eq!(
            parse("main.v:7: ERROR: syntax error, unexpected TOK_ENDMODULE"),
            Some(diag(
                Severity::Error,
                Some("main.v"),
                Some(7),
                "syntax error, unexpected TOK_ENDMODULE"
            ))
        );
        // No space after the colon.
        assert_eq!(
            parse("main.v:7: ERROR:Can't resolve function name `\\foo'.")
                .unwrap()
                .message,
            "Can't resolve function name `\\foo'."
        );
        assert_eq!(
            parse("main.sv:3: ERROR: x").unwrap().file.as_deref(),
            Some("main.sv")
        );
    }

    #[test]
    fn yosys_warning_with_column_range() {
        let d = parse("main.v:5.3-5.9: Warning: Identifier `x' is implicitly declared.").unwrap();
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!((d.line, d.column), (Some(5), Some(3)));
    }

    #[test]
    fn verilator_messages() {
        let d = parse("%Warning-UNUSED: main.v:12:9: Signal is not used: 'x'").unwrap();
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(
            (d.file.as_deref(), d.line, d.column),
            (Some("main.v"), Some(12), Some(9))
        );
    }

    #[test]
    fn constraint_file_messages() {
        assert_eq!(
            parse("Warning: unmatched constraint 'leds[1]' (on line 10)"),
            Some(diag(
                Severity::Warning,
                Some("pinout.pcf"),
                Some(10),
                "unmatched constraint 'leds[1]'"
            ))
        );
    }

    #[test]
    fn errors_without_location() {
        assert_eq!(
            parse("ERROR: IO 'LED0' is unconstrained in PCF (override this error with --pcf-allow-unconstrained)"),
            Some(diag(
                Severity::Error,
                None,
                None,
                "IO 'LED0' is unconstrained in PCF (override this error with --pcf-allow-unconstrained)"
            ))
        );
        assert_eq!(
            parse("Error: No matching USB device."),
            Some(diag(Severity::Error, None, None, "No matching USB device."))
        );
    }

    #[test]
    fn testbench_failures() {
        let files = vec!["main.v".to_string(), "main_tb.v".to_string()];
        let p = |l: &str| parse_line(l, &files, None);
        assert_eq!(
            p("*** ASSERTION FAILED at main_tb.v:52: expected 'h01, actual 'h00"),
            Some(diag(
                Severity::Error,
                Some("main_tb.v"),
                Some(52),
                "Assertion failed: expected 'h01, actual 'h00"
            ))
        );
        assert_eq!(
            p("FATAL: main_tb.v:57: "),
            Some(diag(
                Severity::Error,
                Some("main_tb.v"),
                Some(57),
                "The testbench stopped with $fatal."
            ))
        );
        assert_eq!(
            p("main_tb.v:64: syntax error"),
            Some(diag(
                Severity::Error,
                Some("main_tb.v"),
                Some(64),
                "syntax error"
            ))
        );
        assert_eq!(
            p("main_tb.v:64: error: Invalid module item.")
                .unwrap()
                .message,
            "Invalid module item."
        );
        assert_eq!(p("main_tb.v:64: $finish called at 316000 (1ps)"), None);
        assert_eq!(p("I give up."), None);
    }

    #[test]
    fn verible_format_errors() {
        let files = vec!["bad.v".to_string()];
        let d = parse_line(
            "bad.v: bad.v:4:1-3: syntax error at token \"end\"",
            &files,
            None,
        )
        .unwrap();
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(
            (d.file.as_deref(), d.line, d.column),
            (Some("bad.v"), Some(4), Some(1))
        );
        assert_eq!(d.message, "syntax error at token \"end\"");
    }

    #[test]
    fn locations_outside_the_project_are_dropped() {
        let d = parse("/opt/yosys/share/cells.v:10: ERROR: boom").unwrap();
        assert_eq!((d.file, d.line), (None, None));
    }

    #[test]
    fn ignores_other_lines() {
        for line in [
            "yosys -p \"synth_ice40 -top main\" -q main.v",
            "scons: *** [_build/default/hardware.json] Error 1",
            "========================== [ERROR] Took 0.38 seconds ===",
            "9 warnings, 2 errors",
            "Setting shell vars.",
        ] {
            assert_eq!(parse(line), None, "{line}");
        }
    }
}
