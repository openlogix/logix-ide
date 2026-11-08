//------------------------------------------------------------------
//-- Blinking segment
//------------------------------------------------------------------
// The Tang Primer 20K Lite has no user LEDs, so this blinky blinks the
// top bar (segment A) of the right-hand digit of an iCEBreaker 7-segment
// display PMOD plugged into the Lite's top-right PMOD connector.
// Written for Logix from the board's published pinout. It hasn't been
// tried on a real board yet: if it doesn't blink, please report it.

// N is the counter width. The segment toggles every 2^(N-1) clocks.
// With the default N = 25 and a 27MHz clock, it toggles about
// every 0.6 seconds. The testbench uses a small N to run quickly.
module main #(
    parameter integer N = 25
) (
    input        sys_clk,   // 27MHz clock
    output [6:0] segs_n,    // Segments A to G, active low.
    output       digit_sel  // Digit select: 1 shows the right-hand digit.
);

  // The blink counter. It starts at 0 when the FPGA is configured.
  reg [N-1:0] counter = 0;

  // Count up on every clock.
  always @(posedge sys_clk) counter <= counter + 1;

  // Segment A follows the counter's top bit; the others stay off
  // (high, since the segments are active low).
  assign segs_n = {6'b11_1111, ~counter[N-1]};

  // Show the right-hand digit.
  assign digit_sel = 1'b1;

endmodule
