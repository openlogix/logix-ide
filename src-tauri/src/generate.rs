//! Generates every board's `blinky` and Bare Minimum from Logix Engine's
//! board catalog (`get_catalog`: each board's resources and requirements,
//! and its FPGA), in the examples' style. Build time, not runtime: run by
//! `scripts/generate-examples.sh`, which writes them to
//! `examples/<board>/blinky/` and `examples/<board>/bare-minimum/`, and the
//! catalog it used to `src-tauri/generated-examples-catalog.json`, from
//! which a test checks they're up to date. Plain code, no Tauri.
//!
//! The rules (Logix Catalog's docs/model.md and docs/design.md):
//! - Only what works on the board as shipped: a resource with a `setup`
//!   requirement (such as a jumper) is left out.
//! - The board's `drive` requirements are always included, and those of the
//!   resources used.
//! - Choices never depend on the order of resources: the clock is the one
//!   with the lowest name; LEDs are every `leds`, `rgb_led` and `led_matrix`
//!   resource, and the one that blinks is the first by type (in that order),
//!   then by name.
//! - Bare Minimum for every board: its known pins, outputs off and drives
//!   at their levels (Logix Engine bcfa672 or later builds a design without
//!   logic on every family); with none known, an empty `main` and a
//!   pin file that shows how to add pins.
//! - No clock usable: the FPGA's internal oscillator where it has one this
//!   module knows (iCE40 UltraPlus and iCE5LP: SB_HFOSC, 48 MHz; ECP5:
//!   OSCG, about 2.4 MHz).
//! - An LED resource on all three of the iCE40 UltraPlus's RGB LED pins goes
//!   through its RGB LED driver (SB_RGBA_DRV, in `rgb_driver.v`), as the
//!   hand-written RGB examples did; one LED on one of those pins is a plain
//!   output, as the iCE40-UP5K, iCESugar 1.5 and OK-iCE40Pro examples were
//!   (both built with nextpnr). Through the driver, LEDs turn on low (it
//!   only sinks current), also when the catalog doesn't know their level
//!   (the Soan Papdi Mini).
//! - Pull and drive are left to each family's defaults; pin files name the
//!   I/O standard where the family uses one, and the clock's frequency where
//!   the toolchain reads it (ECP5 `FREQUENCY`, Xilinx `create_clock`).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Deserialize;

/// The parts of `get_catalog`'s result the generator uses.
#[derive(Debug, Deserialize)]
pub struct Catalog {
    pub boards: BTreeMap<String, Board>,
    pub fpgas: BTreeMap<String, Fpga>,
}

#[derive(Debug, Deserialize)]
pub struct Board {
    pub name: String,
    pub fpga_id: String,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub requires: Vec<Requirement>,
    #[serde(default)]
    pub resources: BTreeMap<String, Resource>,
}

#[derive(Debug, Deserialize)]
pub struct Fpga {
    pub architecture: String,
    pub part_number: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Requirement {
    Drive(Drive),
    // What the person does (`text`) isn't used: generators leave the resource out.
    Setup {},
}

#[derive(Debug, Clone, Deserialize)]
pub struct Drive {
    pub name: String,
    pub pins: Vec<String>,
    pub level: Level,
    pub io_standard: Option<String>,
    pub why: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    High,
    Low,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Colors {
    pub red: String,
    pub green: String,
    pub blue: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Resource {
    Clock {
        pin: String,
        mhz: f64,
        io_standard: Option<String>,
        #[serde(default)]
        requires: Vec<Requirement>,
    },
    Leds {
        pins: Vec<String>,
        active: Option<Level>,
        io_standard: Option<String>,
        #[serde(default)]
        requires: Vec<Requirement>,
    },
    RgbLed {
        red: String,
        green: String,
        blue: String,
        active: Option<Level>,
        io_standard: Option<String>,
        #[serde(default)]
        requires: Vec<Requirement>,
    },
    LedMatrix {
        leds: Vec<String>,
        colors: Colors,
        active: Option<Level>,
        io_standard: Option<String>,
        #[serde(default)]
        requires: Vec<Requirement>,
    },
    #[serde(other)]
    Other,
}

impl Resource {
    fn requires(&self) -> &[Requirement] {
        match self {
            Resource::Clock { requires, .. }
            | Resource::Leds { requires, .. }
            | Resource::RgbLed { requires, .. }
            | Resource::LedMatrix { requires, .. } => requires,
            Resource::Other => &[],
        }
    }

    fn needs_setup(&self) -> bool {
        self.requires()
            .iter()
            .any(|q| matches!(q, Requirement::Setup { .. }))
    }

    fn drives(&self) -> impl Iterator<Item = &Drive> {
        self.requires().iter().filter_map(|q| match q {
            Requirement::Drive(d) => Some(d),
            Requirement::Setup { .. } => None,
        })
    }
}

/// A generated project: its folder name and files, by name.
#[derive(Debug, Clone, PartialEq)]
pub struct Generated {
    pub name: String,
    pub files: BTreeMap<String, String>,
}

/// The board's generated projects: `blinky` when it has LEDs and a clock
/// (a pin or an internal oscillator), and `bare-minimum` always: the
/// board's known pins (none when none are known) and a design that does
/// nothing yet.
/// Errors for a board this module can't handle, such as an FPGA family it
/// doesn't know.
pub fn generate(id: &str, board: &Board, fpga: &Fpga) -> Result<Vec<Generated>, String> {
    let plan = Plan::new(id, board, fpga)?;
    let mut out = Vec::new();
    if let Some(blinky) = plan.blinky() {
        out.push(blinky);
    }
    if let Some(bare) = plan.bare_minimum() {
        out.push(bare);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// The plan: what a board's designs use.

#[derive(Debug, Clone, Copy, PartialEq)]
enum Arch {
    Ice40,
    Ecp5,
    Gowin,
    Xilinx,
}

/// The internal oscillators this module knows.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Osc {
    /// iCE40 UltraPlus and iCE5LP: SB_HFOSC at 48 MHz.
    Hfosc,
    /// ECP5: OSCG, 310 MHz / 128, about 2.4 MHz.
    Oscg,
}

impl Osc {
    fn mhz(self) -> f64 {
        match self {
            Osc::Hfosc => 48.0,
            Osc::Oscg => 310.0 / 128.0,
        }
    }
    fn file(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Osc::Hfosc => ("hf_osc.v", "hf_osc", HF_OSC_V),
            Osc::Oscg => ("osc.v", "osc", OSC_V),
        }
    }
}

#[derive(Debug, Clone)]
enum Clock {
    Pin {
        name: String,
        pin: String,
        mhz: f64,
        io_standard: Option<String>,
    },
    Internal(Osc),
}

impl Clock {
    fn mhz(&self) -> f64 {
        match self {
            Clock::Pin { mhz, .. } => *mhz,
            Clock::Internal(o) => o.mhz(),
        }
    }
}

/// One LED resource as ports: their names, pins (bit 0 first) and what the
/// bits are.
#[derive(Debug, Clone)]
struct LedGroup {
    kind: LedKind,
    /// The level that turns them on; None when the catalog doesn't know.
    active: Option<Level>,
    io_standard: Option<String>,
    /// For LEDs on the iCE40 UltraPlus's RGB LED pins: the driver output
    /// (0, 1 or 2 for RGB0..RGB2) of each pin, bit 0 first.
    rgb_driver: Option<Vec<u8>>,
    drives: Vec<Drive>,
}

#[derive(Debug, Clone)]
enum LedKind {
    /// A plain port (one pin) or a vector.
    Leds { name: String, pins: Vec<String> },
    /// `name[2:0]`: [0] red, [1] green, [2] blue.
    Rgb { name: String, pins: [String; 3] },
    /// `name` (a pin per LED) and `name_color[2:0]` (red, green, blue).
    Matrix {
        name: String,
        leds: Vec<String>,
        colors: [String; 3],
    },
}

impl LedKind {
    fn order(&self) -> u8 {
        match self {
            LedKind::Leds { .. } => 0,
            LedKind::Rgb { .. } => 1,
            LedKind::Matrix { .. } => 2,
        }
    }
    fn name(&self) -> &str {
        match self {
            LedKind::Leds { name, .. }
            | LedKind::Rgb { name, .. }
            | LedKind::Matrix { name, .. } => name,
        }
    }
}

struct Plan<'a> {
    board: &'a Board,
    arch: Arch,
    clock: Option<Clock>,
    leds: Vec<LedGroup>,
    drives: Vec<Drive>,
}

/// The iCE40 UltraPlus's RGB LED driver pins, by package: RGB0, RGB1, RGB2.
/// From Lattice's iCE40 UltraPlus family data sheet (pin tables), and the
/// boards' examples built with nextpnr.
fn rgb_driver_pins(part: &str) -> Option<[&'static str; 3]> {
    let part = part.to_ascii_uppercase();
    if !part.starts_with("ICE40UP") {
        return None;
    }
    if part.ends_with("-SG48") {
        Some(["39", "40", "41"])
    } else if part.ends_with("-UWG30") {
        Some(["A5", "B5", "C5"])
    } else {
        None
    }
}

fn internal_osc(arch: Arch, part: &str) -> Option<Osc> {
    let part = part.to_ascii_uppercase();
    match arch {
        Arch::Ice40 if part.starts_with("ICE40UP") || part.starts_with("ICE5LP") => {
            Some(Osc::Hfosc)
        }
        Arch::Ecp5 => Some(Osc::Oscg),
        _ => None,
    }
}

impl<'a> Plan<'a> {
    fn new(id: &str, board: &'a Board, fpga: &Fpga) -> Result<Plan<'a>, String> {
        let arch = match fpga.architecture.as_str() {
            "ice40" => Arch::Ice40,
            "ecp5" => Arch::Ecp5,
            "gowin" => Arch::Gowin,
            "xilinx" => Arch::Xilinx,
            other => return Err(format!("{id}: unknown architecture {other}")),
        };
        let usable = |r: &Resource| !r.needs_setup();
        let clock = board
            .resources
            .iter()
            .filter(|(_, r)| usable(r))
            .find_map(|(name, r)| match r {
                Resource::Clock {
                    pin,
                    mhz,
                    io_standard,
                    ..
                } => Some(Clock::Pin {
                    name: name.clone(),
                    pin: pin.clone(),
                    mhz: *mhz,
                    io_standard: io_standard.clone(),
                }),
                _ => None,
            })
            .or_else(|| internal_osc(arch, &fpga.part_number).map(Clock::Internal));
        let rgb_pins = rgb_driver_pins(&fpga.part_number);
        let mut leds = Vec::new();
        for (name, r) in board.resources.iter().filter(|(_, r)| usable(r)) {
            let (kind, active, io_standard) = match r {
                Resource::Leds {
                    pins,
                    active,
                    io_standard,
                    ..
                } => (
                    LedKind::Leds {
                        name: name.clone(),
                        pins: pins.clone(),
                    },
                    *active,
                    io_standard.clone(),
                ),
                Resource::RgbLed {
                    red,
                    green,
                    blue,
                    active,
                    io_standard,
                    ..
                } => (
                    LedKind::Rgb {
                        name: name.clone(),
                        pins: [red.clone(), green.clone(), blue.clone()],
                    },
                    *active,
                    io_standard.clone(),
                ),
                Resource::LedMatrix {
                    leds,
                    colors,
                    active,
                    io_standard,
                    ..
                } => (
                    LedKind::Matrix {
                        name: name.clone(),
                        leds: leds.clone(),
                        colors: [
                            colors.red.clone(),
                            colors.green.clone(),
                            colors.blue.clone(),
                        ],
                    },
                    *active,
                    io_standard.clone(),
                ),
                _ => continue,
            };
            let pins = match &kind {
                LedKind::Leds { pins, .. } => pins.clone(),
                LedKind::Rgb { pins, .. } => pins.to_vec(),
                LedKind::Matrix { .. } => Vec::new(),
            };
            let on_driver: Vec<Option<u8>> = pins
                .iter()
                .map(|p| rgb_pins.and_then(|rgb| rgb.iter().position(|q| q == p).map(|i| i as u8)))
                .collect();
            let mut active = active;
            let rgb_driver = if on_driver.len() == 3 && on_driver.iter().all(Option::is_some) {
                // The driver only sinks current: its LEDs turn on low,
                // whatever the catalog knows.
                match active {
                    Some(Level::High) => {
                        return Err(format!(
                            "{id}: {name} is on the RGB LED driver's pins, which turn LEDs on low"
                        ));
                    }
                    _ => active = Some(Level::Low),
                }
                Some(on_driver.into_iter().map(Option::unwrap).collect())
            } else {
                None
            };
            leds.push(LedGroup {
                kind,
                active,
                io_standard,
                rgb_driver,
                drives: r.drives().cloned().collect(),
            });
        }
        leds.sort_by(|a, b| (a.kind.order(), a.kind.name()).cmp(&(b.kind.order(), b.kind.name())));
        let drives = board
            .requires
            .iter()
            .filter_map(|q| match q {
                Requirement::Drive(d) => Some(d.clone()),
                Requirement::Setup { .. } => None,
            })
            .collect();
        Ok(Plan {
            board,
            arch,
            clock,
            leds,
            drives,
        })
    }

    /// Every drive: the board's, then those of the LEDs used.
    fn all_drives(&self) -> Vec<&Drive> {
        self.drives
            .iter()
            .chain(self.leds.iter().flat_map(|g| g.drives.iter()))
            .collect()
    }

    fn blinky(&self) -> Option<Generated> {
        let clock = self.clock.as_ref()?;
        let target = self.leds.first()?;
        let n = counter_width(clock.mhz());
        let mut files = BTreeMap::new();
        files.insert("main.v".into(), self.main_v(clock, n));
        files.insert("main_tb.v".into(), self.main_tb_v(clock));
        if let Some(layout) = self.gtkw(Some(clock), true) {
            files.insert("main_tb.gtkw".into(), layout);
        }
        files.insert(
            format!("pinout.{}", self.arch.extension()),
            self.pin_file("blinky", Some(clock)),
        );
        if let Clock::Internal(osc) = clock {
            let (file, _, text) = osc.file();
            files.insert(file.into(), text.into());
        }
        for g in &self.leds {
            if let Some(map) = &g.rgb_driver {
                files.insert("rgb_driver.v".into(), rgb_driver_v(map));
            }
        }
        files.insert(
            "logix-ide.toml".into(),
            format!(
                "# Logix IDE project settings.\nboard = \"{}\"\nsummary = \"{}\"\n",
                self.board_id_placeholder(),
                summary(target)
            ),
        );
        Some(Generated {
            name: "blinky".into(),
            files,
        })
    }

    fn bare_minimum(&self) -> Option<Generated> {
        let clock = match &self.clock {
            Some(c @ Clock::Pin { .. }) => Some(c),
            _ => None,
        };
        let mut files = BTreeMap::new();
        files.insert("main.v".into(), self.bare_main_v(clock));
        files.insert("main_tb.v".into(), self.bare_tb_v(clock));
        files.insert(
            "logix-ide.toml".into(),
            format!(
                "# Logix IDE project settings.\nboard = \"{}\"\nsummary = \"The board's pins and an empty design, to start from scratch\"\n",
                self.board_id_placeholder()
            ),
        );
        if let Some(layout) = self.gtkw(clock, false) {
            files.insert("main_tb.gtkw".into(), layout);
        }
        for g in &self.leds {
            if let Some(map) = &g.rgb_driver {
                files.insert("rgb_driver.v".into(), rgb_driver_v(map));
            }
        }
        files.insert(
            format!("pinout.{}", self.arch.extension()),
            self.pin_file("bare-minimum", clock),
        );
        Some(Generated {
            name: "bare-minimum".into(),
            files,
        })
    }

    /// `logix-ide.toml` names the board by ID; the caller replaces this.
    fn board_id_placeholder(&self) -> &'static str {
        BOARD_ID
    }

    // -- main.v ------------------------------------------------------------

    /// The top module's ports: (declaration, comment).
    fn ports(&self, clock: Option<&Clock>, blinky: bool) -> Vec<(String, String)> {
        let mut ports = Vec::new();
        if let Some(Clock::Pin { name, mhz, .. }) = clock {
            ports.push((
                format!("input        {name}"),
                format!("{} clock", mhz_text(*mhz)),
            ));
        }
        for (i, g) in self.leds.iter().enumerate() {
            let blinks = blinky && i == 0;
            let level = active_text(g.active);
            match &g.kind {
                LedKind::Leds { name, pins } => {
                    let what = if pins.len() == 1 {
                        if blinks {
                            "The blinking LED".to_string()
                        } else {
                            "An LED, off".to_string()
                        }
                    } else if blinks {
                        format!("{name}[{}] blinks, the rest are off", pins.len() - 1)
                    } else {
                        "Off".to_string()
                    };
                    ports.push((
                        format!("output {}{name}", width_decl(pins.len())),
                        format!("{what}. {level}"),
                    ));
                }
                LedKind::Rgb { name, .. } => ports.push((
                    format!("output [2:0] {name}"),
                    format!(
                        "RGB LED: [0] red, [1] green, [2] blue{}. {level}",
                        if blinks { "; red blinks" } else { ", off" }
                    ),
                )),
                LedKind::Matrix { name, leds, .. } => {
                    ports.push((
                        format!("output {}{name}", width_decl(leds.len())),
                        format!(
                            "{}. {level}",
                            if blinks {
                                format!("LED matrix: {name}[{}] blinks red", leds.len() - 1)
                            } else {
                                "LED matrix, off".into()
                            }
                        ),
                    ));
                    ports.push((
                        format!("output [2:0] {name}_color"),
                        format!("Its colors: [0] red, [1] green, [2] blue. {level}"),
                    ));
                }
            }
        }
        for d in self.all_drives() {
            ports.push((
                format!("output {}{}", width_decl(d.pins.len()), d.name),
                format!("Held {}, see below.", level_word(d.level)),
            ));
        }
        // Align the comments.
        let w = ports.iter().map(|(d, _)| d.len()).max().unwrap_or(0);
        let last = ports.len().saturating_sub(1);
        ports
            .into_iter()
            .enumerate()
            .map(|(i, (d, c))| {
                let d = if i == last { d } else { format!("{d},") };
                (format!("{d:<w$}", w = w + 1), c)
            })
            .collect()
    }

    fn drive_lines(&self, out: &mut String) {
        for d in self.all_drives() {
            let why = d
                .why
                .clone()
                .unwrap_or_else(|| "The board needs it.".into());
            let _ = writeln!(
                out,
                "  // {}",
                wrap_comment(&format!("{} {why}", held_text(d)), 2)
            );
            let _ = writeln!(
                out,
                "  assign {} = {};\n",
                d.name,
                level_literal(d.pins.len(), d.level)
            );
        }
    }

    fn main_v(&self, clock: &Clock, n: u32) -> String {
        let target = &self.leds[0];
        let mut s = String::new();
        let title = match target.kind {
            LedKind::Leds { .. } => "Blinking LED",
            LedKind::Rgb { .. } => "Blinking RGB LED",
            LedKind::Matrix { .. } => "Blinking LED matrix",
        };
        s += "//------------------------------------------------------------------\n";
        let _ = writeln!(s, "//-- {title}");
        s += "//------------------------------------------------------------------\n\n";
        if let Clock::Internal(_) = clock {
            let _ = writeln!(
                s,
                "// The clock comes from the FPGA's internal oscillator, so the design\n// needs no clock pin.\n"
            );
        }
        let _ = writeln!(
            s,
            "// N is the counter width. {} toggles every 2^(N-1) clocks.",
            blink_name(target)
        );
        let secs = 2f64.powi(n as i32 - 1) / (clock.mhz() * 1e6);
        let _ = writeln!(
            s,
            "// With the default N = {n} and a {} clock, it toggles about\n// every {secs:.1} seconds. The testbench uses a small N to run quickly.",
            mhz_text(clock.mhz())
        );
        let _ = writeln!(s, "module main #(\n    parameter integer N = {n}\n) (");
        for (d, c) in self.ports(Some(clock), true) {
            let _ = writeln!(s, "    {d} // {c}");
        }
        s += ");\n\n";
        self.drive_lines(&mut s);
        if let Clock::Internal(osc) = clock {
            let (_, module, _) = osc.file();
            let _ = writeln!(
                s,
                "  // The {} clock from the internal oscillator.\n  wire sys_clk;\n  {module} {module} (.clk(sys_clk));\n",
                mhz_text(osc.mhz())
            );
        }
        let clk = clock_name(clock);
        s += "  // The blink counter. It starts at 0 when the FPGA is configured.\n";
        s += "  reg [N-1:0] counter = 0;\n\n";
        s += "  // Count up on every clock.\n";
        let _ = writeln!(s, "  always @(posedge {clk}) counter <= counter + 1;\n");
        for (i, g) in self.leds.iter().enumerate() {
            let on = if i == 0 { "counter[N-1]" } else { "1'b0" };
            led_assigns(&mut s, g, i == 0, on);
        }
        s += "endmodule\n";
        s
    }

    fn bare_main_v(&self, clock: Option<&Clock>) -> String {
        let mut s = String::new();
        let ports = self.ports(clock, false);
        if ports.is_empty() {
            s += "// The top module must be named 'main'. No pins are known for this\n";
            s += "// board yet: add ports here, and a line for each in the pin file.\n\n";
            s += "module main;\n\n  // Your design goes here.\n\nendmodule\n";
            return s;
        }
        s += "// The top module must be named 'main'. Its ports are the board's\n";
        s += "// pins in the pin file. The outputs start off; your design goes here.\n\n";
        s += "module main (\n";
        for (d, c) in self.ports(clock, false) {
            let _ = writeln!(s, "    {d} // {c}");
        }
        s += ");\n\n";
        self.drive_lines(&mut s);
        for g in &self.leds {
            led_assigns(&mut s, g, false, "1'b0");
        }
        s += "  // Your design goes here.\n\nendmodule\n";
        s
    }

    // -- main_tb.v -----------------------------------------------------------

    /// The design's outputs, as (port, width): the LEDs', then the drives'.
    fn outputs(&self) -> Vec<(String, usize)> {
        let mut outs = Vec::new();
        for g in &self.leds {
            match &g.kind {
                LedKind::Leds { name, pins } => outs.push((name.clone(), pins.len())),
                LedKind::Rgb { name, .. } => outs.push((name.clone(), 3)),
                LedKind::Matrix { name, leds, .. } => {
                    outs.push((name.clone(), leds.len()));
                    outs.push((format!("{name}_color"), 3));
                }
            }
        }
        for d in self.all_drives() {
            outs.push((d.name.clone(), d.pins.len()));
        }
        outs
    }

    /// Bare Minimum's testbench: the outputs off and the drives at their
    /// levels, then 32 clocks (or 100ns without a clock), with a place for
    /// the user's own checks.
    fn bare_tb_v(&self, clock: Option<&Clock>) -> String {
        let mut s = String::from(TB_HEAD);
        if let Some(Clock::Pin { name, .. }) = clock {
            let _ = writeln!(
                s,
                "  // Clock.\n  reg {name} = 0;\n  always #5 {name} = ~{name};\n\n  // Number of {name} rising edges so far.\n  integer clk_num = 0;\n  always @(posedge {name}) clk_num <= clk_num + 1;\n"
            );
        }
        let outs = self.outputs();
        if !outs.is_empty() {
            s += "  // The design's outputs.\n";
            for (name, w) in &outs {
                let _ = writeln!(s, "  wire {}{name};", width_decl(*w));
            }
            s.push('\n');
        }
        let mut conns: Vec<String> = Vec::new();
        if let Some(Clock::Pin { name, .. }) = clock {
            conns.push(format!("      .{name}({name})"));
        }
        for (name, _) in &outs {
            conns.push(format!("      .{name}({name})"));
        }
        if conns.is_empty() {
            s += "  // The design.\n  main dut ();\n\n";
        } else {
            s += "  // The design.\n  main dut (\n";
            s += &conns.join(",\n");
            s += "\n  );\n\n";
        }
        s += "  // The test sequence.\n  initial begin\n    $dumpvars(0, main_tb);\n\n";
        let mut checks = String::new();
        for g in &self.leds {
            for (port, value) in led_values(g, false, "1'b0") {
                let _ = writeln!(checks, "    `EXPECT_EQ({port}, {value})");
            }
        }
        for d in self.all_drives() {
            let _ = writeln!(
                checks,
                "    `EXPECT_EQ({}, {})",
                d.name,
                level_literal(d.pins.len(), d.level)
            );
        }
        if !checks.is_empty() {
            s += "    // At the start, the LEDs are off and the held pins at their levels.\n    #1;\n";
            s += &checks;
            s.push('\n');
        }
        match clock {
            Some(c) => {
                let clk = clock_name(c);
                let _ = writeln!(
                    s,
                    "    // Add your own checks here. This runs 32 clocks.\n    repeat (32) @(posedge {clk});"
                );
            }
            None => s += "    // Add your own checks here.\n    #100;\n",
        }
        s += "\n    $display(\"End of simulation\");\n    $finish;\n  end\n\nendmodule\n";
        s
    }

    /// A GTKWave layout for main_tb.v, in the hand-written examples' style:
    /// the clock count and clock, the counter (blinky), the LEDs (the
    /// blinking group highlighted), then the held pins. None when there is
    /// nothing to show.
    fn gtkw(&self, clock: Option<&Clock>, blinky: bool) -> Option<String> {
        let mut body = String::new();
        if let Some(c) = clock {
            let _ = writeln!(
                body,
                "@420\nmain_tb.clk_num\n@28\nmain_tb.{}",
                clock_name(c)
            );
        }
        if blinky {
            body += "@200\n-\n-Counter\n@24\nmain_tb.dut.counter[3:0]\n";
        }
        let signal = |name: &str, w: usize| {
            if w == 1 {
                format!("main_tb.{name}")
            } else {
                format!("main_tb.{name}[{}:0]", w - 1)
            }
        };
        // Single bits and RGB LEDs in binary, wider LED vectors in hex.
        let format = |w: usize| if w <= 3 { "@28" } else { "@22" };
        for (i, g) in self.leds.iter().enumerate() {
            let title = match &g.kind {
                LedKind::Leds { pins, .. } if pins.len() == 1 => "LED",
                LedKind::Leds { .. } => "LEDs",
                LedKind::Rgb { .. } => "RGB LED",
                LedKind::Matrix { .. } => "LED matrix",
            };
            let _ = writeln!(body, "@200\n-\n-{title}");
            let ports: Vec<(String, usize)> = match &g.kind {
                LedKind::Leds { name, pins } => vec![(name.clone(), pins.len())],
                LedKind::Rgb { name, .. } => vec![(name.clone(), 3)],
                LedKind::Matrix { name, leds, .. } => {
                    vec![(name.clone(), leds.len()), (format!("{name}_color"), 3)]
                }
            };
            for (name, w) in ports {
                let _ = writeln!(body, "{}", format(w));
                if blinky && i == 0 {
                    body += "[color] 2\n";
                }
                let _ = writeln!(body, "{}", signal(&name, w));
            }
        }
        let drives = self.all_drives();
        if !drives.is_empty() {
            body += "@200\n-\n-Held\n";
            for d in drives {
                let _ = writeln!(
                    body,
                    "{}\n{}",
                    format(d.pins.len()),
                    signal(&d.name, d.pins.len())
                );
            }
        }
        if body.is_empty() {
            return None;
        }
        Some(format!(
            "{GTKW_HEAD}{body}[pattern_trace] 1\n[pattern_trace] 0\n"
        ))
    }

    fn main_tb_v(&self, clock: &Clock) -> String {
        let mut s = String::from(TB_HEAD);
        match clock {
            Clock::Pin { name, .. } => {
                let _ = writeln!(
                    s,
                    "  // Clock.\n  reg {name} = 0;\n  always #5 {name} = ~{name};\n"
                );
            }
            Clock::Internal(osc) => {
                let (file, _, _) = osc.file();
                let _ = writeln!(
                    s,
                    "  // Clock. The design makes its own clock with the oscillator model in\n  // {file}, which has a 10ns period.\n  wire sys_clk = dut.sys_clk;\n"
                );
            }
        }
        let clk = clock_name(clock);
        let _ = writeln!(
            s,
            "  // Number of {clk} rising edges so far.\n  integer clk_num = 0;\n  always @(posedge {clk}) clk_num <= clk_num + 1;\n"
        );
        let outs = self.outputs();
        s += "  // The design's outputs.\n";
        for (name, w) in &outs {
            let _ = writeln!(s, "  wire {}{name};", width_decl(*w));
        }
        s += "\n  // A 4 bit counter, so the LED toggles every 8 clocks.\n";
        s += "  main #(\n      .N(4)\n  ) dut (\n";
        let mut conns: Vec<String> = Vec::new();
        if let Clock::Pin { name, .. } = clock {
            conns.push(format!("      .{name}({name})"));
        }
        for (name, _) in &outs {
            conns.push(format!("      .{name}({name})"));
        }
        s += &conns.join(",\n");
        s += "\n  );\n\n";
        s += "  // Loop index for the LED checks.\n  integer i;\n\n";
        s += "  // The test sequence.\n  initial begin\n    $dumpvars(0, main_tb);\n\n";
        s += "    // Before the first clock, the counter starts at 0 and the LEDs\n    // are off.\n    #1;\n";
        s += "    `EXPECT_EQ(dut.counter, 4'd0)\n";
        let expect = |s: &mut String, on: &str| {
            for (i, g) in self.leds.iter().enumerate() {
                for (port, value) in led_values(g, i == 0, on) {
                    let _ = writeln!(s, "    `EXPECT_EQ({port}, {value})");
                }
            }
        };
        expect(&mut s, "1'b0");
        for d in self.all_drives() {
            let _ = writeln!(
                s,
                "    `EXPECT_EQ({}, {})",
                d.name,
                level_literal(d.pins.len(), d.level)
            );
        }
        let _ = writeln!(
            s,
            "\n    // After i clocks the counter equals i, so {} shows bit 3\n    // of i. The other LEDs stay off.",
            blink_name(&self.leds[0])
        );
        let _ = writeln!(
            s,
            "    for (i = 1; i <= 32; i = i + 1) begin\n      @(posedge {clk});\n      #1;"
        );
        let mut inner = String::new();
        expect(&mut inner, "i[3]");
        for line in inner.lines() {
            let _ = writeln!(s, "  {line}");
        }
        s += "    end\n\n    $display(\"End of simulation\");\n    $finish;\n  end\n\nendmodule\n";
        s
    }

    // -- The pin file ----------------------------------------------------------

    fn pin_file(&self, example: &str, clock: Option<&Clock>) -> String {
        let c = self.arch.comment();
        let mut s = String::new();
        let _ = writeln!(
            s,
            "{c} Constraint file of the {example} example for the {} board.\n{c} It assigns the design's signals to FPGA pins, and lists only\n{c} the pins that this example uses.",
            self.board.name
        );
        if let Some(Clock::Internal(osc)) = clock {
            let _ = writeln!(
                s,
                "{c} The design uses the FPGA's internal {} oscillator, so there is\n{c} no clock pin.",
                mhz_text(osc.mhz())
            );
        }
        if let Some(Clock::Pin {
            name,
            pin,
            mhz,
            io_standard,
        }) = clock
        {
            let _ = writeln!(s, "\n{c} Clock ({}).", mhz_text(*mhz));
            self.arch.pin(&mut s, name, pin, io_standard.as_deref());
            match self.arch {
                Arch::Ecp5 => {
                    let _ = writeln!(s, "FREQUENCY PORT \"{name}\" {} MHZ;", num(*mhz));
                }
                Arch::Xilinx => {
                    let _ = writeln!(
                        s,
                        "create_clock -period {:.3} [get_ports {{{name}}}]",
                        1000.0 / mhz
                    );
                }
                _ => {}
            }
        }
        for g in &self.leds {
            let level = active_text(g.active).to_lowercase();
            let level = level.trim_end_matches('.');
            let io = g.io_standard.as_deref();
            match &g.kind {
                LedKind::Leds { name, pins } => {
                    let what = if pins.len() == 1 { "LED" } else { "LEDs" };
                    let _ = writeln!(s, "\n{c} {what}, {level}.");
                    if g.rgb_driver.is_some() {
                        let _ = writeln!(s, "{c} On the FPGA's dedicated RGB LED driver pins.");
                    }
                    self.vector(&mut s, name, pins, io);
                }
                LedKind::Rgb { name, pins } => {
                    let _ = writeln!(s, "\n{c} RGB LED, {level}: [0] red, [1] green, [2] blue.");
                    if g.rgb_driver.is_some() {
                        let _ = writeln!(s, "{c} On the FPGA's dedicated RGB LED driver pins.");
                    }
                    self.vector(&mut s, name, pins, io);
                }
                LedKind::Matrix { name, leds, colors } => {
                    let _ = writeln!(s, "\n{c} LED matrix, {level}: an LED lights when its pin and a color's are on.");
                    self.vector(&mut s, name, leds, io);
                    let _ = writeln!(
                        s,
                        "\n{c} The matrix's colors: [0] red, [1] green, [2] blue."
                    );
                    self.vector(&mut s, &format!("{name}_color"), colors, io);
                }
            }
        }
        for d in self.all_drives() {
            let _ = writeln!(s, "\n{c} {}", held_text(d));
            self.vector(&mut s, &d.name, &d.pins, d.io_standard.as_deref());
        }
        if clock.is_none() && self.leds.is_empty() && self.all_drives().is_empty() {
            let _ = writeln!(
                s,
                "\n{c} No pins are known for this board yet. Add a line for each port of\n{c} main, such as:\n{c}"
            );
            let mut example = String::new();
            self.arch.pin(
                &mut example,
                "led",
                "<pin>",
                (self.arch != Arch::Ice40).then_some("LVCMOS33"),
            );
            for line in example.lines() {
                let _ = writeln!(s, "{c}   {line}");
            }
        }
        s
    }

    fn vector(&self, s: &mut String, name: &str, pins: &[String], io: Option<&str>) {
        if pins.len() == 1 {
            self.arch.pin(s, name, &pins[0], io);
        } else {
            for (i, p) in pins.iter().enumerate() {
                self.arch.pin(s, &format!("{name}[{i}]"), p, io);
            }
        }
    }
}

impl Arch {
    fn extension(self) -> &'static str {
        match self {
            Arch::Ice40 => "pcf",
            Arch::Ecp5 => "lpf",
            Arch::Gowin => "cst",
            Arch::Xilinx => "xdc",
        }
    }

    fn comment(self) -> &'static str {
        match self {
            Arch::Gowin => "//",
            _ => "#",
        }
    }

    /// One pin's lines.
    fn pin(self, s: &mut String, port: &str, pin: &str, io: Option<&str>) {
        let _ = match self {
            Arch::Ice40 => writeln!(s, "set_io {port} {pin}"),
            Arch::Ecp5 => {
                let _ = writeln!(s, "LOCATE COMP \"{port}\" SITE \"{pin}\";");
                match io {
                    Some(io) => writeln!(s, "IOBUF  PORT \"{port}\" IO_TYPE={io};"),
                    None => Ok(()),
                }
            }
            Arch::Gowin => {
                let _ = writeln!(s, "IO_LOC  \"{port}\" {pin};");
                match io {
                    Some(io) => writeln!(s, "IO_PORT \"{port}\" IO_TYPE={io};"),
                    None => Ok(()),
                }
            }
            Arch::Xilinx => match io {
                Some(io) => writeln!(
                    s,
                    "set_property -dict {{ PACKAGE_PIN {pin:<5} IOSTANDARD {io} }} [get_ports {{{port}}}]"
                ),
                None => writeln!(s, "set_property PACKAGE_PIN {pin} [get_ports {{{port}}}]"),
            },
        };
    }
}

// ---------------------------------------------------------------------------
// Helpers.

/// The catalog the generated examples came from, as far as the generator
/// reads it (`generate_examples` writes it), for the test that checks
/// `examples/` is up to date.
#[cfg(test)]
const SNAPSHOT_FILE: &str = "generated-examples-catalog.json";
#[cfg(test)]
const CATALOG_SNAPSHOT: &str = include_str!("../generated-examples-catalog.json");

/// Replaced with the board's ID by the caller (`with_board_id`).
const BOARD_ID: &str = "@BOARD_ID@";

/// The generated files with the board's ID in `logix-ide.toml`.
pub fn with_board_id(mut g: Generated, id: &str) -> Generated {
    for text in g.files.values_mut() {
        *text = text.replace(BOARD_ID, id);
    }
    g
}

/// The counter width that makes the LED toggle about every 0.7 seconds:
/// 2^(N-1) clocks.
fn counter_width(mhz: f64) -> u32 {
    let bits = (mhz * 1e6 * 0.7).log2().round() as u32;
    bits.max(3) + 1
}

fn num(x: f64) -> String {
    if x.fract() == 0.0 {
        format!("{}", x as i64)
    } else {
        let s = format!("{x:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn mhz_text(mhz: f64) -> String {
    if mhz < 10.0 && mhz.fract() != 0.0 {
        format!("{:.1}MHz", mhz)
    } else {
        format!("{}MHz", num(mhz))
    }
}

fn width_decl(w: usize) -> String {
    if w == 1 {
        "      ".into()
    } else {
        format!("{:<6}", format!("[{}:0]", w - 1))
    }
}

fn active_text(a: Option<Level>) -> &'static str {
    match a {
        Some(Level::High) => "Active high.",
        Some(Level::Low) => "Active low.",
        None => "Active level unknown, taken as high.",
    }
}

fn level_word(l: Level) -> &'static str {
    match l {
        Level::High => "high",
        Level::Low => "low",
    }
}

fn level_literal(w: usize, l: Level) -> String {
    match l {
        Level::High if w == 1 => "1'b1".into(),
        Level::High => format!("{w}'b{}", "1".repeat(w)),
        Level::Low => format!("{w}'b0"),
    }
}

fn held_text(d: &Drive) -> String {
    format!("Held {}: {}.", level_word(d.level), d.name)
}

fn wrap_comment(text: &str, indent: usize) -> String {
    let mut out = String::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && indent + 3 + line.len() + 1 + word.len() > 72 {
            out += &line;
            out += &format!("\n{}// ", " ".repeat(indent));
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line += word;
    }
    out + &line
}

fn clock_name(c: &Clock) -> String {
    match c {
        Clock::Pin { name, .. } => name.clone(),
        Clock::Internal(_) => "sys_clk".into(),
    }
}

fn blink_name(g: &LedGroup) -> String {
    match &g.kind {
        LedKind::Leds { name, pins } if pins.len() == 1 => name.clone(),
        LedKind::Leds { name, pins } => format!("{name}[{}]", pins.len() - 1),
        LedKind::Rgb { .. } => "The red LED".into(),
        LedKind::Matrix { name, leds, .. } => format!("{name}[{}]", leds.len() - 1),
    }
}

fn summary(g: &LedGroup) -> &'static str {
    match g.kind {
        LedKind::Leds { .. } => "Blinks an LED",
        LedKind::Rgb { .. } => "Blinks the RGB LED in red",
        LedKind::Matrix { .. } => "Blinks an LED of the LED matrix in red",
    }
}

/// The value of each of a group's ports, as Verilog, when `on` (a 1-bit
/// expression) says whether the blinking LED is on: (port, value).
fn led_values(g: &LedGroup, blinks: bool, on: &str) -> Vec<(String, String)> {
    let high = g.active != Some(Level::Low);
    let off_bit = if high { "0" } else { "1" };
    let lit = |w: usize, bit: &str| format!("{w}'b{}", bit.repeat(w));
    let pin_on = |on: &str| {
        if high {
            on.to_string()
        } else if on == "1'b0" {
            "1'b1".into()
        } else {
            format!("~{on}")
        }
    };
    // A vector whose top bit (or bit 0 when `low_bit`) follows `on`.
    let vector = |w: usize, low_bit: bool| -> String {
        if !blinks {
            return lit(w, off_bit);
        }
        if w == 1 {
            return pin_on(on);
        }
        if low_bit {
            format!("{{{}, {}}}", lit(w - 1, off_bit), pin_on(on))
        } else {
            format!("{{{}, {}}}", pin_on(on), lit(w - 1, off_bit))
        }
    };
    match &g.kind {
        LedKind::Leds { name, pins } => vec![(name.clone(), vector(pins.len(), false))],
        LedKind::Rgb { name, .. } => vec![(name.clone(), vector(3, true))],
        LedKind::Matrix { name, leds, .. } => {
            let color = if blinks {
                // Red on, green and blue off.
                format!(
                    "{{{}, {}}}",
                    lit(2, off_bit),
                    if high { "1'b1" } else { "1'b0" }
                )
            } else {
                lit(3, off_bit)
            };
            vec![
                (name.clone(), vector(leds.len(), false)),
                (format!("{name}_color"), color),
            ]
        }
    }
}

fn led_assigns(s: &mut String, g: &LedGroup, blinks: bool, on: &str) {
    let values = led_values(g, blinks, on);
    let what = match &g.kind {
        LedKind::Leds { .. } if blinks => {
            format!("{} follows the counter's top bit.", blink_name(g))
        }
        LedKind::Rgb { .. } if blinks => {
            "Red follows the counter's top bit, green and blue stay off.".into()
        }
        LedKind::Matrix { .. } if blinks => {
            format!("{} follows the counter's top bit, in red.", blink_name(g))
        }
        _ => format!("{} stays off.", g.kind.name()),
    };
    let more = match &g.kind {
        LedKind::Leds { pins, .. } if blinks && pins.len() > 1 => " The other LEDs are off.",
        _ => "",
    };
    let _ = writeln!(s, "  // {what}{more}");
    if g.rgb_driver.is_some() {
        // Through the RGB LED driver: `on` is 1 for a lit color, in the
        // port's bit order.
        let (port, _) = &values[0];
        let w = 3;
        let on_bits = if blinks {
            match &g.kind {
                LedKind::Rgb { .. } => format!("{{2'b00, {on}}}"),
                _ => format!("{{{on}, 2'b00}}"),
            }
        } else {
            format!("{w}'b000")
        };
        let _ = writeln!(
            s,
            "  rgb_driver rgb_driver (\n      .on  ({on_bits}),\n      .pins({port})\n  );\n"
        );
        return;
    }
    for (port, value) in values {
        let _ = writeln!(s, "  assign {port} = {value};");
    }
    s.push('\n');
}

fn rgb_driver_v(map: &[u8]) -> String {
    // map[i] is the driver output (RGB0..2) of the port's bit i.
    let mut pwm = [0usize; 3];
    for (bit, out) in map.iter().enumerate() {
        pwm[*out as usize] = bit;
    }
    format!(
        "{RGB_HEAD}  SB_RGBA_DRV #(
      .CURRENT_MODE(\"0b1\"),
      .RGB0_CURRENT(\"0b000001\"),
      .RGB1_CURRENT(\"0b000001\"),
      .RGB2_CURRENT(\"0b000001\")
  ) driver (
      .CURREN  (1'b1),
      .RGBLEDEN(1'b1),
      .RGB0PWM (on[{a}]),
      .RGB1PWM (on[{b}]),
      .RGB2PWM (on[{c}]),
      .RGB0    (pins[{a}]),
      .RGB1    (pins[{b}]),
      .RGB2    (pins[{c}])
  );
{RGB_TAIL}",
        a = pwm[0],
        b = pwm[1],
        c = pwm[2]
    )
}

/// The hand-written examples' layout header: window, zoom and panes.
const GTKW_HEAD: &str = "[*]
[*] GTKWave save file for main_tb.v, used by Simulate.
[*]
[timestart] 0
[size] 1000 600
[pos] -1 -1
*-16.786717 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1 -1
[treeopen] main_tb.
[sst_width] 253
[signals_width] 160
[sst_expanded] 1
[sst_vpaned_height] 158
";

const TB_HEAD: &str = "// Testbench for main.v.
// Run it in batch mode with Run Tests, or view the signals
// in GTKWave with Simulate.

// Checks that 'actual_val' equals 'expected_val'. On a mismatch, reports
// an error with both values. Run Tests stops at the first error; Simulate
// runs to the end so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error(\"expected 'h%h, actual 'h%h\", (expected_val), (actual_val));

`timescale 1ns / 1ps

module main_tb;

";

const RGB_HEAD: &str = "// Drives the RGB LED through the FPGA's RGB LED driver (SB_RGBA_DRV).
// SB_RGBA_DRV can't be simulated, so simulations use a simple model
// instead.

module rgb_driver (
    input  [2:0] on,   // 1 turns an LED on, in the order of pins.
    output [2:0] pins  // The LED pins, active low.
);

`ifdef SYNTHESIS

  // The real driver, set to its lowest current.
";

const RGB_TAIL: &str = "
`else

  // Simulation model: a pin goes low when its LED is on.
  assign pins = ~on;

`endif

endmodule
";

const HF_OSC_V: &str = "// The FPGA's internal 48MHz oscillator (SB_HFOSC). SB_HFOSC can't be
// simulated, so simulations use a simple model instead.

`timescale 1ns / 1ps

module hf_osc (
    output clk  // The 48MHz clock.
);

`ifdef SYNTHESIS

  // The real oscillator. CLKHF_DIV \"0b00\" selects 48MHz.
  SB_HFOSC #(
      .CLKHF_DIV(\"0b00\")
  ) osc (
      .CLKHFPU(1'b1),
      .CLKHFEN(1'b1),
      .CLKHF  (clk),

      // Unused.
      .TRIM0(),
      .TRIM1(),
      .TRIM2(),
      .TRIM3(),
      .TRIM4(),
      .TRIM5(),
      .TRIM6(),
      .TRIM7(),
      .TRIM8(),
      .TRIM9()
  );

`else

  // Simulation model: a clock with a 10ns period, the same as in the
  // other testbenches.
  reg sim_clk = 0;
  always #5 sim_clk = ~sim_clk;
  assign clk = sim_clk;

`endif

endmodule
";

const OSC_V: &str = "// The ECP5's internal oscillator (OSCG), about 2.4MHz. OSCG can't be
// simulated, so simulations use a simple model instead.

`timescale 1ns / 1ps

module osc (
    output clk  // About 2.4MHz.
);

`ifdef SYNTHESIS

  // The real oscillator. It runs at about 310MHz / DIV.
  OSCG #(.DIV(128)) oscg (.OSC(clk));

`else

  // Simulation model: a clock with a 10ns period, the same as in the
  // other testbenches.
  reg sim_clk = 0;
  always #5 sim_clk = ~sim_clk;
  assign clk = sim_clk;

`endif

endmodule
";

#[cfg(test)]
mod tests {
    use super::*;

    fn board(v: serde_json::Value) -> Board {
        serde_json::from_value(v).unwrap()
    }

    fn fpga(arch: &str, part: &str) -> Fpga {
        Fpga {
            architecture: arch.into(),
            part_number: part.into(),
        }
    }

    fn files(id: &str, b: &Board, f: &Fpga) -> BTreeMap<String, BTreeMap<String, String>> {
        generate(id, b, f)
            .unwrap()
            .into_iter()
            .map(|g| {
                let g = with_board_id(g, id);
                (g.name, g.files)
            })
            .collect()
    }

    #[test]
    fn counter_widths_match_the_examples() {
        // As the hand-written examples chose them: about 0.7 s.
        assert_eq!(counter_width(12.0), 24);
        assert_eq!(counter_width(25.0), 25);
        assert_eq!(counter_width(27.0), 25);
        assert_eq!(counter_width(48.0), 26);
        assert_eq!(counter_width(100.0), 27);
        assert_eq!(counter_width(310.0 / 128.0), 22);
    }

    #[test]
    fn an_ice40_board_with_leds() {
        let b = board(serde_json::json!({
            "name": "Alhambra II", "fpga_id": "x",
            "resources": {
                "leds": {"type": "leds", "pins": ["45", "44", "43", "42", "41", "39", "38", "37"], "active": "high"},
                "sys_clk": {"type": "clock", "pin": "49", "mhz": 12},
                "btn": {"type": "buttons", "pins": ["34"]}
            }
        }));
        let all = files("alhambra-ii", &b, &fpga("ice40", "ICE40HX4K-TQ144"));
        let blinky = &all["blinky"];
        assert_eq!(
            blinky.keys().collect::<Vec<_>>(),
            [
                "logix-ide.toml",
                "main.v",
                "main_tb.gtkw",
                "main_tb.v",
                "pinout.pcf"
            ]
        );
        let layout = &blinky["main_tb.gtkw"];
        assert!(
            layout.contains("@420\nmain_tb.clk_num\n@28\nmain_tb.sys_clk\n@200\n-\n-Counter\n@24\nmain_tb.dut.counter[3:0]\n@200\n-\n-LEDs\n@22\n[color] 2\nmain_tb.leds[7:0]\n"),
            "{layout}"
        );
        let main = &blinky["main.v"];
        assert!(main.contains("parameter integer N = 24"), "{main}");
        assert!(main.contains("input        sys_clk,"), "{main}");
        assert!(main.contains("output [7:0] leds "), "{main}");
        assert!(
            main.contains("assign leds = {counter[N-1], 7'b0000000};"),
            "{main}"
        );
        let pcf = &blinky["pinout.pcf"];
        assert!(pcf.contains("set_io sys_clk 49\n"), "{pcf}");
        assert!(pcf.contains("set_io leds[7] 37\n"), "{pcf}");
        assert!(blinky["logix-ide.toml"].contains("board = \"alhambra-ii\""));
        let tb = &blinky["main_tb.v"];
        assert!(tb.contains("`EXPECT_EQ(leds, {i[3], 7'b0000000})"), "{tb}");
        // Bare Minimum: the same ports, the LEDs off, with a testbench that
        // checks it and a layout.
        let tb = &all["bare-minimum"]["main_tb.v"];
        assert!(tb.contains("`EXPECT_EQ(leds, 8'b00000000)"), "{tb}");
        assert!(tb.contains("repeat (32) @(posedge sys_clk);"), "{tb}");
        assert!(all["bare-minimum"]["main_tb.gtkw"].contains("-LEDs\n@22\nmain_tb.leds[7:0]\n"));
        let bare = &all["bare-minimum"]["main.v"];
        assert!(bare.contains("assign leds = 8'b00000000;"), "{bare}");
        assert!(!bare.contains("counter"), "{bare}");
    }

    #[test]
    fn active_low_leds_and_drives_on_ecp5() {
        let b = board(serde_json::json!({
            "name": "ULX3S", "fpga_id": "x",
            "requires": [{"type": "drive", "name": "wifi_gpio0", "pins": ["L2"], "level": "high",
                          "io_standard": "LVCMOS33", "why": "Keeps the ESP32 from resetting the board."}],
            "resources": {
                "sys_clk": {"type": "clock", "pin": "G2", "mhz": 25, "io_standard": "LVCMOS33"},
                "led": {"type": "leds", "pins": ["B2"], "active": "low", "io_standard": "LVCMOS33"}
            }
        }));
        let all = files("b", &b, &fpga("ecp5", "LFE5U-85F-6BG381C"));
        let main = &all["blinky"]["main.v"];
        assert!(main.contains("assign led = ~counter[N-1];"), "{main}");
        assert!(main.contains("assign wifi_gpio0 = 1'b1;"), "{main}");
        assert!(main.contains("Keeps the ESP32"), "{main}");
        let lpf = &all["blinky"]["pinout.lpf"];
        assert!(lpf.contains("LOCATE COMP \"sys_clk\" SITE \"G2\";\nIOBUF  PORT \"sys_clk\" IO_TYPE=LVCMOS33;\nFREQUENCY PORT \"sys_clk\" 25 MHZ;"), "{lpf}");
        assert!(
            lpf.contains("LOCATE COMP \"wifi_gpio0\" SITE \"L2\";"),
            "{lpf}"
        );
        let tb = &all["blinky"]["main_tb.v"];
        assert!(tb.contains("`EXPECT_EQ(led, 1'b1)"), "{tb}");
        assert!(tb.contains("`EXPECT_EQ(led, ~i[3])"), "{tb}");
        assert!(tb.contains("`EXPECT_EQ(wifi_gpio0, 1'b1)"), "{tb}");
        // Bare Minimum keeps the drive and turns the LED off.
        let bare = &all["bare-minimum"]["main.v"];
        assert!(bare.contains("assign led = 1'b1;"), "{bare}");
        assert!(bare.contains("assign wifi_gpio0 = 1'b1;"), "{bare}");
    }

    #[test]
    fn a_clock_that_needs_a_jumper_gives_way_to_the_oscillator() {
        let b = board(serde_json::json!({
            "name": "UPduino v3.0", "fpga_id": "x",
            "resources": {
                "sys_clk": {"type": "clock", "pin": "20", "mhz": 12,
                            "requires": [{"type": "setup", "text": "Short R16."}]},
                "rgb_led": {"type": "rgb_led", "red": "41", "green": "39", "blue": "40", "active": "low"}
            }
        }));
        let all = files("upduino3", &b, &fpga("ice40", "ICE40UP5K-SG48"));
        let blinky = &all["blinky"];
        assert!(blinky.contains_key("hf_osc.v"));
        assert!(blinky.contains_key("rgb_driver.v"));
        assert!(!blinky["pinout.pcf"].contains("sys_clk"));
        let main = &blinky["main.v"];
        assert!(main.contains("hf_osc hf_osc (.clk(sys_clk));"), "{main}");
        assert!(main.contains(".on  ({2'b00, counter[N-1]}),"), "{main}");
        // Red (bit 0) is on pin 41, the driver's RGB2.
        let drv = &blinky["rgb_driver.v"];
        assert!(drv.contains(".RGB2PWM (on[0]),"), "{drv}");
        assert!(drv.contains(".RGB0    (pins[1]),"), "{drv}");
        let tb = &blinky["main_tb.v"];
        assert!(tb.contains("wire sys_clk = dut.sys_clk;"), "{tb}");
        assert!(tb.contains("`EXPECT_EQ(rgb_led, {2'b11, ~i[3]})"), "{tb}");
        // Bare Minimum has no clock pin to give, and its LED goes through
        // the driver too.
        assert!(!all["bare-minimum"]["main.v"].contains("sys_clk"));
        assert!(all["bare-minimum"].contains_key("rgb_driver.v"));
    }

    #[test]
    fn choices_never_depend_on_order() {
        // Two LED resources: `led` blinks, `pmod_leds` stays off.
        let b = board(serde_json::json!({
            "name": "iCEBreaker", "fpga_id": "x",
            "resources": {
                "pmod_leds": {"type": "leds", "pins": ["1", "2"], "active": "high"},
                "sys_clk": {"type": "clock", "pin": "35", "mhz": 12},
                "led": {"type": "leds", "pins": ["11"], "active": "low"}
            }
        }));
        let main = &files("icebreaker", &b, &fpga("ice40", "ICE40UP5K-SG48"))["blinky"]["main.v"];
        assert!(main.contains("assign led = ~counter[N-1];"), "{main}");
        assert!(main.contains("assign pmod_leds = 2'b00;"), "{main}");
    }

    #[test]
    fn xilinx_pin_files_constrain_the_clock() {
        let b = board(serde_json::json!({
            "name": "Arty A7-35T", "fpga_id": "x",
            "resources": {
                "sys_clk": {"type": "clock", "pin": "E3", "mhz": 100, "io_standard": "LVCMOS33"},
                "leds": {"type": "leds", "pins": ["H5", "J5"], "active": "high", "io_standard": "LVCMOS33"}
            }
        }));
        let xdc = &files("arty", &b, &fpga("xilinx", "XC7A35TICSG324-1L"))["blinky"]["pinout.xdc"];
        assert!(xdc.contains("set_property -dict { PACKAGE_PIN E3    IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]\ncreate_clock -period 10.000 [get_ports {sys_clk}]"), "{xdc}");
        assert!(xdc.contains("[get_ports {leds[1]}]"), "{xdc}");
    }

    #[test]
    fn boards_without_leds_or_pins() {
        // A clock only: Bare Minimum with the clock, no blinky.
        let b = board(
            serde_json::json!({"name": "Lite", "fpga_id": "x", "resources": {
            "sys_clk": {"type": "clock", "pin": "H11", "mhz": 27, "io_standard": "LVCMOS33"}}}),
        );
        let all = files("lite", &b, &fpga("gowin", "GW2A-LV18PG256C8/I7"));
        assert_eq!(all.keys().collect::<Vec<_>>(), ["bare-minimum"]);
        assert!(all["bare-minimum"]["pinout.cst"].contains("IO_LOC  \"sys_clk\" H11;"));
        // Nothing known: an empty Bare Minimum, with how to add pins.
        let b = board(serde_json::json!({"name": "Soan", "fpga_id": "x"}));
        let all = files("soan", &b, &fpga("ice40", "ICE40UP5K-SG48"));
        assert_eq!(all.keys().collect::<Vec<_>>(), ["bare-minimum"]);
        assert!(all["bare-minimum"]["main.v"].contains("module main;\n"));
        assert!(all["bare-minimum"]["main_tb.v"].contains("  main dut ();\n"));
        assert!(!all["bare-minimum"].contains_key("main_tb.gtkw"));
        assert!(all["bare-minimum"]["pinout.pcf"].contains("#   set_io led <pin>\n"));
    }

    /// The examples folder.
    fn examples_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples")
    }

    /// Every generated project, by board then name, from a catalog.
    fn generate_all(catalog: &Catalog) -> Vec<(String, Generated)> {
        let mut out = Vec::new();
        for (id, b) in catalog.boards.iter().filter(|(_, b)| !b.custom) {
            let fpga = &catalog.fpgas[&b.fpga_id];
            for g in generate(id, b, fpga).unwrap_or_else(|e| panic!("{e}")) {
                out.push((id.clone(), with_board_id(g, id)));
            }
        }
        out
    }

    /// The generated projects in `examples/` are what the catalog snapshot
    /// gives (`CATALOG_SNAPSHOT`); after a catalog or generator change, run
    /// `scripts/generate-examples.sh`.
    #[test]
    fn generated_examples_are_up_to_date() {
        let catalog: Catalog = serde_json::from_str(CATALOG_SNAPSHOT).unwrap();
        let mut stale = Vec::new();
        for (board, g) in generate_all(&catalog) {
            let dir = examples_dir().join(&board).join(&g.name);
            let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
                .map(|d| {
                    d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            on_disk.sort();
            let wanted: Vec<String> = g.files.keys().cloned().collect();
            if on_disk != wanted {
                stale.push(format!(
                    "{board}/{}: files {on_disk:?}, want {wanted:?}",
                    g.name
                ));
                continue;
            }
            for (name, text) in &g.files {
                if std::fs::read_to_string(dir.join(name)).ok().as_deref() != Some(text) {
                    stale.push(format!("{board}/{}/{name}", g.name));
                }
            }
        }
        assert!(
            stale.is_empty(),
            "Generated examples are stale, run scripts/generate-examples.sh: {stale:#?}"
        );
    }

    /// Takes the catalog from the engine `LOGIX_DEV_ENGINE_BINARY` names,
    /// keeps the parts the generator reads in `CATALOG_SNAPSHOT`'s file, and
    /// writes every generated project to `examples/<board>/<name>/`
    /// (replaced). Run by `scripts/generate-examples.sh`.
    #[tokio::test]
    #[ignore]
    async fn generate_examples() {
        let exe = std::env::var("LOGIX_DEV_ENGINE_BINARY").expect("set LOGIX_DEV_ENGINE_BINARY");
        let tmp = tempfile::tempdir().unwrap();
        let folder = tmp.path().join("engine");
        std::fs::create_dir_all(&folder).unwrap();
        let engine = crate::engine::Engine::start(std::path::Path::new(&exe), &folder)
            .await
            .unwrap();
        let call = |tool: &'static str, args: serde_json::Value| {
            let engine = &engine;
            async move { engine.call(tool, args, None).await.unwrap().unwrap() }
        };
        let info = call("get_engine_info", serde_json::json!({})).await;
        let full = call(
            "get_catalog",
            serde_json::json!({"boards": {"fetch": true}, "fpgas": {"fetch": true}}),
        )
        .await;
        // The snapshot: what the generator reads, and where it came from.
        let mut boards = serde_json::Map::new();
        let mut fpgas = serde_json::Map::new();
        for (id, b) in full["boards"].as_object().unwrap() {
            if b["custom"].as_bool() == Some(true) {
                continue;
            }
            let mut kept = serde_json::Map::new();
            for key in ["name", "fpga_id", "requires", "resources"] {
                if let Some(v) = b.get(key) {
                    kept.insert(key.into(), v.clone());
                }
            }
            let fpga_id = b["fpga_id"].as_str().unwrap();
            let f = &full["fpgas"][fpga_id];
            fpgas.insert(
                fpga_id.into(),
                serde_json::json!({"architecture": f["architecture"], "part_number": f["part_number"]}),
            );
            boards.insert(id.clone(), kept.into());
        }
        let snapshot = serde_json::json!({
            "catalog": info["catalog"],
            "boards": boards,
            "fpgas": fpgas,
        });
        let text = serde_json::to_string_pretty(&snapshot).unwrap() + "\n";
        std::fs::write(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SNAPSHOT_FILE),
            &text,
        )
        .unwrap();
        let catalog: Catalog = serde_json::from_str(&text).unwrap();
        let mut made = 0;
        for (board, g) in generate_all(&catalog) {
            let dir = examples_dir().join(&board).join(&g.name);
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            for (name, text) in &g.files {
                std::fs::write(dir.join(name), text).unwrap();
            }
            made += 1;
        }
        println!(
            "{made} projects written to examples/, catalog {}",
            info["catalog"]
        );
    }

    #[test]
    fn one_led_on_an_rgb_driver_pin_is_a_plain_output() {
        let b = board(
            serde_json::json!({"name": "X", "fpga_id": "x", "resources": {
            "sys_clk": {"type": "clock", "pin": "35", "mhz": 12},
            "led": {"type": "leds", "pins": ["41"], "active": "low"}}}),
        );
        let all = files("x", &b, &fpga("ice40", "ICE40UP5K-SG48"));
        assert!(!all["blinky"].contains_key("rgb_driver.v"));
        assert!(all["blinky"]["main.v"].contains("assign led = ~counter[N-1];"));
        // Through the driver, an LED of unknown level turns on low.
        let b = board(
            serde_json::json!({"name": "X", "fpga_id": "x", "resources": {
            "sys_clk": {"type": "clock", "pin": "35", "mhz": 12},
            "rgb": {"type": "rgb_led", "red": "39", "green": "40", "blue": "41"}}}),
        );
        let all = files("x", &b, &fpga("ice40", "ICE40UP5K-SG48"));
        assert!(all["blinky"]["main_tb.v"].contains("`EXPECT_EQ(rgb, {2'b11, ~i[3]})"));
        assert!(all["blinky"]["rgb_driver.v"].contains(".RGB0PWM (on[0]),"));
        // An active-high LED on the driver's pins can't work through it.
        let b = board(
            serde_json::json!({"name": "X", "fpga_id": "x", "resources": {
            "rgb": {"type": "rgb_led", "red": "41", "green": "39", "blue": "40", "active": "high"}}}),
        );
        assert!(generate("x", &b, &fpga("ice40", "ICE40UP5K-SG48")).is_err());
    }
}
