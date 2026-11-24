// Bare minimum for the iCESugar v1.5: a design that does nothing yet.
//
// The board's clock, RGB LED, switches and USB serial port are already
// declared and wired in pinout.pcf, so you can start adding your own logic
// right away. The RGB LED is held off and the serial line idles until your
// logic uses them. Build and upload it as it is: nothing lights up, which
// shows the board takes a design.

module main (
  input sys_clk,          // 12 MHz clock
  output [2:0] rgb_led,   // RGB LED: [0] red, [1] green, [2] blue, lit when 0
  input [3:0] switches,   // DIP switches S1 to S4, 0 when on
  output uart_tx,         // Serial data to the computer, idles at 1
  input uart_rx           // Serial data from the computer, idles at 1
);

  // The RGB LED is on the FPGA's RGB LED driver pads, which can only be
  // driven through SB_RGBA_DRV, the iCE40 UltraPlus's LED driver cell. It
  // pulls a pad low (lighting its color) while its PWM input is 1, and leaves
  // it off while it's 0. All three inputs are 0 here, so the LED stays off.
  wire red_on = 1'b0;
  wire green_on = 1'b0;
  wire blue_on = 1'b0;

`ifdef SYNTHESIZE
  // The driver's pads: green on RGB0, red on RGB1, blue on RGB2.
  SB_RGBA_DRV #(
    .CURRENT_MODE("0b1"),          // Half current mode
    .RGB0_CURRENT("0b000001"),     // The lowest current, bright enough
    .RGB1_CURRENT("0b000001"),
    .RGB2_CURRENT("0b000001")
  ) rgb_driver (
    .CURREN(1'b1),                 // Turn the driver's current source on
    .RGBLEDEN(1'b1),               // Turn the three LED outputs on
    .RGB0PWM(green_on),
    .RGB1PWM(red_on),
    .RGB2PWM(blue_on),
    .RGB0(rgb_led[1]),
    .RGB1(rgb_led[0]),
    .RGB2(rgb_led[2])
  );
`else
  // Simulation only: the simulator has no model of SB_RGBA_DRV, so these
  // lines give the pads the levels the driver gives them: 0 (lit) while a
  // color's input is 1, else 1 (off).
  assign rgb_led[0] = !red_on;
  assign rgb_led[1] = !green_on;
  assign rgb_led[2] = !blue_on;
`endif

  // The serial line idles at 1 when nothing is sent.
  assign uart_tx = 1'b1;

  // Your logic goes here.

endmodule
