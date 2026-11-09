// Bare minimum for the Sipeed Tang Primer 20K Dock: the board's pins, ready
// for your design. It does nothing yet: the LEDs are held off.

module main (
  input sys_clk,     // 27 MHz clock.
  output [5:0] leds  // 6 LEDs, lit when 1.
);

  // Everything off for now.
  assign leds = 6'b000000;

  // Your design goes here.

endmodule
