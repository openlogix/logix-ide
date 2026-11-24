// Blinky for the Arty A7-35T: blinks LED 0 about once a second.
//
// Upload it to the board and LED 0 turns on and off, proof that the board,
// the tools and the upload all work.

module main #(
  // The counter's width. Its top bit flips every 2^(N-1) clock cycles, so at
  // 100 MHz, with N = 27, one blink (on and off) takes about 1.3 seconds.
  parameter N = 27
) (
  input sys_clk,      // 100 MHz clock
  output [3:0] leds   // LEDs 0 to 3, lit when 1
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
  assign leds[3:1] = 3'b000;

endmodule
