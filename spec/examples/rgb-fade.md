# rgb-fade

The RGB LED fades in and out, one color at a time, red, then green, then
blue, about 4 seconds each, and looks even to the eye from off to the top:
the PWM follows a gamma curve. Each color has its own top brightness, so
the three look alike. It teaches PWM, a lookup table in block RAM and why
a straight brightness ramp doesn't look straight. (The user's design,
2026-11-24, tuned by eye on an iCESugar v1.5.)

## Applies to

Boards with an `rgb_led` resource and a clock (a `clock` resource, or the
FPGA's internal oscillator), both working as shipped. Other boards get no
`rgb-fade`. A WS2812-style addressable LED (one data pin) isn't an
`rgb_led` and doesn't count.

## Choices

- **The clock:** the clock resource with the lowest frequency, then the
  first by name. With none, the FPGA's internal oscillator at 12 MHz
  (COMMON.md: Clocks), which the design and `notes` state.
- **The LED:** the first `rgb_led` resource by name.

## Files

Besides COMMON.md's files, two files, the same on every board, copied
unchanged from [`rgb-fade/`](rgb-fade/) next to this file: `gamma.v` (the
module `gamma`, a 256-entry table of 12-bit values, `round(4095 * (i /
255) ^ 2.5)`, read one clock after its index, which the build puts in one
block RAM) and its testbench `gamma_tb.v`, which checks every entry
against the formula, its input and output named `gamma_in` and
`gamma_out`, and the testbench's view `gamma_tb.view` (COMMON.md, Waveform
view): groups "Clock" (`clk`), "Gamma function" (`gamma_in` and `gamma_out`
as analog lines, so the whole curve shows at once) and "Check" (`expected`
in decimal). Never edit them for one board.

`main_tb.view` is made per board, the same groups on every board (the
user's review, 2026-11-25), signals under `main_tb.dut.` unless named
otherwise: "Clock" (the testbench's clock, or `main_tb.dut.clk` when the
design makes its own); "Fade" (`color` in decimal, `rising` in binary,
`fade` as an analog line); "Gamma function" (`gamma_in` and `gamma_out` as
analog lines); "PWM" (`color_max` in decimal, `duty` and `pwm` as analog
lines, so the LED's on time shows where `pwm` is below `duty`); "LED"
(`main_tb.rgb_led` in binary, its bits shown).

## Ports

The clock (unless it's the internal oscillator), the whole RGB LED, and
the drives (COMMON.md). Nothing else.

## Design

`main.v`, top to bottom:

- **Parameters:** `STEP`, the clocks per fade step, so a color rises and
  falls (4,094 steps) in about 4 seconds: `round(4 * clock Hz / 4094)`,
  11724 at 12 MHz, 46898 at 48 MHz. `RED_MAX = 121`, `GREEN_MAX = 76`,
  `BLUE_MAX = 243`: each color's top, in PWM steps out of 4096, tuned by
  eye on an iCESugar v1.5 through its LED driver, so the three look alike
  and not too bright. The header comment says
  how to tune them, and that to look X times as bright all three take
  X^2.5 times the power.
- **PWM:** a 12-bit counter `pwm`, free running; the LED is on while
  `pwm < duty`.
- **The fade:** a timer counts 0 to `STEP - 1`; at its end, `fade`
  (11 bits, 0 to 2047) moves one step up while `rising`, and down
  otherwise, turning at 2047; when it falls back to 0 it rises again in
  the next color, `color` 0 red, 1 green, 2 blue, then red again.
- **The gamma function:** its input `gamma_in` is `fade`'s top 8 bits, and
  the `gamma` module gives its output `gamma_out` one clock later. The
  names say what they are, since Simulate shows them (the user's review,
  2026-11-25).
- **The duty:** `gamma_out * color_max / 4096`, `color_max` the color's top.
- **The LED:** the color's PWM input is `pwm < duty` for the current color
  and 0 for the others, through the LED driver the board's `ai_hints` name
  (on the iCE40 UltraPlus, `SB_RGBA_DRV` with `CURRENT_MODE("0b1")`, half
  current, and the lowest current, `"0b000001"`, on each pad: 2 mA), with
  the pads in the board's order, and a simulation model (COMMON.md, Code):
  each pad at 0 while its color's PWM input is 1.

## Testbench

`main_tb.v`, with `STEP` set to 2 and the same tops as `main.v`, runs four
fades (red, green, blue, red again) and checks every clock: the LED is off
or lit in the fade's color only; it's lit exactly while `dut.pwm <
dut.duty`; the duty rises, then falls, never above the color's top
(`4095 * top / 4096`); each fade reaches exactly that top, and the next
color follows. Held outputs (drives) checked once. `gamma_tb.v` is the
second testbench.

## logix-ide.toml

`summary`: "Fades the RGB LED in and out, red, green, blue, evenly to the
eye." Notes as COMMON.md says, and, on a board whose `rgb_led` has
`user_notes` that change which color shows, that.
