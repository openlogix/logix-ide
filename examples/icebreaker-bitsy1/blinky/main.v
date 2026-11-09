// Blinky for the iCEBreaker bitsy v1: blinks its LED about once a second.

module main #(
  parameter N = 24  // Counter width: 2^24 cycles at 12 MHz is about 1.4 s.
) (
  input sys_clk, // 12 MHz clock.
  output led     // LED, lit when 0.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // The LED is lit while the top bit is 1 (it lights at 0, so inverted).
  assign led = ~counter[N-1];

endmodule
