// Bare minimum for the OrangeCrab r0.2 85K: the board's pins, ready for
// your design. It does nothing yet: the LED is held off.

module main (
  input sys_clk, // 48 MHz clock.
  output led     // LED, lit when 0.
);

  // Everything off for now.
  assign led = 1'b1;

  // Your design goes here.

endmodule
