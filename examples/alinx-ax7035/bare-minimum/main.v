// Bare minimum for the Alinx ARTIX-7 AX7035: the board's pins, ready for
// your design. It does nothing yet: the LEDs are held off.

module main (
  input sys_clk,       // 50 MHz clock.
  output [3:0] leds_n  // 4 LEDs, lit when 0.
);

  // Everything off for now.
  assign leds_n = 4'b1111;

  // Your design goes here.

endmodule
