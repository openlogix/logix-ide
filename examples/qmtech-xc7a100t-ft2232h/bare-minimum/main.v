// Bare minimum for the QMTECH XC7A100T core board: the board's pins, ready
// for your design. It does nothing yet: the LEDs are held off.

module main (
  input sys_clk,       // 50 MHz clock.
  output [1:0] leds_n  // 2 LEDs, lit when 0.
);

  // Everything off for now.
  assign leds_n = 2'b11;

  // Your design goes here.

endmodule
