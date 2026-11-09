// Bare minimum for the butterstick r1.0: the board's pins, ready for your
// design. It does nothing yet: the LEDs are held off.
//
// Its seven RGB LEDs are a matrix: an LED lights when its pin in leds and
// a color pin in leds_colors are both 1.

module main (
  input sys_clk,            // 30 MHz clock.
  output [6:0] leds,        // LED matrix: one pin per LED, lit when 1.
  output [2:0] leds_colors  // LED matrix colors: [0] red, [1] green, [2] blue.
);

  // Everything off for now.
  assign leds = 7'b0000000;
  assign leds_colors = 3'b000;

  // Your design goes here.

endmodule
