# What every standard example shares

The rules every standard example follows, on every board. Each example's
own file (`<example>.md`) adds what it does. When the two disagree, the
example's file wins.

## Who it's for

Beginners opening their first project for a board, as with the Arduino
IDE's examples: the design should be short, plain and explained, and work
on the board as it comes, with no jumpers or add-ons.

## Files

A standard example is a Logix project (the IDE's project model, CLAUDE.md:
Project on disk, `logix-ide.toml`):

| File | What it holds |
| --- | --- |
| `main.v` | The design, top module `main` |
| `pinout.<ext>` | The pin file, `<ext>` from the board's family (`constraint_extension`: `pcf`, `lpf`, `cst` or `xdc`) |
| `main_tb.v` | The testbench |
| `main_tb.gtkw` | The GTKWave layout Simulate opens, when there are signals to show |
| `logix-ide.toml` | `# Logix IDE project settings.`, then `board`, `summary` and optional `notes` (see below) |

`logix-ide.toml` names the board's default variant by leaving `[variant]`
out. `summary` is one line saying what the example does on this board,
such as "Blinks the green LED of the RGB LED about once a second."
`notes` is a list of lines, only for what someone must know before using
it on this board, such as "No clock works as shipped: the design uses the
FPGA's internal oscillator."; most examples have none.

## The board's data

Everything about the board comes from its `board-info.sh` entry (README.md,
Inputs):

- **Resources** (Logix Catalog's `docs/model.md`) give the ports: a
  resource's name is the port's name, one pin a plain port, several a
  vector, bit 0 first. An `rgb_led` is a 3-bit vector in its fields'
  order: `[0]` red, `[1]` green, `[2]` blue. An `led_matrix` is two
  ports: `<name>` with a bit per LED (its `leds`), and `<name>_colors`,
  `[0]` red, `[1]` green, `[2]` blue. Never pick a resource by its
  position in the list: each example's file gives explicit rules.
- **Only what works as shipped:** a resource with a `setup` requirement
  (such as a jumper) is never used.
- **Drives:** the board's `drive` requirements are in every example, and
  so are those of the resources it uses: each is an output port named by
  the drive's `name`, held at its `level`, with a comment giving its `why`.
- **`active`** says which level turns an LED on, or which level a pressed
  button gives. When it's missing, assume high, and add a note: "The
  LEDs' on level isn't known: if they're lit when they should be off,
  invert them."
- **`ai_hints`** on the board and on each resource used: follow them. They
  say what the other fields can't, such as an LED that must be driven
  through an FPGA primitive.
- **`user_notes`** on a resource the example uses: when it changes what
  the user sees, say so in `notes` (such as a board revision with another
  clock frequency, so the LED blinks at another rate).
- **Clocks:** a clock resource's `mhz` is its frequency. With no clock
  that works as shipped, use the FPGA's internal oscillator, if it has one
  the toolchain supports, and say so in `notes`.

## Pin file

- Every port of `main` is in it, and nothing else.
- Grouped by resource, in the order the ports are declared, each group
  under a comment naming the resource and what it is, such as
  `# rgb_led: RGB LED (active low).`
- A resource's `io_standard`, where the family's format has one (not
  iCE40).
- The clock's frequency where the toolchain reads it: ECP5 `FREQUENCY
  PORT`, and Xilinx `create_clock -period <ns> [get_ports {<clock>}]`
  right after the clock's pin line, with the period in ns to three
  decimals and no `-name` (CLAUDE.md: Clock constraint in Xilinx examples).
- No pull or drive strength settings: the family's defaults.
- The first line says what the file is: `<comment> Pins of <board name>
  used by main.v.`

## Code

- Verilog-2005, two-space indents, lines of at most 100 characters.
- `main.v` starts with a comment: what the design does, on which board, in
  plain words. Each block has a short comment saying what it does and why.
- Ports are declared in the module header, one per line, each with a
  comment saying what's wired to it, such as `output led, // LED, lit
  when 0`.
- Time constants, such as a counter's width, are parameters, so the
  testbench can make them small.
- Outputs the design doesn't use, such as other LEDs, are held off.
- FPGA primitives only where needed (an internal oscillator, or what an
  `ai_hints` line asks for), each in a module of its own file only when
  that makes `main.v` clearer.
- A primitive the simulator has no behavior for (the toolchain's
  simulation models make it a black box, such as iCE40's `SB_HFOSC` and
  `SB_RGBA_DRV`) is used under `` `ifdef SYNTHESIZE `` (defined in builds,
  not in tests and simulations), with a few lines of equivalent behavior
  under `` `else ``, commented as simulation only, so the testbench sees
  what the board does: an internal oscillator's clock at its frequency,
  an LED driver's pads at the levels that light or darken the LEDs.

## Testbench

`main_tb.v` follows Logix Engine's guidelines (its docs/tool-api.md,
Writing testbenches) and the IDE's convention: failures are reported with
a one-line `EXPECT_EQ(actual_val, expected_val)` macro that calls `$error`
with both values.

- `` `timescale 1ns / 1ps ``, module `main_tb`, the design instantiated as
  `dut`, with small parameters so the run is short.
- A clock, when the design has a clock input, at the board's frequency
  (half period in ns, rounded to the picosecond).
- `$dumpvars(0, main_tb)` first, and `$finish` last, after
  `$display("End of simulation")`.
- Checks for what the example promises, each with a comment.
- A comment showing where the user adds their own checks.

## GTKWave layout

`main_tb.gtkw` lists the signals worth watching, in this order: the clock,
the design's state (such as its counter), the outputs that show the
behavior (the LEDs), then the held drives. Signals by their testbench
names (`main_tb.<name>`, or `main_tb.dut.<name>` inside the design), with
`-<title>` comment lines between groups. No layout when there's nothing
to show.
