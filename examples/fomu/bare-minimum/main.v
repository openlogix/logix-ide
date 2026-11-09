// Bare minimum for the Fomu: the board's pins, ready for your design. It
// does nothing yet: the RGB LED is held off.
//
// The RGB LED is on the FPGA's RGB LED driver pads, so it's driven through
// the driver (SB_RGBA_DRV).

module main (
  input sys_clk,        // 48 MHz clock.
  output [2:0] rgb_led, // RGB LED: [0] red, [1] green, [2] blue, lit when 0.
  output usb_dp,        // Held low: disconnects the board from the computer's USB.
  output usb_dn,        // Held low: disconnects the board from the computer's USB.
  output usb_dp_pu      // Held low: disconnects the board from the computer's USB.
);

  // Held low in every design: they disconnect the board from the computer's
  // USB, which would otherwise try to talk to a design without a USB stack.
  assign usb_dp = 1'b0;
  assign usb_dn = 1'b0;
  assign usb_dp_pu = 1'b0;

  // Which colors are lit: none yet.
  wire red_on = 1'b0;
  wire green_on = 1'b0;
  wire blue_on = 1'b0;

  // The RGB LED driver. The board wires red to its RGB1 pad, green to RGB2 and blue to RGB0.
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
    .RGB0PWM(blue_on),
    .RGB1PWM(red_on),
    .RGB2PWM(green_on),
    .RGB0(rgb_led[2]),
    .RGB1(rgb_led[0]),
    .RGB2(rgb_led[1])
  );
`else
  // Simulation only: the driver pulls a pad low to light its color.
  assign rgb_led = ~{blue_on, green_on, red_on};
`endif

  // Your design goes here.

endmodule
