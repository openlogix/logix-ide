// Blinky for the Mimas ECP5 Mini: blinks its LED about once a second.

module main #(
  parameter N = 27  // Counter width: 2^27 cycles at 100 MHz is about 1.3 s.
) (
  input sys_clk, // 100 MHz clock.
  output led     // LED, lit when 1.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // The LED is lit while the top bit is 1.
  assign led = counter[N-1];

endmodule
