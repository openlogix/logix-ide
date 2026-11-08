// The top module must be named 'main'. Its ports are the board's
// pins in the pin file. The outputs start off; your design goes here.

module main (
    input        sys_clk, // 125MHz clock
    output [3:0] leds     // Off. Active high.
);

  // leds stays off.
  assign leds = 4'b0000;

  // Your design goes here.

endmodule
