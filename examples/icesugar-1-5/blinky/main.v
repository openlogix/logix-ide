// Blinky for the iCESugar v1.5: blinks the RGB LED in green about once a
// second.
//
// Upload it to the board and the RGB LED turns green and off, proof that the
// board, the tools and the upload all work.

module main #(
  // The counter's width. Its top bit flips every 2^(N-1) clock cycles, so at
  // 12 MHz, with N = 24, one blink (on and off) takes about 1.4 seconds.
  parameter N = 24
) (
  input sys_clk,         // 12 MHz clock
  output [2:0] rgb_led   // RGB LED: [0] red, [1] green, [2] blue, lit when 0
);

  // A free-running counter of clock cycles. It starts at 0, with the LED
  // off, and wraps around when it's full.
  reg [N-1:0] counter = {N{1'b0}};

  always @(posedge sys_clk) begin
    counter <= counter + 1'b1;
  end

  // The counter's top bit lights green, so it's on half the time. Red and
  // blue are held off.
  wire green_on = counter[N-1];
  wire red_on = 1'b0;
  wire blue_on = 1'b0;

  // The RGB LED is on the FPGA's RGB LED driver pads, which can only be
  // driven through SB_RGBA_DRV, the iCE40 UltraPlus's LED driver cell. It
  // pulls a pad low (lighting its color) while its PWM input is 1, and leaves
  // it off while it's 0.
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

endmodule
