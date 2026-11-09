// Bare minimum for the Arty A7-100T: the board's pins, ready for your
// design. It does nothing yet: the LEDs are held off.

module main (
  input sys_clk,     // 100 MHz clock.
  output [3:0] leds  // 4 LEDs, lit when 1.
);

  // Everything off for now.
  assign leds = 4'b0000;

  // Your design goes here.

endmodule
