// Bare minimum for the ColorLight-i9 v7.2: the board's pins, ready for your
// design. It does nothing yet: the LED is held off.

module main (
  input sys_clk, // 25 MHz clock.
  output led     // LED, lit when 0.
);

  // Everything off for now.
  assign led = 1'b1;

  // Your design goes here.

endmodule
