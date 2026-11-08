// The top module must be named 'main'. Its ports are the board's
// pins in the pin file. The outputs start off; your design goes here.

module main (
    input        sys_clk,   // 48MHz clock
    output [2:0] rgb_led,   // RGB LED: [0] red, [1] green, [2] blue, off. Active low.
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

  // rgb_led stays off.
  rgb_driver rgb_driver (
      .on  (3'b000),
      .pins(rgb_led)
  );

  // Your design goes here.

endmodule
