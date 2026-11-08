//------------------------------------------------------------------
//-- Blinking RGB LED
//------------------------------------------------------------------

// N is the counter width. The red LED toggles every 2^(N-1) clocks.
// With the default N = 26 and a 48MHz clock, it toggles about
// every 0.7 seconds. The testbench uses a small N to run quickly.
module main #(
    parameter integer N = 26
) (
    input        sys_clk,   // 48MHz clock
    output [2:0] rgb_led,   // RGB LED: [0] red, [1] green, [2] blue; red blinks. Active low.
    output       usb_dp,    // Held low, see below.
    output       usb_dn,    // Held low, see below.
    output       usb_dp_pu  // Held low, see below.
);

  // Held low: usb_dp. Disconnects the board from the computer's USB,
  // which would otherwise try to talk to a design without a USB stack.
  assign usb_dp = 1'b0;

  // Held low: usb_dn. Disconnects the board from the computer's USB,
  // which would otherwise try to talk to a design without a USB stack.
  assign usb_dn = 1'b0;

  // Held low: usb_dp_pu. Disconnects the board from the computer's USB,
  // which would otherwise try to talk to a design without a USB stack.
  assign usb_dp_pu = 1'b0;

  // The blink counter. It starts at 0 when the FPGA is configured.
  reg [N-1:0] counter = 0;

  // Count up on every clock.
  always @(posedge sys_clk) counter <= counter + 1;

  // Red follows the counter's top bit, green and blue stay off.
  rgb_driver rgb_driver (
      .on  ({2'b00, counter[N-1]}),
      .pins(rgb_led)
  );

endmodule
