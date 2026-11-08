// The top module must be named 'main'. Its ports are the board's
// pins in the pin file. The outputs start off; your design goes here.

module main (
    input        sys_clk, // 12MHz clock
    output [4:0] leds     // Off. Active high.
);

  // leds stays off.
  assign leds = 5'b00000;

  // Your design goes here.

endmodule
