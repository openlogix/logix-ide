# serial

The board's USB serial port, both ways: the board echoes what you type,
shows the last character's code on its LEDs, and greets you after 5 seconds
of quiet. It also gives users a UART transmitter and receiver they can copy
into their own designs.

## Applies to

Boards with a `uart` resource and a clock (a `clock` resource), both
working as shipped. Other boards get no `serial`.

## Choices

- **The clock:** the clock resource with the lowest frequency, then the
  first by name.
- **The UART:** the first `uart` resource by name. Only its `tx` and `rx`
  are used, never `rts` or `cts`.
- **The LEDs:** the first `leds` resource by name, if the board has one;
  without one, the example has no LEDs (no `rgb_led` or `led_matrix`).

## Files

Besides COMMON.md's files, two files, the same on every board, copied
unchanged from [`serial/`](serial/) next to this file: `uart.v` (the
modules `uart_tx` and `uart_rx`, in one file so a design that uses them
gets one file, not two; the user's choice, 2026-11-19) and its testbench
`uart_tb.v`. They're the reusable parts: users copy them into their own
projects. Never edit them for one board; a change goes into `serial/` and
then into every board's example.

The view `uart_tb.view` (COMMON.md, Waveform view) is copied unchanged
from `serial/` too.

`main.v`, `main_tb.v` and `main_tb.view` follow the reference in
[`serial/reference/`](serial/reference/), the Alhambra II's: the same text,
names, sections and comments on every board, changed only where Per board,
below, says. So every board's example reads the same, and a change to the
example is made in the reference first. The reference is the spec's, not
an example: when the Alhambra II's own example is written, it comes out
the same.

## Ports

The clock, `uart_tx` and `uart_rx` (COMMON.md: a `uart` is a port per
signal), the LED resource if there is one, and the drives. Nothing else.

## Design

`main.v` uses `uart_tx` (instance `sender`) and `uart_rx` (instance
`receiver`), with `CLK_HZ` the clock's frequency in Hz and `BAUD` a
parameter of `main`, 115200 by default. Its parts, each in a section with
a heading comment (`// ---- ... ----` style, as the files in `serial/`):

- **Echo:** each received character waits in `echo_char` (`echo_waiting`
  set) until the transmitter takes it; one received while another still
  waits replaces it. An echo goes before the greeting's next character.
- **LEDs** (with a LED resource): `last_char` holds the last character
  received, and the LEDs show its lowest bits, bit 0 on LED 0, as many as
  the board has, inverted if they're active low. All LEDs off at first.
- **Greeting:** `"Hello from your FPGA board"`, the same on every board,
  followed by a carriage return and a line feed, written in octal
  (`\015\012`; Verilog-2005 has no `\r`). A carriage return and line feed
  also come before it, sent only when a character was received since the
  last greeting (`typed`), so it starts on a new line after the echoes.
  The text is one `localparam` with both, and `char_number` starts at 0
  (after typing) or 2 (otherwise); after the last character it goes back
  to 0, so it never points past the text (COMMON.md, No reads past the
  end).
- **Quiet time:** `quiet_cycles` counts clock cycles and starts over on
  every character received; at `QUIET_CYCLES` (a parameter of `main`, 5
  seconds of the clock by default, such as `60_000_000` at 12 MHz) it starts
  over and starts a greeting, unless one is still being sent. So the
  greeting comes after 5 seconds of quiet, then every 5 seconds.

Counter and index widths are written as numbers, the smallest that fits,
with a comment saying why (such as "26 bits count up to 67 million, enough
for 5 seconds at 12 MHz"), not computed with `$clog2`, so beginners read a
plain size.

The top comment says what the example does and how to try it: open the
board's serial port in a terminal program (PuTTY on Windows, `screen
<port> 115200` on macOS and Linux) at 115200 baud, 8N1. Then a "Which
port" paragraph from the UART's `user_notes`, in plain words: when they
say the computer lists several serial ports for the board (such as an
FTDI chip's two channels), which one to use and how to tell, and "If you
see no greeting or echo, try the other."; when they say nothing about it,
no paragraph.

## Testbench

`main_tb.v` plays the computer: it reads the board's TX line and types on
its RX line, with tasks `read_char`, `type_char`, `type_and_check_echo` and
`check_greeting`. It uses `BAUD` 1 Mbit/s, and a clock whose frequency in
MHz is a whole number gives a whole number of cycles per bit
(`CYCLES_PER_BIT`, the clock's MHz); a clock that doesn't is rounded, as
`uart_tx` does. `QUIET_CYCLES` is 500 bit times, longer than any greeting.
It checks:

- the line idles high, and the LEDs (if any) are all off;
- the first greeting comes after the quiet time, whole, without a new line
  first;
- "A" typed is echoed, and the LEDs show its lowest bits;
- "B" typed half a quiet time later is echoed before any greeting (typing
  started the quiet time over), and the greeting comes one quiet time after
  "B": at least `QUIET_CYCLES` after typing began, at most 10 bits later;
  it starts on a new line;
- the next greeting, with nothing typed since, has no new line first.

## Per board

`main_tb.view` is the reference's with its LED row (`main_tb.leds`) naming
the testbench's LED wire (`leds` or `led`); without a LED resource, the
LEDs group and everything under it are left out.

What changes from the reference, and nothing else (the reference's values
in parentheses):

- **Board name** in the first line of each file ("Alhambra II"): the
  board's `name`.
- **Clock:** its frequency in the clock port's comment, `CLK_HZ` of both
  instances (`12_000_000`), `QUIET_CYCLES`'s default, 5 seconds of the
  clock, and its comment (`60_000_000`, "5 s at 12 MHz"); the clock port's
  name is the clock resource's (`sys_clk`).
- **Quiet counter width:** the smallest number of bits that counts to
  `QUIET_CYCLES - 1`, in its declaration, its three assignments and its
  comment ("26 bits count up to 67 million, enough for 5 seconds at
  12 MHz"; 27 bits at 25 MHz, 28 at 27 MHz, 29 at 100 MHz).
- **LEDs,** by how many the LED resource has (`<n>`) and its `active`
  level. The texts, exactly (each in the reference's place):

  | | 8 LEDs, active high (the reference) | Fewer than 8 | One LED |
  | --- | --- | --- | --- |
  | Top comment bullet | "The 8 LEDs show the code of the last character you typed, in binary. For example "A" (code 65) lights 0100 0001." | "The <n> LEDs show the lowest <n> bits of the code of the last character you typed, in binary. For example "A" (code 65, 0100 0001 in binary) lights <its lowest n bits>: only LED 0." | "The LED shows the lowest bit of the code of the last character you typed: lit for an odd code, such as "A" (65), off for an even one, such as "B" (66)." |
  | Port | `output [7:0] leds   // 8 LEDs, lit when 1.` | `output [<n-1>:0] <name>   // <n> LEDs, lit when 1.` | `output <name>          // LED, lit when 1.` (a plain port, COMMON.md) |
  | Section title | "The LEDs: the last character received" | the same | "The LED: the last character received" |
  | `assign` | `assign leds = last_char;` | `assign <name> = last_char[<n-1>:0];  // Its lowest <n> bits, bit 0 on LED 0.` | `assign <name> = last_char[0];  // Its lowest bit.` |
  | Testbench wire | `wire [7:0] leds;` | `wire [<n-1>:0] <name>;` | `wire <name>;` |
  | Testbench, top comment and the echo check's comment | "shown on the LEDs" | the same | "shown on the LED" |
  | Testbench, first check | `` `EXPECT_EQ(leds, 8'h00)    // Nothing received yet. `` | `` `EXPECT_EQ(<name>, <n>'h0)     // Nothing received yet. `` | `` `EXPECT_EQ(<name>, 1'b0)      // Nothing received yet. `` |
  | Testbench, echo check | `` `EXPECT_EQ(leds, c)  // The LEDs show its code. `` | `` `EXPECT_EQ(<name>, c[<n-1>:0])  // The LEDs show its lowest <n> bits. `` | `` `EXPECT_EQ(<name>, c[0])  // The LED shows its lowest bit. `` |

  **Active low** changes these: the port says "lit when 0"; the `assign`
  inverts (`~last_char[...]`) and its comment ends with ", inverted: the
  LED is lit by a 0." ("the LEDs are", for several), replacing the period
  (8 LEDs: `// The code, inverted: the LEDs are lit by a 0.`); the first
  check expects all 1s, with "// Nothing received yet: the LED is off."
  ("the LEDs are off"); the echo check expects `~` the value, its comment
  ending ", lit by a 0." **No LED resource:** no bullet, port, section or
  wire; the testbench's top comment and echo comment say "echoed back" /
  "echoed" without "and shown on the LEDs", and there are no LED checks.
- **Which port:** the paragraph as in the reference when the UART's
  `user_notes` say the board's USB chip is an FTDI with two channels and
  the UART on interface 1; for another case the notes describe, the same
  shape in their words; none when the notes say nothing about it.
- **Testbench clock:** `CYCLES_PER_BIT` (the clock's MHz, as a whole
  number) and its comment ("12 MHz / 1 Mbit/s."), the half period
  (`#41.667`: 500 / MHz ns, three decimals), the clock's comment ("The
  board's 12 MHz clock, ..."), and "a bit is 12 clock cycles" in the top
  comment.
- **Pin file:** COMMON.md's rules, the ports in their declared order, in
  three groups, each under its comment (`#`, or `//` in `.cst`): `<clock>:
  <MHz> MHz clock.` (with COMMON.md's frequency line for ECP5 and Xilinx),
  `uart: the USB serial port.` (`uart_tx`, then `uart_rx`), and, with LEDs,
  `<name>: <n> LEDs (active <level>).`, or `<name>: LED (active <level>).`
  for one. Each port's lines are the family's format: `.pcf` `set_io <port>
  <pin>`; `.lpf` `LOCATE COMP "<port>" SITE "<pin>";` then `IOBUF PORT
  "<port>" IO_TYPE=<io_standard>;`; `.cst` `IO_LOC "<port>" <pin>;` then,
  with an `io_standard`, `IO_PORT "<port>" IO_TYPE=<io_standard>;`; `.xdc`
  `set_property -dict { PACKAGE_PIN <pin> IOSTANDARD <io_standard> }
  [get_ports {<port>}]`. The I/O standard is the resource's own, or none
  when it has none (`.pcf` never has one).

## Adding a board

1. **Logix Catalog:** the board needs a `uart` resource, with its pins from
   a source (the maker's pin files or schematic, LiteX or Amaranth), in
   `source`, and `user_notes` saying what it's wired to and, when computers
   list several serial ports for the board, which one it is (the catalog's
   `docs/model.md`). A pull request in `logix-catalog`.
2. **Logix Engine:** a catalog release with it, named by the engine's dev
   config (`remote-config/logix-engine/<NNN>/dev-channel.jsonc`, a pull request),
   or tried first with `LOGIX_ENGINE_DEV_REMOTE_CONFIG`.
3. **Here:** run the prompt (README.md) for `serial` and the board, then the
   checks. If the board doesn't fit Per board (such as a clock that isn't a
   whole number of MHz), say what's missing and fix this spec first.

## logix-ide.toml

`summary`: "Echoes what you type over the USB serial port, shows it on the
LEDs, and greets you after 5 seconds of quiet." With one LED, "on the LED";
without LEDs, without "shows it on the LEDs,". Notes only as COMMON.md says.
