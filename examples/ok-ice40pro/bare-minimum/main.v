// Bare minimum for the OK-iCE40Pro: the board's pins, ready for your
// design. It does nothing yet: the LED is held off.
//
// The LED is on the FPGA's RGB LED driver pad, so it's driven through the
// driver (SB_RGBA_DRV).

module main (
  input sys_clk, // 12 MHz clock.
  output led     // LED, lit when 0.
);

  // Whether the LED is lit: not yet.
  wire led_on = 1'b0;

  // The RGB LED driver. The board wires the LED to its RGB2 pad.
  // A PWM input of 1 lights the LED.
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
    .RGB2PWM(led_on),
    .RGB2(led)
  );
`else
  // Simulation only: the driver pulls the pad low to light the LED.
  assign led = ~led_on;
`endif

  // Your design goes here.

endmodule
