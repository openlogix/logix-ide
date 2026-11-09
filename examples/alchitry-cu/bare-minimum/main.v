// Bare minimum for the Alchitry Cu Development Board: the board's pins,
// ready for your design. It does nothing yet: the LEDs are held off.

module main (
  input sys_clk,     // 100 MHz clock.
  output [7:0] leds  // 8 LEDs, lit when 1.
);

  // Everything off for now.
  assign leds = 8'b00000000;

  // Your design goes here.

endmodule
