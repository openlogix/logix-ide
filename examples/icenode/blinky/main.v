//------------------------------------------------------------------
//-- Blinking LED
//------------------------------------------------------------------

// N is the counter width. rgb_led[2] toggles every 2^(N-1) clocks.
// With the default N = 24 and a 12MHz clock, it toggles about
// every 0.7 seconds. The testbench uses a small N to run quickly.
module main #(
    parameter integer N = 24
) (
    input        sys_clk, // 12MHz clock
    output [2:0] rgb_led  // rgb_led[2] blinks, the rest are off. Active low.
);

  // The blink counter. It starts at 0 when the FPGA is configured.
  reg [N-1:0] counter = 0;

  // Count up on every clock.
  always @(posedge sys_clk) counter <= counter + 1;

  // rgb_led[2] follows the counter's top bit. The other LEDs are off.
  rgb_driver rgb_driver (
      .on  ({counter[N-1], 2'b00}),
      .pins(rgb_led)
  );

endmodule
