//------------------------------------------------------------------
//-- Blinking LED
//------------------------------------------------------------------

// N is the counter width. leds[5] toggles every 2^(N-1) clocks.
// With the default N = 25 and a 27MHz clock, it toggles about
// every 0.6 seconds. The testbench uses a small N to run quickly.
module main #(
    parameter integer N = 25
) (
    input        sys_clk, // 27MHz clock
    output [5:0] leds     // leds[5] blinks, the rest are off. Active level unknown, taken as high.
);

  // The blink counter. It starts at 0 when the FPGA is configured.
  reg [N-1:0] counter = 0;

  // Count up on every clock.
  always @(posedge sys_clk) counter <= counter + 1;

  // leds[5] follows the counter's top bit. The other LEDs are off.
  assign leds = {counter[N-1], 5'b00000};

endmodule
