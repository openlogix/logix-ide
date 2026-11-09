// Blinky for the Alinx ARTIX-7 AX7035: blinks LED 0 about once a second.

module main #(
  parameter N = 26  // Counter width: 2^26 cycles at 50 MHz is about 1.3 s.
) (
  input sys_clk,       // 50 MHz clock.
  output [3:0] leds_n  // 4 LEDs, lit when 0.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // LED 0 is lit while the top bit is 1; the others stay off.
  assign leds_n = {3'b111, ~counter[N-1]};

endmodule
