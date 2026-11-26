# serial-debug

A simple debugger over the board's USB serial port: the board prints a
value inside the design (a counter) to the Serial Monitor each time it
changes, and a value typed there (hexadecimal, then Enter) sets it. It
gives users two modules they can copy into their own designs, to watch and
set values on the real board (the user's request, 2026-11-19; a fuller
debug core may come later).

## Applies to

The same boards as [`serial`](serial.md): a `uart` resource and a clock (a
`clock` resource), both working as shipped. Other boards get no
`serial-debug`.

## Choices

As `serial`'s: the clock with the lowest frequency, then the first by name;
the first `uart` resource by name (only `tx` and `rx`); the first `leds`
resource by name, if the board has one (no `rgb_led` or `led_matrix`).

## Files

Besides COMMON.md's files, four files, the same on every board, copied
unchanged:

- `uart.v` and `uart_tb.v` from [`serial/`](serial/) (the UART, shared with
  `serial`, so both examples have the same one);
- `debug.v` and its testbench `debug_tb.v`, from
  [`serial-debug/`](serial-debug/).
- the views `uart_tb.view` from `serial/` and `debug_tb.view` from
  `serial-debug/` (COMMON.md, Waveform view). Users wire one module, `debug`, to the
  serial pins: it holds the UART (`uart_tx`, `uart_rx`) and its parts,
  `debug_print` and `debug_input`, inside, so a design needs one instance
  and no UART wiring of its own (the user's choice, 2026-11-19). One per
  serial port, since the port has one line each way: several values are
  watched together as one wider value.

They're the reusable parts: users copy them into their own projects. Never
edit them for one board; a change goes into its folder here and then into
every board's example (for `uart.v`, both examples).

`main.v`, `main_tb.v` and `main_tb.view` follow the reference in
[`serial-debug/reference/`](serial-debug/reference/), the Alhambra II's:
the same text, names, sections and comments on every board, changed only
where Per board, below, says. A change to the example is made in the
reference first. The reference is the spec's, not an example: when the
Alhambra II's own example is written, it comes out the same.

## Ports

The clock, `uart_tx` and `uart_rx`, the LED resource if there is one, and
the drives. Nothing else.

## Design

As the reference: a 16-bit `count` goes up by one at each `tick` (every
`TICK_CYCLES`, a parameter of `main`, half a second of the clock by
default) or takes the value typed; one `debug` (instance `dbg`, label
"count", 4 digits, `OVERWRITE` 1, so the value updates in place) prints it
one cycle after each change and gives the values typed. `BAUD` is a
parameter of `main`, 115200 by default, or the speed the UART's
`ai_hints` ask for, as in `serial`. The LEDs, if any, show the count's
lowest bits.

Counter widths are written as numbers, the smallest that fits, with a
comment saying why, as in `serial`.

The top comment says what the example does and how to try it (Tools >
Serial Monitor, at `BAUD`'s default), then the "Which port" paragraph, written as
in `serial` (from the UART's `user_notes`), ending "If you see nothing, try
the other."

## Testbench

`main_tb.v` plays the computer, as the reference: tasks `read_char`,
`type_char` and `check_line`, `BAUD` 1 Mbit/s, `CYCLES_PER_BIT` the
clock's MHz (rounded, as in `serial`, for a clock that isn't a whole
number of MHz), and `TICK_CYCLES` `400 * CYCLES_PER_BIT` (400 bit times,
so a printed line, 140 bit times, always fits between ticks). It checks:
the line idles high and the LEDs (if any) start at 0; the counts 1 and 2
come out as lines ending in a carriage return, with the LEDs; typing "4d"
and Enter prints 0x004D at once, shown on the LEDs; then it counts on to
0x004E.

## Per board

`main_tb.view`: as in `serial` (its Per board), the LED row names the
testbench's LED wire, and without a LED resource the LEDs group is left
out.

What changes from the reference, and nothing else (the reference's values
in parentheses):

- **Board name** in the first line of each file ("Alhambra II"): the
  board's `name`.
- **Clock:** its frequency in the clock port's comment, `CLK_HZ` of `dbg`
  (`12_000_000`), `TICK_CYCLES`'s default, half a second of
  the clock, and its comment (`6_000_000`, "0.5 s at 12 MHz"); the clock
  port's name is the clock resource's (`sys_clk`).
- **Tick counter width:** the smallest number of bits that counts to
  `TICK_CYCLES - 1`, in its declaration, its two assignments and its
  comment ("23 bits count up to 8 million, enough for 0.5 s at 12 MHz";
  24 bits at 25 MHz and 27 MHz, "up to 16 million"; 26 bits at 100 MHz,
  "up to 67 million").
- **LEDs,** by how many the LED resource has (`<n>`) and its `active`
  level, as in `serial`'s table, with these texts:

  | | 8 LEDs, active high (the reference) | Fewer than 8 | One LED |
  | --- | --- | --- | --- |
  | Top comment bullet | "A counter goes up every half second, and the 8 LEDs show it in binary." | "A counter goes up every half second, and the <n> LEDs show its lowest <n> bits in binary." | "A counter goes up every half second, and the LED shows its lowest bit: it blinks once a second." |
  | Port | `output [7:0] leds   // 8 LEDs, lit when 1.` | `output [<n-1>:0] <name>   // <n> LEDs, lit when 1.` | `output <name>          // LED, lit when 1.` |
  | Comment and `assign` | `// The LEDs: the counter's lowest 8 bits.` `assign leds = count[7:0];` | `// The LEDs: the counter's lowest <n> bits.` `assign <name> = count[<n-1>:0];` | `// The LED: the counter's lowest bit.` `assign <name> = count[0];` |
  | Testbench wire | `wire [7:0] leds;` | `wire [<n-1>:0] <name>;` | `wire <name>;` |
  | Testbench top comment | "that the LEDs show the count" | the same | "that the LED shows the count's lowest bit" |
  | Testbench, the ticks' comment | "Each tick, the new count comes out as a line, and on the LEDs." | the same | "..., and on the LED." |
  | Testbench checks | `` `EXPECT_EQ(leds, 8'h00)    // The count starts at 0. ``, then `8'h01`, `8'h02`, `8'h4D` | `<n>'h` and the value's lowest `<n>` bits | `1'b` and the value's lowest bit |

  **Active low** changes these: the port says "lit when 0"; the `assign`
  inverts (`~count[...]`) and its comment ends with ", inverted: the LEDs
  are lit by a 0." ("the LED is", for one), replacing the period; the
  checks expect `~` the value. **No LED resource:** no bullet (the first
  bullet is "A counter goes up every half second."), port, LED comment and
  `assign`, or wire; the testbench's top comment leaves out "that the LEDs
  show the count," the ticks' comment ends at "as a line.", and there are
  no LED checks; the counter's section is the same.
- **Comments' line breaks:** a comment whose text changed is rewrapped as
  a paragraph, lines at most 76 characters, "1 Mbit/s" kept together.
- **Baud:** as `serial`'s.
- **Which port:** as `serial`'s.
- **Testbench clock:** as `serial`'s (`CYCLES_PER_BIT`, its comment, the
  half period, the clock's comment, "a bit is 12 clock cycles").
- **Pin file:** as `serial`'s, the same file.

## Adding a board

As `serial` (its Adding a board): a board that gets `serial` gets
`serial-debug` too.

## logix-ide.toml

`summary`: "Watch and set a value inside the design over the USB serial
port: a simple debugger." Notes only as COMMON.md says. A `[serial]`
table as in `serial`.
