# blinky

The "hello world" of FPGA boards: one LED blinking about once a second,
proof that the board, the tools and the upload all work.

## Applies to

Boards with an LED (a `leds`, `rgb_led` or `led_matrix` resource) and a
clock (a `clock` resource, or the FPGA's internal oscillator), both
working as shipped. Other boards get no `blinky`.

## Choices

- **The clock:** the clock resource with the lowest frequency, then the
  first by name. With none, the FPGA's internal oscillator (COMMON.md:
  Clocks), set to a frequency it documents, which the design states.
- **The LED resource:** the first `leds` resource by name; with none, the
  first `rgb_led`; with none, the first `led_matrix`.
- **What blinks:** bit 0 of a `leds` resource; the green color of an RGB
  LED; LED 0 of an LED matrix, in green. The resource's other LEDs and
  colors are held off.

## Ports

The clock (unless it's the internal oscillator), the whole LED resource,
and the drives (COMMON.md). Nothing else.

## Design

A free-running counter, `counter`, of `N` bits, counts clock cycles; its
top bit lights the LED, so the LED is on half the time. `N` is a parameter:
the smallest width with 2^N at least the clock's frequency in Hz, so one
blink (on and off) takes between one and two seconds, such as 24 at
12 MHz. The counter starts at 0 (the LED off).

## Testbench

With `N` set to 4: the LED is off at first, on after 8 clock cycles, off
again after 16. Checks each, then runs to 32 cycles. Held outputs (the
LED resource's others, drives) checked once.

## logix-ide.toml

`summary`: what blinks, in the board's words, "about once a second", such
as "Blinks LED 0 about once a second." or "Blinks the RGB LED in green
about once a second." Notes only as COMMON.md says.
