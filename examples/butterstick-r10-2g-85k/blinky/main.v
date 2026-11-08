//------------------------------------------------------------------
//-- Blinking LED matrix
//------------------------------------------------------------------

// N is the counter width. leds[6] toggles every 2^(N-1) clocks.
// With the default N = 25 and a 30MHz clock, it toggles about
// every 0.6 seconds. The testbench uses a small N to run quickly.
module main #(
    parameter integer N = 25
) (
    input        sys_clk,    // 30MHz clock
    output [6:0] leds,       // LED matrix: leds[6] blinks red. Active high.
    output [2:0] leds_color  // Its colors: [0] red, [1] green, [2] blue. Active high.
);

  // The blink counter. It starts at 0 when the FPGA is configured.
  reg [N-1:0] counter = 0;

  // Count up on every clock.
  always @(posedge sys_clk) counter <= counter + 1;

  // leds[6] follows the counter's top bit, in red.
  assign leds = {counter[N-1], 6'b000000};
  assign leds_color = {2'b00, 1'b1};

endmodule
