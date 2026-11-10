// Blinky for the iCESugar v1.5: blinks the RGB LED in green about once a
// second.

module main #(
  parameter N = 24  // Counter width: 2^24 cycles at 12 MHz is about 1.4 s.
) (
  input sys_clk,       // 12 MHz clock.
  output [2:0] rgb_led // RGB LED: [0] red, [1] green, [2] blue, lit when 0.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // Green is lit while the counter's top bit is 1; red and blue stay off.
  wire green_on = counter[N-1];

  // The RGB LED driver. The board wires red to its RGB1 pad, green to
  // RGB2 and blue to RGB0. A PWM input of 1 lights that color.
`ifdef SYNTHESIZE
  SB_RGBA_DRV #(
    .CURRENT_MODE("0b1"),       // Half current mode.
    .RGB0_CURRENT("0b000001"),  // The lowest current on each pad: bright enough.
    .RGB1_CURRENT("0b000001"),
    .RGB2_CURRENT("0b000001")
  ) driver (
    .CURREN(1'b1),    // Current reference on.
    .RGBLEDEN(1'b1),  // Driver on.
    .RGB0PWM(1'b0),
    .RGB1PWM(1'b0),
    .RGB2PWM(green_on),
    .RGB0(rgb_led[2]),
    .RGB1(rgb_led[0]),
    .RGB2(rgb_led[1])
  );
`else
  // Simulation only: the driver pulls a color's pad low to light it.
  assign rgb_led = ~{1'b0, green_on, 1'b0};
`endif

endmodule
