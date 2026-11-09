# bare-minimum

Every board's starting point, as the Arduino IDE's BareMinimum: a design
that does nothing yet, with the board's basic pins already declared and
wired, so the user adds logic without looking up pins first. New projects
start from it.

## Applies to

Every board.

## Ports

Every resource that works as shipped, of these types: `clock`, `leds`,
`rgb_led`, `led_matrix`, `buttons`, `switches` and `uart`; plus every
drive (COMMON.md). Connectors (`pmod`, `header`) and `unclassified` pins
are left out: what's on them is the user's choice.

Ports in this order: clocks, LEDs (`leds`, `rgb_led`, `led_matrix`),
buttons, switches, UARTs, drives; by name within each type.

- Clocks, buttons, switches and a UART's `rx` and `cts` are inputs, left
  unused.
- LEDs are outputs, held off (at the level opposite `active`).
- A UART's `tx` and `rts` are outputs, held at 1 (idle).
- Drives are outputs, held at their levels.

`main.v`'s first comment says the design does nothing yet, and where the
user's logic goes (a marked place in the module body).

## No ports

A board with none of these: `main` has no ports and an empty body with
the comment for the user's logic. The pin file has only comments: the
first line, and how to add a pin in the family's format, as an example
line for a port named `led` with `<pin>` in place of a pin.

## Testbench

- With a clock: runs 32 clock cycles, then checks. Without one: waits
  100 ns, then checks.
- Checks that every LED is off, every `tx` and `rts` is 1, and every drive
  is at its level.
- Without ports: waits 100 ns and finishes, with no checks but the comment
  for the user's.

## logix-ide.toml

`summary`: "The board's pins, ready for your design." Notes only as
COMMON.md says, and on a board with no clock that works as shipped: "No
clock works as shipped: <why, from the board's data>. The FPGA's internal
oscillator can give one." (only when the FPGA has one).
