// Blinky for the Icesugar Pro: blinks its LED about once a second.

module main #(
  parameter N = 25  // Counter width: 2^25 cycles at 25 MHz is about 1.3 s.
) (
  input sys_clk, // 25 MHz clock.
  output led     // LED, lit when 0.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // The LED is lit while the top bit is 1 (it lights at 0, so inverted).
  assign led = ~counter[N-1];

endmodule
