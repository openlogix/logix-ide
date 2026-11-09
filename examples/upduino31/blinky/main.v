// Blinky for the UPduino v3.1: blinks the RGB LED in green about once a
// second.
//
// The board's 12 MHz oscillator reaches the FPGA only with a jumper
// shorted, so the clock comes from the FPGA's internal 48 MHz oscillator
// (SB_HFOSC). The RGB LED is on the FPGA's RGB LED driver pads, so it's
// driven through the driver (SB_RGBA_DRV).

`timescale 1ns / 1ps  // For the simulation-only clock below.

module main #(
  parameter N = 26  // Counter width: 2^26 cycles at 48 MHz is about 1.4 s.
) (
  output [2:0] rgb_led  // RGB LED: [0] red, [1] green, [2] blue, lit when 0.
);

  // The clock: the internal oscillator at 48 MHz.
  wire clk;
`ifdef SYNTHESIZE
  SB_HFOSC #(.CLKHF_DIV("0b00")) osc (  // 0b00: 48 MHz, undivided.
    .CLKHFPU(1'b1),  // Powered up.
    .CLKHFEN(1'b1),  // Output enabled.
    .CLKHF(clk)
  );
`else
  // Simulation only: a 48 MHz clock, as the oscillator gives.
  reg sim_clk = 1'b0;
  always #10.417 sim_clk = ~sim_clk;
  assign clk = sim_clk;
`endif

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge clk) counter <= counter + 1'b1;

  // Which colors are lit: green while the counter's top bit is 1.
  wire red_on = 1'b0;
  wire green_on = counter[N-1];
  wire blue_on = 1'b0;

  // The RGB LED driver. The board wires red to its RGB2 pad, green to RGB0
  // and blue to RGB1. A PWM input of 1 lights its color.
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
