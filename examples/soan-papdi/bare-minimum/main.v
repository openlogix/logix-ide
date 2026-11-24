// Bare minimum for the Soan-Papdi: a design that does nothing yet.
//
// The board's clock, LEDs, button and switches are already declared and
// wired in pinout.pcf, so you can start adding your own logic right away.
// All LEDs are held off until your logic uses them. Build and upload it as
// it is: nothing lights up, which shows the board takes a design.

module main (
  input sys_clk,                // 12 MHz clock
  output [7:0] leds,            // LEDs 0 to 7, lit when 1
  output [3:0] status_leds,     // Status LEDs S0 to S3, lit when 0
  input prog_button,            // PROG button, 1 when pressed (assumed)
  input [3:0] switches_a,       // First 4 switches, 1 when on
  input [3:0] switches_b        // Other 4 switches, 1 when on
);

  // Hold the LEDs off: they light at 1.
  assign leds = 8'b00000000;

  // Status LEDs S0 to S2 are on the FPGA's RGB LED driver pads, which can only
  // be driven through SB_RGBA_DRV, the iCE40 UltraPlus's LED driver cell. It
  // pulls a pad low (lighting its LED) while its PWM input is 1, and leaves it
  // off while it's 0. All three inputs are 0 here, so S0 to S2 stay off.
  // S3 is a plain pin: it lights at 0, so 1 holds it off.
  wire [2:0] status_pwm = 3'b000;  // 1 lights S0, S1, S2 (bit 0 is S0)

`ifdef SYNTHESIZE
  SB_RGBA_DRV #(
    .CURRENT_MODE("0b1"),          // Half current mode
    .RGB0_CURRENT("0b000001"),     // The lowest current, plenty for a status LED
    .RGB1_CURRENT("0b000001"),
    .RGB2_CURRENT("0b000001")
  ) status_driver (
    .CURREN(1'b1),                 // Turn the driver's current source on
    .RGBLEDEN(1'b1),               // Turn the three LED outputs on
    .RGB0PWM(status_pwm[0]),
    .RGB1PWM(status_pwm[1]),
    .RGB2PWM(status_pwm[2]),
    .RGB0(status_leds[0]),
    .RGB1(status_leds[1]),
    .RGB2(status_leds[2])
  );
`else
  // Simulation only: the simulator has no model of SB_RGBA_DRV, so these
  // lines give the pads the levels the driver gives them: 0 (lit) while a
  // PWM input is 1, else 1 (off).
  assign status_leds[0] = !status_pwm[0];
  assign status_leds[1] = !status_pwm[1];
  assign status_leds[2] = !status_pwm[2];
`endif

  assign status_leds[3] = 1'b1;

  // Your logic goes here.

endmodule
