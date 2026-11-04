// Blinks one color of the RGB LED (the one on the FPGA's RGB0 pin).
// Written for Logix from the board's published pinout. It hasn't been
// tried on a real board yet: if it doesn't blink, please report it.

// N is the counter width. The LED toggles every 2^(N-1) clocks.
// With the default N = 24 and a 12MHz clock, it toggles about
// every 0.7 seconds. The testbench uses a small N to run quickly.
module main #(
    parameter integer N = 24
) (
    input        sys_clk,  // 12MHz clock
    // The RGB LED pins RGB0, RGB1 and RGB2, active low.
    output [2:0] rgb_led
);

  // The blink counter. It starts at 0 when the FPGA is configured.
  reg [N-1:0] counter = 0;

  // Count up on every clock.
  always @(posedge sys_clk) counter <= counter + 1;

  // Drive the RGB LED. RGB0 follows the counter's top bit, the other
  // two colors stay off.
  rgb_driver rgb_driver (
      .on  ({2'b00, counter[N-1]}),
      .pins(rgb_led)
  );

endmodule
