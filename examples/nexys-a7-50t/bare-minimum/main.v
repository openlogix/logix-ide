// Bare minimum for the Nexys A7-50T: the board's pins, ready for your
// design. It does nothing yet: the LEDs are held off.

module main (
  input sys_clk,      // 100 MHz clock.
  output [15:0] leds  // 16 LEDs, lit when 1.
);

  // Everything off for now.
  assign leds = 16'b0000000000000000;

  // Your design goes here.

endmodule
