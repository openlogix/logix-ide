// Blinky for the iCE40 UltraPlus Breakout: blinks its LED about once a
// second.
//
// The LED is on the FPGA's RGB LED driver pad, so it's driven through the
// driver (SB_RGBA_DRV).

module main #(
  parameter N = 24  // Counter width: 2^24 cycles at 12 MHz is about 1.4 s.
) (
  input sys_clk, // 12 MHz clock.
  output led     // LED, lit when 0.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // The LED is lit while the counter's top bit is 1.
  wire led_on = counter[N-1];

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
    .RGB2(led),
    .RGB0(),  // The pads the board doesn't use.
    .RGB1()
  );
`else
  // Simulation only: the driver pulls the pad low to light the LED.
  assign led = ~led_on;
`endif

endmodule
