//! Bare Minimum projects (named after the Arduino IDE's BareMinimum
//! example): a board's pins and a design that does nothing yet, to start
//! from scratch. Plain code, no Tauri or Apio.
//!
//! Apio's example pin files list only the pins their example uses, and the
//! tools reject a port without a pin (and an empty design), so Bare Minimum
//! takes both the pin file and the ports from the board's starter example
//! (blinky) and sets the outputs to 0.

use std::sync::LazyLock;

use regex::Regex;

use crate::project::ProjectFile;

/// Pin constraint file extensions (the formats used by Apio's boards).
const PIN_FILE_EXTENSIONS: &[&str] = &["pcf", "lpf", "cst", "xdc", "pdc", "ccf"];

/// `main.v` for a board without an example to take the pins from.
pub const NO_PINS_MAIN: &str = "\
// The top module must be named 'main'.
//
// There's no example for this board to take its pins from. Before
// Verify, add a pin file (File > New File...) that assigns the ports
// below to the FPGA's pins, in the format the board uses (e.g. a .pcf
// file for iCE40 boards). See the board's documentation for the pins.

module main (
    input  CLK,   // Clock
    output LED0   // An LED
);

  // Your design goes here.

endmodule
";

/// Whether `name` is a pin constraint file.
pub fn is_pin_file(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| PIN_FILE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// The files of a Bare Minimum project, from the board's starter example
/// files: its pin file and a `main.v` with the same ports. `None` if the
/// starter has no pin file or its ports can't be read.
pub fn bare_files(starter: &[ProjectFile]) -> Option<Vec<ProjectFile>> {
    let pins = starter.iter().find(|f| is_pin_file(&f.name))?;
    let main = starter.iter().find(|f| f.name == "main.v")?;
    let text = bare_main(&main.text, &pins.name)?;
    Some(vec![
        ProjectFile {
            name: "main.v".into(),
            text,
        },
        pins.clone(),
    ])
}

/// One port of the top module.
#[derive(Debug, PartialEq)]
struct Port {
    direction: String,
    /// E.g. "[7:0]", or "" for one bit.
    range: String,
    name: String,
}

/// A `main.v` with the ports of `starter_main`'s `main` module, its outputs
/// set to 0. `None` if the ports can't be read or there are no outputs
/// (the tools reject a design with nothing in it).
fn bare_main(starter_main: &str, pin_file: &str) -> Option<String> {
    let ports = main_ports(&strip_comments(starter_main))?;
    let outputs: Vec<&Port> = ports.iter().filter(|p| p.direction == "output").collect();
    if outputs.is_empty() {
        return None;
    }
    let range_width = ports.iter().map(|p| p.range.len()).max().unwrap_or(0);
    let declarations: Vec<String> = ports
        .iter()
        .map(|p| {
            let range = if range_width > 0 {
                format!("{:<range_width$} ", p.range)
            } else {
                String::new()
            };
            format!("    {:<6} {range}{}", p.direction, p.name)
        })
        .collect();
    let assigns: Vec<String> = outputs
        .iter()
        .map(|p| format!("  assign {} = 0;", p.name))
        .collect();
    Some(format!(
        "\
// Bare minimum: the board's pins, and a design that does nothing yet.
//
// The ports below are the pins in {pin_file}. To use more pins, add them
// to both. The outputs are set to 0 for now (an active-low LED is then
// on); replace that with your design.

module main (
{}
);

{}

endmodule
",
        declarations.join(",\n"),
        assigns.join("\n"),
    ))
}

/// Removes `//` and `/* */` comments.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("//") {
            rest = after.find('\n').map_or("", |i| &after[i..]);
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = after.find("*/").map_or("", |i| &after[i + 2..]);
            out.push(' ');
        } else {
            let c = rest.chars().next().unwrap_or(' ');
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

/// The ports of `module main`, declared in the port list (ANSI style), in
/// comment-free text. Parameters (`#(...)`) are skipped.
fn main_ports(text: &str) -> Option<Vec<Port>> {
    static MODULE_MAIN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bmodule\s+main\b").expect("valid regex"));
    let found = MODULE_MAIN.find(text)?;
    let mut rest = text[found.end()..].trim_start();
    if let Some(after) = rest.strip_prefix('#') {
        let (_, after) = balanced(after.trim_start())?;
        rest = after.trim_start();
    }
    let (list, _) = balanced(rest)?;
    let mut ports = Vec::new();
    let mut direction: Option<String> = None;
    let mut range = String::new();
    for item in list.split(',') {
        let mut words = item.split_whitespace().peekable();
        if let Some(&first) = words.peek() {
            if matches!(first, "input" | "output" | "inout") {
                direction = Some(first.to_string());
                range.clear();
                words.next();
                // Net types aren't needed: the outputs are driven by assign.
                while words
                    .peek()
                    .is_some_and(|w| matches!(*w, "wire" | "reg" | "logic" | "signed"))
                {
                    words.next();
                }
                if words.peek().is_some_and(|w| w.starts_with('[')) {
                    let mut r = String::new();
                    for w in words.by_ref() {
                        r.push_str(w);
                        if w.ends_with(']') {
                            break;
                        }
                    }
                    range = r;
                }
            }
        }
        let name: Vec<&str> = words.collect();
        // `output a, b` declares b like a; anything else is too unusual.
        let [name] = name.as_slice() else {
            return None;
        };
        if !crate::project::is_verilog_identifier(name) {
            return None;
        }
        ports.push(Port {
            direction: direction.clone()?,
            range: range.clone(),
            name: name.to_string(),
        });
    }
    (!ports.is_empty()).then_some(ports)
}

/// For text starting with `(`: the contents up to the matching `)`, and
/// the text after it.
fn balanced(text: &str) -> Option<(&str, &str)> {
    let inner = text.strip_prefix('(')?;
    let mut depth = 1;
    for (i, c) in inner.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&inner[..i], &inner[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALHAMBRA_BLINKY: &str = "\
// N is the counter width. leds[7] toggles every 2^(N-1) clocks.
module main #(
    parameter integer N = 24
) (
    input        sys_clk,  // 12MHz clock
    output [7:0] leds      // leds[7] blinks (the rest are off). Active high.
);
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1;
  assign leds = {counter[N-1], 7'b0};
endmodule
";

    fn file(name: &str, text: &str) -> ProjectFile {
        ProjectFile {
            name: name.into(),
            text: text.into(),
        }
    }

    #[test]
    fn keeps_the_starter_ports_and_sets_outputs_to_zero() {
        let main = bare_main(ALHAMBRA_BLINKY, "pinout.pcf").unwrap();
        assert!(main.contains("pins in pinout.pcf"), "{main}");
        assert!(
            main.contains(
                "module main (\n    input        sys_clk,\n    output [7:0] leds\n);\n\n  assign leds = 0;\n"
            ),
            "{main}"
        );
        assert!(!main.contains("counter"), "{main}");
    }

    #[test]
    fn reads_other_port_styles() {
        let main =
            "module main(input wire clk, output reg a, b, inout [3:0] /* bus */ io);\nendmodule";
        let ports = main_ports(&strip_comments(main)).unwrap();
        let names: Vec<_> = ports
            .iter()
            .map(|p| format!("{} {}{}", p.direction, p.range, p.name))
            .collect();
        assert_eq!(
            names,
            ["input clk", "output a", "output b", "inout [3:0]io"]
        );
        let text = bare_main(main, "pins.lpf").unwrap();
        assert!(
            text.contains("  assign a = 0;\n  assign b = 0;\n"),
            "{text}"
        );
        assert!(!text.contains("io = 0"), "{text}");
    }

    #[test]
    fn gives_up_on_what_it_cant_read() {
        // Old-style ports, no outputs, no main module.
        assert_eq!(
            bare_main("module main(a);\noutput a;\nendmodule", "p.pcf"),
            None
        );
        assert_eq!(bare_main("module main(input a);\nendmodule", "p.pcf"), None);
        assert_eq!(
            bare_main("module mainx(output a);\nendmodule", "p.pcf"),
            None
        );
        assert_eq!(bare_main("// module main(output a);\n", "p.pcf"), None);
    }

    #[test]
    fn takes_the_pin_file_from_the_starter() {
        let starter = [
            file("main.v", ALHAMBRA_BLINKY),
            file("main_tb.v", "module main_tb; endmodule"),
            file("pinout.pcf", "set_io sys_clk 49"),
        ];
        let files = bare_files(&starter).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["main.v", "pinout.pcf"]);
        assert_eq!(files[1].text, "set_io sys_clk 49");
        assert_eq!(bare_files(&starter[..2]), None);
    }

    #[test]
    fn knows_pin_files() {
        assert!(is_pin_file("pinout.pcf"));
        assert!(is_pin_file("x.LPF"));
        assert!(!is_pin_file("main.v"));
        assert!(!is_pin_file("pcf"));
    }
}

/// Checks Bare Minimum against Apio's real examples: run with
/// `LOGIX_EXAMPLES_DIR=<apio packages>/definitions/examples cargo test
/// survey_bare_minimum -- --ignored --nocapture`. With `LOGIX_BARE_OUT=<dir>`
/// it also writes each board's project there, to build with Apio.
#[cfg(test)]
mod survey {
    use super::*;
    use std::path::Path;

    #[test]
    #[ignore]
    fn survey_bare_minimum() {
        let root = std::env::var("LOGIX_EXAMPLES_DIR").expect("LOGIX_EXAMPLES_DIR");
        let out = std::env::var("LOGIX_BARE_OUT").ok();
        let mut boards: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .collect();
        boards.sort();
        let (mut ok, mut failed) = (0, Vec::new());
        for board in boards {
            let Some(dir) = ["blinky", "ledon"]
                .iter()
                .map(|n| board.join(n))
                .find(|d| d.is_dir())
            else {
                continue;
            };
            let files: Vec<ProjectFile> = std::fs::read_dir(&dir)
                .unwrap()
                .flatten()
                .filter(|e| e.path().is_file())
                .map(|e| ProjectFile {
                    name: e.file_name().to_string_lossy().into_owned(),
                    text: std::fs::read_to_string(e.path()).unwrap_or_default(),
                })
                .collect();
            let name = board.file_name().unwrap().to_string_lossy().into_owned();
            match bare_files(&files) {
                Some(bare) => {
                    ok += 1;
                    if let Some(out) = &out {
                        let d = Path::new(out).join(&name);
                        std::fs::create_dir_all(&d).unwrap();
                        for f in &bare {
                            std::fs::write(d.join(&f.name), &f.text).unwrap();
                        }
                        std::fs::write(
                            d.join("apio.ini"),
                            format!("[env:default]\nboard = {name}\ntop-module = main\n"),
                        )
                        .unwrap();
                    }
                }
                None => failed.push(name),
            }
        }
        println!(
            "Bare Minimum: {ok} boards, can't read {}: {failed:?}",
            failed.len()
        );
    }
}
