// Bare minimum for the Cmod A7-35T: the board's pins, ready for your
// design. It does nothing yet: the LEDs are held off.

module main (
  input sys_clk,     // 12 MHz clock.
  output [1:0] leds  // 2 LEDs, lit when 1.
);

  // Everything off for now.
  assign leds = 2'b00;

  // Your design goes here.

endmodule
