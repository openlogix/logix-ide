//------------------------------------------------------------------
//-- Blinking LED, with the project's own board
//------------------------------------------------------------------

// This project has its own board, in custom-boards.json: the
// Alhambra II, uploaded with iceprog instead of openFPGALoader.
// Choose "Alhambra II (iceprog)" in the board list to use it, and
// "Alhambra II" to go back. Both have the same FPGA, so switching
// doesn't rebuild the design: only Upload changes.

// N is the counter width. leds[7] toggles every 2^(N-1) clocks.
// With the default N = 24 and a 12MHz clock, it toggles about
// every 0.7 seconds. The testbench uses a small N to run quickly.
module main #(
    parameter integer N = 24
) (
    input        sys_clk,  // 12MHz clock
    output [7:0] leds      // leds[7] blinks, the rest are off. Active high.
);

  // The blink counter. It starts at 0 when the FPGA is configured.
  reg [N-1:0] counter = 0;

  // Count up on every clock.
  always @(posedge sys_clk) counter <= counter + 1;

  // leds[7] follows the counter's top bit. The other LEDs are off.
  assign leds = {counter[N-1], 7'b0};

endmodule
