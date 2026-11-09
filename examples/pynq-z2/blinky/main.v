// Blinky for the Pynq-Z2: blinks LED 0 about once a second.

module main #(
  parameter N = 27  // Counter width: 2^27 cycles at 125 MHz is about 1.1 s.
) (
  input sys_clk,     // 125 MHz clock.
  output [3:0] leds  // 4 LEDs, lit when 1.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // LED 0 is lit while the top bit is 1; the others stay off.
  assign leds = {3'b000, counter[N-1]};

endmodule
