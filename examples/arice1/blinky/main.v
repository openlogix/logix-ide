// Blinky for the ARiCE v1: blinks LED 0 about once a second.
//
// The LEDs are on the FPGA's RGB LED driver pads, so they're driven through
// the driver (SB_RGBA_DRV).

module main #(
  parameter N = 24  // Counter width: 2^24 cycles at 12 MHz is about 1.4 s.
) (
  input sys_clk,        // 12 MHz clock.
  output [2:0] rgb_led  // 3 LEDs, lit when 0.
);

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // Which LEDs are lit: LED 0 while the counter's top bit is 1.
  wire [2:0] led_on = {2'b00, counter[N-1]};

  // The RGB LED driver. The board wires bit 0 to its RGB0 pad, bit 1 to RGB1 and bit 2 to RGB2.
  // A PWM input of 1 lights its LED.
`ifdef SYNTHESIZE
  SB_RGBA_DRV #(
    .CURRENT_MODE("0b1"),       // Half current mode.
    .RGB0_CURRENT("0b000001"),  // The lowest current on each pad: bright enough.
    .RGB1_CURRENT("0b000001"),
    .RGB2_CURRENT("0b000001")
  ) driver (
    .CURREN(1'b1),    // Current reference on.
    .RGBLEDEN(1'b1),  // Driver on.
    .RGB0PWM(led_on[0]),
    .RGB1PWM(led_on[1]),
    .RGB2PWM(led_on[2]),
    .RGB0(rgb_led[0]),
    .RGB1(rgb_led[1]),
    .RGB2(rgb_led[2])
  );
`else
  // Simulation only: the driver pulls a pad low to light its LED.
  assign rgb_led = ~led_on;
`endif

endmodule
