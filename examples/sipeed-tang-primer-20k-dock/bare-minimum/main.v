// The top module must be named 'main'. Its ports are the board's
// pins in the pin file. The outputs start off; your design goes here.

module main (
    input        sys_clk, // 27MHz clock
    output [5:0] leds     // Off. Active level unknown, taken as high.
);

  // leds stays off.
  assign leds = 6'b000000;

  // Your design goes here.

endmodule
