// Blinky for the butterstick r1.0: blinks LED 0 in green about once a
// second.
//
// Its seven RGB LEDs are a matrix: an LED lights when its pin in leds and
// a color pin in leds_colors are both 1.

module main #(
  parameter N = 25  // Counter width: 2^25 cycles at 30 MHz is about 1.1 s.
) (
  input sys_clk,            // 30 MHz clock.
  output [6:0] leds,        // LED matrix: one pin per LED, lit when 1.
  output [2:0] leds_colors  // LED matrix colors: [0] red, [1] green, [2] blue.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // Green only, and LED 0 lit while the top bit is 1; the others stay off.
  assign leds_colors = 3'b010;
  assign leds = {6'b000000, counter[N-1]};

endmodule
