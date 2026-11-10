// Bare minimum for the iCESugar v1.5: the board's pins, ready for your
// design. It does nothing yet: the RGB LED is off and the UART is idle.
// Add your logic where marked below.

module main (
  input sys_clk,        // 12 MHz clock.
  output [2:0] rgb_led, // RGB LED: [0] red, [1] green, [2] blue, lit when 0.
  input [3:0] switches, // DIP switches, 0 when on.
  input uart_rx,        // UART from the computer (the iCELink's serial port).
  output uart_tx        // UART to the computer, 1 when idle.
);

  // The RGB LED's colors, 1 to light one. All off for now.
  wire red_on = 1'b0;
  wire green_on = 1'b0;
  wire blue_on = 1'b0;

  // The UART's output, idle.
  assign uart_tx = 1'b1;

  // Your logic goes here.

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
    .RGB0PWM(blue_on),
    .RGB1PWM(red_on),
    .RGB2PWM(green_on),
    .RGB0(rgb_led[2]),
    .RGB1(rgb_led[0]),
    .RGB2(rgb_led[1])
  );
`else
  // Simulation only: the driver pulls a color's pad low to light it.
  assign rgb_led = ~{blue_on, green_on, red_on};
`endif

endmodule
