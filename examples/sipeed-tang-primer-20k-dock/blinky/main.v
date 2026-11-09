// Blinky for the Sipeed Tang Primer 20K Dock: blinks LED 0 about once a
// second.

module main #(
  parameter N = 25  // Counter width: 2^25 cycles at 27 MHz is about 1.2 s.
) (
  input sys_clk,     // 27 MHz clock.
  output [5:0] leds  // 6 LEDs, lit when 1.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // LED 0 is lit while the top bit is 1; the others stay off.
  assign leds = {5'b00000, counter[N-1]};

endmodule
