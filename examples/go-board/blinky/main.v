// Blinky for the Go Board: blinks LED 0 about once a second.

module main #(
  parameter N = 25  // Counter width: 2^25 cycles at 25 MHz is about 1.3 s.
) (
  input sys_clk,     // 25 MHz clock.
  output [3:0] leds  // 4 LEDs, lit when 1.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // LED 0 is lit while the top bit is 1; the others stay off.
  assign leds = {3'b000, counter[N-1]};

endmodule
