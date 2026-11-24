// Blinky for the Alhambra II: blinks LED 0 about once a second.
//
// Upload it to the board and LED 0 turns on and off, proof that the board,
// the tools and the upload all work.

module main #(
  // The counter's width. Its top bit flips every 2^(N-1) clock cycles, so at
  // 12 MHz, with N = 24, one blink (on and off) takes about 1.4 seconds.
  parameter N = 24
) (
  input sys_clk,      // 12 MHz clock
  output [7:0] leds   // LEDs 0 to 7, lit when 1
);

  // A free-running counter of clock cycles. It starts at 0, with the LED
  // off, and wraps around when it's full.
  reg [N-1:0] counter = {N{1'b0}};

  always @(posedge sys_clk) begin
    counter <= counter + 1'b1;
  end

  // The counter's top bit lights LED 0, so it's on half the time.
  assign leds[0] = counter[N-1];

  // The other LEDs are held off.
  assign leds[7:1] = 7'b0000000;

endmodule
