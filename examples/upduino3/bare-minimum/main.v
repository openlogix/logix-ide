// Bare minimum for the UPduino v3.0: the board's pins, ready for your
// design. It does nothing yet: the RGB LED is held off.
//
// The board's 12 MHz oscillator reaches the FPGA only with a jumper
// shorted, so there's no clock pin here. The RGB LED is on the FPGA's RGB
// LED driver pads, so it's driven through the driver (SB_RGBA_DRV).

module main (
  output [2:0] rgb_led  // RGB LED: [0] red, [1] green, [2] blue, lit when 0.
);

  // Which colors are lit: none yet.
  wire red_on = 1'b0;
  wire green_on = 1'b0;
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

  // Your design goes here.

endmodule
