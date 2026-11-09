// Bare minimum for the Icepi Zero: the board's pins, ready for your design.
// It does nothing yet: the LEDs are held off.

module main (
  input sys_clk,     // 50 MHz clock.
  output [4:0] leds  // 5 LEDs, lit when 1.
);

  // Everything off for now.
  assign leds = 5'b00000;

  // Your design goes here.

endmodule
