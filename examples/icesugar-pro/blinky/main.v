// Blinky for the Icesugar Pro: blinks the LED about once a second.
//
// Upload it to the board and the LED turns on and off, proof that the board,
// the tools and the upload all work.

module main #(
  // The counter's width. Its top bit flips every 2^(N-1) clock cycles, so at
  // 25 MHz, with N = 25, one blink (on and off) takes about 1.3 seconds.
  parameter N = 25
) (
  input sys_clk,   // 25 MHz clock
  output led       // LED, lit when 0
);

  // A free-running counter of clock cycles. It starts at 0, with the LED
  // off, and wraps around when it's full.
  reg [N-1:0] counter = {N{1'b0}};

  always @(posedge sys_clk) begin
    counter <= counter + 1'b1;
  end

  // The counter's top bit lights the LED, so it's on half the time. The LED
  // lights at 0, so the bit is inverted.
  assign led = !counter[N-1];

endmodule
