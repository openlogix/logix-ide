// The top module must be named 'main'. Its ports are the board's
// pins in the pin file. The outputs start off; your design goes here.

module main (
    input        sys_clk, // 50MHz clock
    output [3:0] leds_n   // Off. Active low.
);

  // leds_n stays off.
  assign leds_n = 4'b1111;

  // Your design goes here.

endmodule
