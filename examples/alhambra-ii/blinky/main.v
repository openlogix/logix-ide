// Blinky for the Alhambra II: blinks LED 0 about once a second.

module main #(
  parameter N = 24  // Counter width: 2^24 cycles at 12 MHz is about 1.4 s.
) (
  input sys_clk,      // 12 MHz clock.
  output [7:0] leds   // 8 LEDs, lit when 1.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // LED 0 is lit while the top bit is 1; the others stay off.
  assign leds = {7'b0000000, counter[N-1]};

endmodule
