// Bare minimum for the EDU-CIAA-FPGA: the board's pins, ready for your
// design. It does nothing yet: the LED is held off.

module main (
  input sys_clk, // 12 MHz clock.
  output led     // LED, lit when 1.
);

  // Everything off for now.
  assign led = 1'b0;

  // Your design goes here.

endmodule
