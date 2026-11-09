// Blinky for the FPGA 101 Workshop Badge: blinks the RGB LED in green about
// once a second.
//
// The RGB LED is on the FPGA's RGB LED driver pads, so it's driven through
// the driver (SB_RGBA_DRV).

module main #(
  parameter N = 24  // Counter width: 2^24 cycles at 12 MHz is about 1.4 s.
) (
  input sys_clk,        // 12 MHz clock.
  output [2:0] rgb_led  // RGB LED: [0] red, [1] green, [2] blue, lit when 0.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // Which colors are lit: green while the counter's top bit is 1.
  wire red_on = 1'b0;
  wire green_on = counter[N-1];
  wire blue_on = 1'b0;

  // The RGB LED driver. The board wires red to its RGB2 pad, green to RGB0 and blue to RGB1.
  // A PWM input of 1 lights its color.
`ifdef SYNTHESIZE
  SB_RGBA_DRV #(
    .CURRENT_MODE("0b1"),       // Half current mode.
    .RGB0_CURRENT("0b000001"),  // The lowest current on each pad: bright enough.
    .RGB1_CURRENT("0b000001"),
    .RGB2_CURRENT("0b000001")
  ) driver (
    .CURREN(1'b1),    // Current reference on.
    .RGBLEDEN(1'b1),  // Driver on.
    .RGB0PWM(green_on),
    .RGB1PWM(blue_on),
    .RGB2PWM(red_on),
    .RGB0(rgb_led[1]),
    .RGB1(rgb_led[2]),
    .RGB2(rgb_led[0])
  );
`else
  // Simulation only: the driver pulls a pad low to light its color.
  assign rgb_led = ~{blue_on, green_on, red_on};
`endif

endmodule
